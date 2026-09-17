//! Explicit-request-only transport and credential storage. Never log payloads,
//! keys, backend errors or provider bodies. No Debug on secret-bearing state.
use flightlens_core::{
    feedback::{self, Redaction},
    llm::*,
    Command, ConfigDocument,
};
use std::{
    collections::BTreeMap,
    future::Future,
    io::Read,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

const STORE_UNAVAILABLE: &str = "The OS credential store is unavailable or locked. FlightLens cannot save or read the key. You can explicitly use a session-only key, discarded on exit.";
const STALE: &str = "The request or settings changed. Review a new preview before sending.";
const BUSY: &str = "An AI request is already running.";
const RESPONSE_LIMIT: usize = 1024 * 1024;

trait Credentials: Send + Sync {
    fn get(&self, provider: Provider) -> Result<Option<String>, String>;
    fn set(&self, provider: Provider, key: &str) -> Result<(), String>;
    fn delete(&self, provider: Provider) -> Result<(), String>;
}
struct OsCredentials;
fn account(provider: Provider) -> &'static str {
    match provider {
        Provider::Gemini => "gemini",
        Provider::Openai => "openai",
        Provider::Openrouter => "openrouter",
        Provider::Custom => "custom",
    }
}
fn entry(provider: Provider) -> Result<keyring::Entry, String> {
    keyring::Entry::new("FlightLens.AI", account(provider)).map_err(|_| STORE_UNAVAILABLE.into())
}
impl Credentials for OsCredentials {
    fn get(&self, provider: Provider) -> Result<Option<String>, String> {
        match entry(provider)?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(STORE_UNAVAILABLE.into()),
        }
    }
    fn set(&self, provider: Provider, key: &str) -> Result<(), String> {
        entry(provider)?
            .set_password(key)
            .map_err(|_| STORE_UNAVAILABLE.into())
    }
    fn delete(&self, provider: Provider) -> Result<(), String> {
        match entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(STORE_UNAVAILABLE.into()),
        }
    }
}
// Clear our owned key buffers promptly. No promise of erasing allocator/IPC copies.
fn clear_key(key: String) {
    let mut bytes = key.into_bytes();
    bytes.fill(0);
    std::hint::black_box(&mut bytes);
}
struct Secret(String);
impl Drop for Secret {
    fn drop(&mut self) {
        clear_key(std::mem::take(&mut self.0));
    }
}

#[derive(Default)]
struct Inner {
    session_keys: BTreeMap<&'static str, Secret>,
    preview: Option<String>,
    cache: BTreeMap<String, LlmSummary>,
    generation: u64,
}
impl Inner {
    fn invalidate(&mut self) {
        self.preview = None;
        self.cache.clear();
        self.generation = self.generation.wrapping_add(1);
    }
}
#[derive(Default)]
struct Control {
    busy: AtomicBool,
    cancelled: AtomicBool,
    abort: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}
struct Permit(Arc<Control>);
impl Control {
    fn acquire(self: &Arc<Self>) -> Result<Permit, String> {
        let _lock = self.abort.lock().map_err(|_| BUSY)?;
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| BUSY)?;
        self.cancelled.store(false, Ordering::SeqCst);
        Ok(Permit(self.clone()))
    }
    fn cancel(&self) {
        if let Ok(abort) = self.abort.lock() {
            self.cancelled.store(true, Ordering::SeqCst);
            if let Some(abort) = abort.as_ref() {
                abort();
            }
        }
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        if let Ok(mut abort) = self.0.abort.lock() {
            if let Some(abort) = abort.take() {
                abort();
            }
            self.0.busy.store(false, Ordering::SeqCst);
        }
    }
}
pub struct Service {
    inner: Mutex<Inner>,
    control: Arc<Control>,
    credentials: Box<dyn Credentials>,
}
impl Default for Service {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            control: Arc::default(),
            credentials: Box::new(OsCredentials),
        }
    }
}
impl Service {
    fn save_key(
        &self,
        provider: Provider,
        key: Secret,
        session_only: bool,
        inner: &mut Inner,
    ) -> Result<(), String> {
        if key.0.is_empty() || key.0.len() > 4096 || !key.0.bytes().all(|c| (33..=126).contains(&c))
        {
            return Err("Enter a valid API key without spaces or control characters.".into());
        }
        if session_only {
            inner.session_keys.insert(account(provider), key);
        } else {
            self.credentials.set(provider, &key.0)?;
            inner.session_keys.remove(account(provider));
        }
        inner.invalidate();
        Ok(())
    }
    fn clear_key(&self, provider: Provider, inner: &mut Inner) -> Result<(), String> {
        inner.session_keys.remove(account(provider));
        inner.invalidate();
        self.credentials.delete(provider)
    }
    pub fn invalidate(&self) {
        self.control.cancel();
        if let Ok(mut inner) = self.inner.lock() {
            inner.invalidate();
        }
    }
    fn status(&self, settings: LlmSettings, inner: &Inner) -> LlmStatus {
        let session = inner.session_keys.get(account(settings.provider));
        let result = match session {
            Some(key) => Ok(Some(key.0.clone())),
            None => self.credentials.get(settings.provider),
        };
        let (has_key, key_hint, problem) = match result {
            Ok(Some(key)) => {
                let hint = (key.chars().count() > 4).then(|| {
                    key.chars()
                        .rev()
                        .take(4)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect()
                });
                let has = !key.is_empty();
                clear_key(key);
                (has, hint, None)
            }
            Ok(None) => (false, None, None),
            Err(error) => (false, None, Some(error)),
        };
        LlmStatus {
            settings,
            has_key,
            key_hint,
            session_only: session.is_some(),
            credential_problem: problem,
        }
    }
    fn key(&self, settings: &LlmSettings, inner: &Inner) -> Result<Option<Secret>, String> {
        if let Some(key) = inner.session_keys.get(account(settings.provider)) {
            return Ok(Some(Secret(key.0.clone())));
        }
        match self.credentials.get(settings.provider) {
            Ok(key) => Ok(key.map(Secret)),
            // No credential store is needed for an explicitly keyless local service.
            Err(_)
                if settings.provider == Provider::Custom
                    && settings.base_url.as_deref().is_some_and(|url| {
                        reqwest::Url::parse(url)
                            .ok()
                            .is_some_and(|u| u.host_str().is_some_and(is_loopback))
                    }) =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
}
fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory unavailable")?
        .join("llm.json"))
}
fn read_settings(path: &Path) -> Result<LlmSettings, String> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(LlmSettings::default()),
        Err(_) => return Err("Cannot read AI settings.".into()),
    };
    let mut bytes = Vec::new();
    file.take(16 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read AI settings.")?;
    if bytes.len() > 16 * 1024 {
        return Err("AI settings file is too large.".into());
    }
    serde_json::from_slice::<LlmSettings>(&bytes)
        .map_err(|_| "AI settings file is invalid.".to_string())?
        .validated()
        .map_err(|e| e.to_string())
}
fn write_settings(path: &Path, settings: &LlmSettings) -> Result<LlmSettings, String> {
    let settings = settings.validated().map_err(|e| e.to_string())?;
    let data = serde_json::to_vec_pretty(&settings).map_err(|_| "Cannot encode AI settings.")?;
    std::fs::create_dir_all(path.parent().ok_or("AI settings directory unavailable")?)
        .and_then(|_| std::fs::write(path, data))
        .map_err(|_| "Cannot save AI settings.")?;
    Ok(settings)
}

struct Prepared {
    prompt: Prompt,
    json: String,
    excluded: Vec<Redaction>,
    labels: Vec<(String, String)>,
    truncated: Option<Truncation>,
    identity: String,
}
fn prepare(request: &SummaryRequest, documents: &[&ConfigDocument]) -> Result<Prepared, String> {
    let (prompt, json, excluded, labels, truncated, hash) = match request {
        SummaryRequest::Inspector {
            rate_profile,
            pid_profile,
            ..
        } => {
            let d = documents.first().ok_or("The backup is no longer open.")?;
            let pack = flightlens_core::compatibility::pack(d.firmware.version.as_deref());
            if *rate_profile >= pack.map_or(6, |p| p.profiles.rate)
                || *pid_profile >= pack.map_or(6, |p| p.profiles.pid)
            {
                return Err("The selected profile is out of range.".into());
            }
            let digest = InspectorDigest::build(d, *rate_profile, *pid_profile);
            let mut excluded = Vec::new();
            for line in &d.syntax {
                if let Command::Set { key, .. } = &line.command {
                    if let Some(category) = feedback::sensitivity(key) {
                        excluded.push(Redaction {
                            key: key.clone(),
                            line: line.line,
                            category,
                        });
                    }
                }
            }
            if d.craft_name.is_some() {
                excluded.push(Redaction {
                    key: "name".into(),
                    line: 0,
                    category: feedback::Category::Identity,
                });
            }
            (
                inspector_prompt(&digest),
                serde_json::to_string_pretty(&digest),
                excluded,
                vec![(d.title.clone(), "Backup A".into())],
                None,
                d.hash.clone(),
            )
        }
        SummaryRequest::Diff {
            slots,
            baseline,
            rows,
        } => {
            if !(2..=3).contains(&slots.len()) || documents.len() != slots.len() {
                return Err("Select two or three open backups.".into());
            }
            if slots.len() == 3 && baseline.is_none() {
                return Err("Select a baseline for the three-backup comparison.".into());
            }
            for (slot, document) in slots.iter().zip(documents) {
                if slot.config_id != document.id {
                    return Err("The comparison backup changed. Select it again.".into());
                }
                let pack =
                    flightlens_core::compatibility::pack(document.firmware.version.as_deref());
                if slot.rate_profile >= pack.map_or(6, |p| p.profiles.rate)
                    || slot.pid_profile >= pack.map_or(6, |p| p.profiles.pid)
                {
                    return Err("The selected profile is out of range.".into());
                }
            }
            let input_bytes: usize = rows
                .iter()
                .map(|r| {
                    r.key.len()
                        + r.scope.len()
                        + r.section.len()
                        + r.status.len()
                        + r.reason.as_ref().map_or(0, String::len)
                        + r.values.iter().flatten().map(String::len).sum::<usize>()
                })
                .sum();
            if input_bytes > 2 * 1024 * 1024 {
                return Err("The comparison input exceeds the preview limit.".into());
            }
            if rows.len() > 20_000
                || rows.iter().any(|r| {
                    r.key.len() > 16384
                        || r.scope.len() > 16384
                        || r.reason.as_ref().is_some_and(|s| s.len() > 16384)
                        || r.values.iter().flatten().any(|s| s.len() > 16384)
                })
            {
                return Err("The comparison input exceeds the preview limit.".into());
            }
            let mut excluded = Vec::new();
            let filtered: Vec<_> = rows
                .iter()
                .filter_map(|row| {
                    if let Some(category) = feedback::sensitivity(&row.key) {
                        excluded.push(Redaction {
                            key: row.key.clone(),
                            line: 0,
                            category,
                        });
                        None
                    } else {
                        Some(row.clone())
                    }
                })
                .collect();
            let digest =
                DiffDigest::build(slots.len(), *baseline, &filtered).map_err(|e| e.to_string())?;
            (
                diff_prompt(&digest),
                serde_json::to_string_pretty(&digest),
                excluded,
                documents
                    .iter()
                    .map(|d| d.title.clone())
                    .zip(digest.labels.iter().cloned())
                    .collect(),
                digest.truncated.clone(),
                serde_json::to_string(&documents.iter().map(|d| &d.hash).collect::<Vec<_>>())
                    .map_err(|_| "Cannot prepare comparison identity.")?,
            )
        }
    };
    // Exact equality avoids hash collisions and extra dependencies. This identity
    // remains local and bounds the cache to the original request as well as digest.
    let identity = serde_json::to_string(&(request, hash, PROMPT_VERSION))
        .map_err(|_| "Cannot prepare AI request.")?;
    Ok(Prepared {
        prompt,
        json: json.map_err(|_| "Cannot encode AI digest.")?,
        excluded,
        labels,
        truncated,
        identity,
    })
}
fn identity(prepared: &Prepared, settings: &LlmSettings) -> String {
    serde_json::to_string(&(
        settings,
        &prepared.identity,
        &prepared.prompt.system,
        &prepared.prompt.user,
    ))
    .expect("serializable request")
}

struct Response {
    status: u16,
    body: String,
}
type Transfer = Pin<Box<dyn Future<Output = Result<Response, String>> + Send>>;
trait Transport: Send + Sync {
    fn send(&self, request: HttpRequest, timeout: Duration) -> Transfer;
}
struct HttpTransport;
fn is_loopback(host: &str) -> bool {
    host == "localhost"
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

impl Transport for HttpTransport {
    fn send(&self, request: HttpRequest, timeout: Duration) -> Transfer {
        Box::pin(async move {
            let url = reqwest::Url::parse(&request.url).map_err(|_| "Invalid AI endpoint.")?;
            let host = url.host_str().unwrap_or("configured endpoint").to_string();
            let local = host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback());
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .referer(false)
                .no_proxy()
                .timeout(timeout)
                .build()
                .map_err(|_| "Cannot initialize AI transport.")?;
            let start = Instant::now();
            let error = |e: reqwest::Error| {
                if e.is_timeout() {
                    format!(
                        "AI request timed out after {} seconds.",
                        start.elapsed().as_secs()
                    )
                } else if local {
                    "FlightLens could not reach the local model. Check that it is running.".into()
                } else {
                    format!("FlightLens could not reach {host}.")
                }
            };
            let mut builder = client.post(url);
            for (name, value) in request.headers {
                let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| "Invalid AI request header.")?;
                let mut header = reqwest::header::HeaderValue::from_str(&value)
                    .map_err(|_| "Invalid AI request header.")?;
                header.set_sensitive(true);
                clear_key(value);
                builder = builder.header(name, header);
            }
            let mut response = builder.body(request.body).send().await.map_err(&error)?;
            let status = response.status().as_u16();
            // Error status mapping needs no provider body, which may contain secrets.
            if !(200..300).contains(&status) {
                return Ok(Response {
                    status,
                    body: String::new(),
                });
            }
            if response
                .content_length()
                .is_some_and(|n| n > RESPONSE_LIMIT as u64)
            {
                return Err("AI response exceeded the size limit.".into());
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(&error)? {
                if bytes.len() + chunk.len() > RESPONSE_LIMIT {
                    return Err("AI response exceeded the size limit.".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(Response {
                status,
                body: String::from_utf8(bytes).map_err(|_| "AI response was not valid UTF-8.")?,
            })
        })
    }
}

async fn transfer(
    control: Arc<Control>,
    transport: &dyn Transport,
    request: HttpRequest,
    settings: &LlmSettings,
) -> Result<Summary, String> {
    let future = transport.send(
        request,
        Duration::from_secs(settings.timeout_seconds.into()),
    );
    let task = {
        let mut abort = control.abort.lock().map_err(|_| "AI request unavailable")?;
        if control.cancelled.load(Ordering::SeqCst) {
            return Err("AI request cancelled.".into());
        }
        let task = tauri::async_runtime::spawn(future);
        let handle = task.inner().abort_handle();
        *abort = Some(Box::new(move || handle.abort()));
        task
    };
    let response = task
        .await
        .map_err(|_| "AI request cancelled or interrupted.")??;
    if control.cancelled.load(Ordering::SeqCst) {
        return Err("AI request cancelled.".into());
    }
    parse_response(settings.provider, response.status, &response.body).map_err(|e| e.to_string())
}

fn request_documents(
    app: &tauri::AppHandle,
    request: &SummaryRequest,
) -> Result<Vec<Arc<ConfigDocument>>, String> {
    match request {
        SummaryRequest::Inspector { config_id, .. } => {
            super::document(&app.state::<super::AppState>(), config_id).map(|d| vec![d])
        }
        SummaryRequest::Diff { slots, .. } => {
            if !(2..=3).contains(&slots.len()) {
                return Err("Select two or three open backups.".into());
            }
            slots
                .iter()
                .map(|slot| super::document(&app.state::<super::AppState>(), &slot.config_id))
                .collect()
        }
    }
}
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| "AI settings worker failed.")?
}

#[tauri::command]
pub async fn llm_settings(app: tauri::AppHandle) -> Result<LlmStatus, String> {
    blocking(move || {
        let settings = read_settings(&settings_path(&app)?)?;
        let state = app.state::<super::AppState>();
        let inner = state
            .llm
            .inner
            .lock()
            .map_err(|_| "AI settings unavailable")?;
        Ok(state.llm.status(settings, &inner))
    })
    .await
}
#[tauri::command]
pub async fn llm_save_settings(
    app: tauri::AppHandle,
    settings: LlmSettings,
) -> Result<LlmStatus, String> {
    blocking(move || {
        let state = app.state::<super::AppState>();
        let _permit = state.llm.control.acquire()?;
        let mut inner = state
            .llm
            .inner
            .lock()
            .map_err(|_| "AI settings unavailable")?;
        let settings = write_settings(&settings_path(&app)?, &settings)?;
        inner.invalidate();
        Ok(state.llm.status(settings, &inner))
    })
    .await
}
#[tauri::command]
pub async fn llm_save_key(
    app: tauri::AppHandle,
    provider: Provider,
    key: String,
    session_only: bool,
) -> Result<LlmStatus, String> {
    let key = Secret(key);
    blocking(move || {
        let state = app.state::<super::AppState>();
        let _permit = state.llm.control.acquire()?;
        let mut inner = state
            .llm
            .inner
            .lock()
            .map_err(|_| "AI settings unavailable")?;
        let settings = read_settings(&settings_path(&app)?)?;
        state
            .llm
            .save_key(provider, key, session_only, &mut inner)?;
        Ok(state.llm.status(settings, &inner))
    })
    .await
}
#[tauri::command]
pub async fn llm_clear_key(app: tauri::AppHandle, provider: Provider) -> Result<LlmStatus, String> {
    blocking(move || {
        let state = app.state::<super::AppState>();
        let _permit = state.llm.control.acquire()?;
        let mut inner = state
            .llm
            .inner
            .lock()
            .map_err(|_| "AI settings unavailable")?;
        let settings = read_settings(&settings_path(&app)?)?;
        state.llm.clear_key(provider, &mut inner)?;
        Ok(state.llm.status(settings, &inner))
    })
    .await
}
// Both preview and send use the same preparation path; only this local receipt
// authorizes a matching rebuilt request. Receipts are single-use.
impl Service {
    fn preview(
        &self,
        settings: &LlmSettings,
        prepared: Prepared,
        connection_test: bool,
    ) -> Result<LlmPreview, String> {
        let mut inner = self.inner.lock().map_err(|_| "AI settings unavailable")?;
        inner.preview = None;
        let mut problems = Vec::new();
        if !settings.enabled && !connection_test {
            problems.push("AI summaries are off. Enable them in settings.".into());
        }
        if self.control.busy.load(Ordering::SeqCst) {
            problems.push(BUSY.into());
        }
        let key = match self.key(settings, &inner) {
            Ok(key) => key,
            Err(e) => {
                problems.push(e);
                None
            }
        };
        let wire = build_request(
            settings,
            key.as_ref().map(|k| k.0.as_str()),
            &prepared.prompt,
        );
        // Auth is not part of the body. The dummy key is never sent.
        let preview_wire = build_request(settings, Some("preview-only"), &prepared.prompt)
            .map_err(|e| e.to_string())?;
        if let Err(error) = wire {
            problems.push(error.to_string());
        }
        if problems.is_empty() {
            inner.preview = Some(identity(&prepared, settings));
        }
        Ok(LlmPreview {
            system_prompt: prepared.prompt.system,
            user_prompt: prepared.prompt.user,
            payload_json: prepared.json,
            excluded: prepared.excluded,
            label_map: prepared.labels,
            bytes: preview_wire.body.len(),
            truncated: prepared.truncated,
            problems,
            destination: preview_wire.url,
        })
    }
    fn authorize(
        &self,
        settings: LlmSettings,
        prepared: Prepared,
        regenerate: bool,
        connection_test: bool,
    ) -> Result<Pending, String> {
        if !settings.enabled && !connection_test {
            return Err("AI summaries are off.".into());
        }
        let mut inner = self.inner.lock().map_err(|_| "AI settings unavailable")?;
        let cache_key = identity(&prepared, &settings);
        if inner.preview.as_ref() != Some(&cache_key) {
            return Err(STALE.into());
        }
        inner.preview = None;
        if self.control.cancelled.load(Ordering::SeqCst) {
            return Err("AI request cancelled.".into());
        }
        let key = self.key(&settings, &inner)?;
        let wire = build_request(
            &settings,
            key.as_ref().map(|k| k.0.as_str()),
            &prepared.prompt,
        )
        .map_err(|e| e.to_string())?;
        let cached = if regenerate || connection_test {
            None
        } else {
            inner.cache.get(&cache_key).cloned()
        };
        Ok(Pending {
            wire,
            settings,
            cache_key,
            generation: inner.generation,
            cached,
        })
    }
    async fn execute(
        &self,
        pending: Pending,
        transport: &dyn Transport,
    ) -> Result<LlmSummary, String> {
        let Pending {
            wire,
            settings,
            cache_key,
            generation,
            cached,
        } = pending;
        if let Some(mut cached) = cached {
            let inner = self.inner.lock().map_err(|_| "AI cache unavailable")?;
            if inner.generation != generation || self.control.cancelled.load(Ordering::SeqCst) {
                return Err(STALE.into());
            }
            cached.cached = true;
            return Ok(cached);
        }
        let summary = transfer(self.control.clone(), transport, wire, &settings).await?;
        let result = LlmSummary {
            text: summary.text,
            provider: settings.provider,
            model: settings.model,
            prompt_version: PROMPT_VERSION,
            generated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| "System clock unavailable")?
                .as_secs()
                .to_string(),
            cached: false,
        };
        let mut inner = self.inner.lock().map_err(|_| "AI cache unavailable")?;
        if inner.generation != generation || self.control.cancelled.load(Ordering::SeqCst) {
            return Err(STALE.into());
        }
        let size = cache_key.len() + result.text.len();
        if inner.cache.len() >= 16
            || inner
                .cache
                .iter()
                .map(|(k, v)| k.len() + v.text.len())
                .sum::<usize>()
                + size
                > 8 * 1024 * 1024
        {
            inner.cache.clear();
        }
        if size <= 8 * 1024 * 1024 {
            inner.cache.insert(cache_key, result.clone());
        }
        Ok(result)
    }
}
struct Pending {
    wire: HttpRequest,
    settings: LlmSettings,
    cache_key: String,
    generation: u64,
    cached: Option<LlmSummary>,
}
fn connection_prompt() -> Prepared {
    Prepared {
        prompt: Prompt {
            system: "Reply with OK only.".into(),
            user: "Connection test. No backup data is included.".into(),
        },
        json: "{}".into(),
        excluded: vec![],
        labels: vec![],
        truncated: None,
        identity: "connection-test-v1".into(),
    }
}
#[tauri::command]
pub async fn llm_preview(
    app: tauri::AppHandle,
    request: SummaryRequest,
) -> Result<LlmPreview, String> {
    blocking(move || {
        let documents = request_documents(&app, &request)?;
        let prepared = prepare(
            &request,
            &documents.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        )?;
        let settings = read_settings(&settings_path(&app)?)?;
        app.state::<super::AppState>()
            .llm
            .preview(&settings, prepared, false)
    })
    .await
}
#[tauri::command]
pub async fn llm_summarize(
    app: tauri::AppHandle,
    request: SummaryRequest,
    regenerate: bool,
) -> Result<LlmSummary, String> {
    let control = app.state::<super::AppState>().llm.control.clone();
    let _permit = control.acquire()?;
    let worker_app = app.clone();
    let pending = blocking(move || {
        let documents = request_documents(&worker_app, &request)?;
        let prepared = prepare(
            &request,
            &documents.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        )?;
        let settings = read_settings(&settings_path(&worker_app)?)?;
        worker_app
            .state::<super::AppState>()
            .llm
            .authorize(settings, prepared, regenerate, false)
    })
    .await?;
    app.state::<super::AppState>()
        .llm
        .execute(pending, &HttpTransport)
        .await
}
#[tauri::command]
pub fn llm_cancel(app: tauri::AppHandle) {
    let state = app.state::<super::AppState>();
    state.llm.control.cancel();
    if let Ok(mut inner) = state.llm.inner.lock() {
        inner.preview = None;
    };
}
#[tauri::command]
pub async fn llm_test_preview(app: tauri::AppHandle) -> Result<LlmPreview, String> {
    blocking(move || {
        let settings = read_settings(&settings_path(&app)?)?;
        app.state::<super::AppState>()
            .llm
            .preview(&settings, connection_prompt(), true)
    })
    .await
}
#[tauri::command]
pub async fn llm_test_connection(app: tauri::AppHandle) -> Result<String, String> {
    let control = app.state::<super::AppState>().llm.control.clone();
    let _permit = control.acquire()?;
    let pending = blocking(move || {
        let settings = read_settings(&settings_path(&app)?)?;
        app.state::<super::AppState>()
            .llm
            .authorize(settings, connection_prompt(), true, true)
    })
    .await?;
    transfer(control, &HttpTransport, pending.wire, &pending.settings).await?;
    Ok("Connection succeeded. No backup data was sent.".into())
}

#[cfg(test)]
mod tests;

// Metadata is local-only. Escape Markdown so filenames cannot create links or HTML.
fn markdown_label(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| {
            if c.is_control() {
                vec![' ']
            } else if c.is_ascii_punctuation() {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect()
}
fn utc_timestamp(seconds: &str) -> String {
    let Ok(seconds) = seconds.parse::<u64>() else {
        return "Unknown".into();
    };
    let mut days = seconds / 86_400;
    let mut year = 1970u32;
    let leap = |year: u32| {
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
    };
    loop {
        let length = if leap(year) { 366 } else { 365 };
        if days < length {
            break;
        }
        days -= length;
        year += 1;
        if year > 9999 {
            return "Unknown".into();
        }
    }
    let lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while days >= lengths[month] {
        days -= lengths[month];
        month += 1;
    }
    format!(
        "{year:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        month + 1,
        days + 1,
        seconds % 86_400 / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}
fn summary_markdown(
    summary: &LlmSummary,
    request: &SummaryRequest,
    documents: &[&ConfigDocument],
) -> String {
    let (mode, provenance) = match request {
        SummaryRequest::Inspector {
            rate_profile,
            pid_profile,
            ..
        } => {
            let document = documents[0];
            (
                "Inspector",
                format!(
                    "- Backup A: {} (SHA-256 {})\n- Profiles: rate {} (CLI {}), PID {} (CLI {})",
                    markdown_label(&document.title),
                    markdown_label(&document.hash),
                    u16::from(*rate_profile) + 1,
                    rate_profile,
                    u16::from(*pid_profile) + 1,
                    pid_profile
                ),
            )
        }
        SummaryRequest::Diff {
            slots, baseline, ..
        } => {
            let mut lines = slots
                .iter()
                .zip(documents)
                .enumerate()
                .map(|(i, (slot, document))| {
                    format!(
                        "- Backup {}: {} (SHA-256 {}) · rate {} (CLI {}), PID {} (CLI {})",
                        ["A", "B", "C"][i],
                        markdown_label(&document.title),
                        markdown_label(&document.hash),
                        u16::from(slot.rate_profile) + 1,
                        slot.rate_profile,
                        u16::from(slot.pid_profile) + 1,
                        slot.pid_profile
                    )
                })
                .collect::<Vec<_>>();
            lines.push(match baseline {
                Some(index) => format!("- Baseline: Backup {}", ["A", "B", "C"][*index]),
                None => "- Baseline: None (two-backup comparison)".into(),
            });
            ("Compare", lines.join("\n"))
        }
    };
    // Indented code preserves untrusted model output as text, including fences,
    // HTML and image links, when the saved Markdown is opened in a renderer.
    let text = summary
        .text
        .lines()
        .map(|line| format!("    {line}\n"))
        .collect::<String>();
    format!("# FlightLens AI summary\n\n- Generated: {}\n- Mode: {}\n{}\n- Provider: {}\n- Model: {}\n- Prompt version: {}\n- FlightLens: {}\n\n> This text was generated by an external language model from a redacted summary of the backup. It is not produced or verified by FlightLens, and it is not a certified finding. Check every claim against the Inspector, Compare and Raw views before acting on it.\n\n---\n\n{}",
        utc_timestamp(&summary.generated_at), mode, provenance, account(summary.provider), markdown_label(&summary.model), summary.prompt_version, env!("CARGO_PKG_VERSION"), text)
}
fn cached_export(
    service: &Service,
    settings: &LlmSettings,
    request: &SummaryRequest,
    documents: &[&ConfigDocument],
    expected: &LlmSummary,
) -> Result<String, String> {
    let prepared = prepare(request, documents)?;
    let inner = service.inner.lock().map_err(|_| "AI cache unavailable")?;
    let summary = inner.cache.get(&identity(&prepared, settings)).ok_or(
        "This summary is no longer cached. Review a new preview before generating it again.",
    )?;
    if summary.text != expected.text
        || summary.generated_at != expected.generated_at
        || summary.provider != expected.provider
        || summary.model != expected.model
        || summary.prompt_version != expected.prompt_version
    {
        return Err("The cached summary changed. Review it again before saving.".into());
    }
    Ok(summary_markdown(summary, request, documents))
}
#[tauri::command]
pub async fn llm_save_summary(
    app: tauri::AppHandle,
    request: SummaryRequest,
    summary: LlmSummary,
) -> Result<bool, String> {
    blocking(move || {
        let documents = request_documents(&app, &request)?;
        let settings = read_settings(&settings_path(&app)?)?;
        let text = cached_export(
            &app.state::<super::AppState>().llm,
            &settings,
            &request,
            &documents.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            &summary,
        )?;
        super::save::save_new(
            &app,
            "flightlens-ai-summary.md",
            ("Markdown", &["md"]),
            |_| Ok(text.as_bytes().to_vec()),
        )
    })
    .await
}
