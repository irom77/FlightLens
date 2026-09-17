use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct FakeCredentials {
    unavailable: bool,
    keys: Mutex<BTreeMap<&'static str, String>>,
}
impl Credentials for FakeCredentials {
    fn get(&self, p: Provider) -> Result<Option<String>, String> {
        if self.unavailable {
            Err(STORE_UNAVAILABLE.into())
        } else {
            Ok(self.keys.lock().unwrap().get(account(p)).cloned())
        }
    }
    fn set(&self, p: Provider, key: &str) -> Result<(), String> {
        if self.unavailable {
            Err(STORE_UNAVAILABLE.into())
        } else {
            self.keys.lock().unwrap().insert(account(p), key.into());
            Ok(())
        }
    }
    fn delete(&self, p: Provider) -> Result<(), String> {
        if self.unavailable {
            Err(STORE_UNAVAILABLE.into())
        } else {
            self.keys.lock().unwrap().remove(account(p));
            Ok(())
        }
    }
}
fn service(unavailable: bool) -> Service {
    Service {
        credentials: Box::new(FakeCredentials {
            unavailable,
            ..Default::default()
        }),
        ..Default::default()
    }
}
fn settings() -> LlmSettings {
    LlmSettings {
        enabled: true,
        provider: Provider::Custom,
        model: "test-model".into(),
        base_url: Some("http://127.0.0.1:11434".into()),
        timeout_seconds: 5,
    }
}
fn documents() -> Vec<ConfigDocument> {
    ["A", "B"]
        .iter()
        .map(|label| {
            let flightlens_core::Artifact::Config(mut d) = flightlens_core::analyze(
                "# Betaflight / STM32F405 4.5.0\nset motor_poles = 14\n",
                &format!("PRIVATE-{label}"),
                "PRIVATE-PATH",
            )
            .unwrap() else {
                panic!("config")
            };
            d.id = label.to_string();
            *d
        })
        .collect()
}
fn prepare_diff(request: &SummaryRequest) -> Result<Prepared, String> {
    prepare(request, &documents().iter().collect::<Vec<_>>())
}
fn request() -> SummaryRequest {
    SummaryRequest::Diff {
        slots: documents()
            .iter()
            .map(|d| SummarySlot {
                config_id: d.id.clone(),
                rate_profile: 0,
                pid_profile: 0,
            })
            .collect(),
        baseline: Some(0),
        rows: vec![DiffRowInput {
            section: "parameters".into(),
            key: "motor_poles".into(),
            scope: "global".into(),
            status: "unknown".into(),
            values: vec![Some("14".into()), None],
            reason: None,
        }],
    }
}
struct FakeTransport {
    calls: AtomicUsize,
    bodies: Mutex<Vec<String>>,
}
impl FakeTransport {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            bodies: Mutex::new(vec![]),
        }
    }
}
impl Transport for FakeTransport {
    fn send(&self, request: HttpRequest, _: Duration) -> Transfer {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.bodies.lock().unwrap().push(request.body);
        Box::pin(async {
            Ok(Response { status: 200, body: r#"{"choices":[{"finish_reason":"stop","message":{"content":"Synthetic summary"}}]}"#.into() })
        })
    }
}
fn preview(service: &Service, settings: &LlmSettings) -> LlmPreview {
    service
        .preview(settings, prepare_diff(&request()).unwrap(), false)
        .unwrap()
}
fn pending(service: &Service, settings: LlmSettings, regenerate: bool) -> Pending {
    service
        .authorize(
            settings,
            prepare_diff(&request()).unwrap(),
            regenerate,
            false,
        )
        .unwrap()
}

#[test]
fn settings_roundtrip_rejects_invalid_stored_url_and_secret_fields() {
    let dir = std::env::temp_dir().join(format!("flightlens-llm-settings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("llm.json");
    assert!(!read_settings(&dir.join("absent.json")).unwrap().enabled);
    assert_eq!(
        write_settings(&path, &settings()).unwrap(),
        read_settings(&path).unwrap()
    );
    let mut data = serde_json::to_value(settings()).unwrap();
    data["baseUrl"] = "http://example.com".into();
    std::fs::write(&path, data.to_string()).unwrap();
    assert!(read_settings(&path).is_err());
    data["baseUrl"] = "http://localhost:11434".into();
    data["key"] = "PRIVATE".into();
    std::fs::write(&path, data.to_string()).unwrap();
    assert!(read_settings(&path).is_err());
    assert!(write_settings(&dir, &settings()).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn credentials_are_write_only_and_fallback_is_explicit() {
    let service = service(false);
    let mut inner = service.inner.lock().unwrap();
    service
        .save_key(
            Provider::Gemini,
            Secret("PRIVATE-key-1234".into()),
            false,
            &mut inner,
        )
        .unwrap();
    let status = service.status(LlmSettings::default(), &inner);
    let json = serde_json::to_string(&status).unwrap();
    assert!(!json.contains("PRIVATE"));
    assert!(status.has_key);
    assert_eq!(status.key_hint.as_deref(), Some("1234"));
    assert!(!status.session_only);
    service.clear_key(Provider::Gemini, &mut inner).unwrap();
    assert!(!service.status(LlmSettings::default(), &inner).has_key);
    service
        .save_key(Provider::Gemini, Secret("abcd".into()), true, &mut inner)
        .unwrap();
    assert!(service
        .status(LlmSettings::default(), &inner)
        .key_hint
        .is_none());
    drop(inner);
    let unavailable = super::tests::service(true);
    let mut inner = unavailable.inner.lock().unwrap();
    assert!(unavailable
        .save_key(
            Provider::Gemini,
            Secret("PRIVATE".into()),
            false,
            &mut inner
        )
        .is_err());
    assert!(inner.session_keys.is_empty());
    assert!(unavailable
        .status(LlmSettings::default(), &inner)
        .credential_problem
        .is_some());
    unavailable
        .save_key(Provider::Gemini, Secret("PRIVATE".into()), true, &mut inner)
        .unwrap();
    assert!(
        unavailable
            .status(LlmSettings::default(), &inner)
            .session_only
    );
    assert!(unavailable.key(&settings(), &inner).unwrap().is_none());
    let mut remote = settings();
    remote.base_url = Some("https://example.com".into());
    assert!(unavailable.key(&remote, &inner).is_err());
}
#[test]
fn preview_and_send_match_and_changes_require_new_review() {
    let service = service(false);
    let settings = settings();
    assert!(service
        .authorize(
            settings.clone(),
            prepare_diff(&request()).unwrap(),
            false,
            false
        )
        .is_err());
    let shown = preview(&service, &settings);
    assert!(shown.problems.is_empty());
    assert!(!shown.user_prompt.contains("PRIVATE"));
    assert_eq!(shown.label_map[0].0, "PRIVATE-A");
    let mut changed = settings.clone();
    changed.model = "different-model".into();
    assert!(service
        .authorize(changed, prepare_diff(&request()).unwrap(), false, false)
        .is_err());
    let sent = pending(&service, settings.clone(), false);
    let body: serde_json::Value = serde_json::from_str(&sent.wire.body).unwrap();
    assert_eq!(body["messages"][0]["content"], shown.system_prompt);
    assert_eq!(body["messages"][1]["content"], shown.user_prompt);
    assert_eq!(shown.bytes, sent.wire.body.len());
    assert_eq!(shown.destination, sent.wire.url);
    assert!(service
        .authorize(
            settings.clone(),
            prepare_diff(&request()).unwrap(),
            false,
            false
        )
        .is_err());
    preview(&service, &settings);
    let mut changed = request();
    if let SummaryRequest::Diff { baseline, .. } = &mut changed {
        *baseline = Some(1);
    }
    assert!(service
        .authorize(settings, prepare_diff(&changed).unwrap(), false, false)
        .is_err());
}
#[test]
fn disabled_missing_key_and_connection_preview_are_enforced() {
    let service = service(false);
    let mut off = settings();
    off.enabled = false;
    assert!(!preview(&service, &off).problems.is_empty());
    assert!(service
        .authorize(off.clone(), prepare_diff(&request()).unwrap(), false, false)
        .is_err());
    assert!(service
        .authorize(off.clone(), connection_prompt(), true, true)
        .is_err());
    assert!(service
        .preview(&off, connection_prompt(), true)
        .unwrap()
        .problems
        .is_empty());
    assert!(service
        .authorize(off, connection_prompt(), true, true)
        .is_ok());
    let cloud = LlmSettings {
        enabled: true,
        ..Default::default()
    };
    assert!(!preview(&service, &cloud).problems.is_empty());
}
#[test]
fn cache_regenerate_and_close_invalidation_use_the_same_execution_path() {
    tauri::async_runtime::block_on(async {
        let service = service(false);
        let settings = settings();
        let transport = FakeTransport::new();
        preview(&service, &settings);
        let permit = service.control.acquire().unwrap();
        let first = service
            .execute(pending(&service, settings.clone(), false), &transport)
            .await
            .unwrap();
        assert!(!first.cached);
        drop(permit);
        preview(&service, &settings);
        let permit = service.control.acquire().unwrap();
        assert!(
            service
                .execute(pending(&service, settings.clone(), false), &transport)
                .await
                .unwrap()
                .cached
        );
        drop(permit);
        assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
        preview(&service, &settings);
        let permit = service.control.acquire().unwrap();
        assert!(
            !service
                .execute(pending(&service, settings.clone(), true), &transport)
                .await
                .unwrap()
                .cached
        );
        drop(permit);
        assert_eq!(transport.calls.load(Ordering::SeqCst), 2);
        service.invalidate();
        assert!(service.inner.lock().unwrap().cache.is_empty());
        assert!(service
            .authorize(
                settings.clone(),
                prepare_diff(&request()).unwrap(),
                false,
                false
            )
            .is_err());
        preview(&service, &settings);
        let permit = service.control.acquire().unwrap();
        let prepared = pending(&service, settings, false);
        service.invalidate();
        assert!(service.execute(prepared, &transport).await.is_err());
        drop(permit);
        assert!(service.inner.lock().unwrap().cache.is_empty());
    });
}
#[test]
fn guard_rejects_concurrency_and_cancellation_drops_transport() {
    struct OnDrop(Arc<AtomicBool>);
    impl Drop for OnDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    struct Waiting(Arc<AtomicBool>);
    impl Transport for Waiting {
        fn send(&self, _: HttpRequest, _: Duration) -> Transfer {
            let flag = OnDrop(self.0.clone());
            Box::pin(async move {
                let _flag = flag;
                std::future::pending::<()>().await;
                unreachable!()
            })
        }
    }
    tauri::async_runtime::block_on(async {
        let control = Arc::new(Control::default());
        let permit = control.acquire().unwrap();
        assert!(control.acquire().is_err());
        let flag = Arc::new(AtomicBool::new(false));
        let worker_control = control.clone();
        let worker_flag = flag.clone();
        let task = tauri::async_runtime::spawn(async move {
            let settings = settings();
            let wire = build_request(&settings, None, &connection_prompt().prompt).unwrap();
            transfer(worker_control, &Waiting(worker_flag), wire, &settings).await
        });
        // Wait until the abort handle is registered, without a timing assumption.
        let deadline = Instant::now() + Duration::from_secs(2);
        while control.abort.lock().unwrap().is_none() {
            assert!(Instant::now() < deadline, "transport task did not start");
            std::thread::sleep(Duration::from_millis(1));
        }
        control.cancel();
        assert!(task.await.unwrap().is_err());
        assert!(flag.load(Ordering::SeqCst));
        drop(permit);
        assert!(control.acquire().is_ok());
    });
}
#[test]
fn inspector_preparation_excludes_secrets_and_tracks_document_hash() {
    let flightlens_core::Artifact::Config(d) = flightlens_core::analyze(
        "# Betaflight / STM32F405 4.5.0\nset pilot_name = PRIVATE-PILOT\n",
        "PRIVATE-TITLE",
        "PRIVATE-PATH",
    )
    .unwrap() else {
        panic!("config");
    };
    let request = SummaryRequest::Inspector {
        config_id: d.id.clone(),
        rate_profile: 0,
        pid_profile: 0,
    };
    let prepared = prepare(&request, &[&d]).unwrap();
    assert!(!prepared.prompt.user.contains("PRIVATE"));
    assert!(prepared.identity.contains(&d.hash));
    assert_eq!(prepared.labels[0].0, "PRIVATE-TITLE");
    assert!(prepared.excluded.iter().any(|r| r.key == "pilot_name"));
    assert!(prepare(&request, &[]).is_err());
}

// Actual reqwest adapter exercised only against a synthetic loopback server.
fn server(reply: Vec<u8>, delay: Duration) -> (String, std::thread::JoinHandle<()>) {
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut socket = loop {
            match listener.accept() {
                Ok((socket, _)) => break socket,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "no loopback request");
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(e) => panic!("synthetic server: {e}"),
            }
        };
        // Accepted sockets can inherit nonblocking mode on macOS.
        socket.set_nonblocking(false).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut received = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = socket.read(&mut chunk).unwrap();
            assert!(n > 0);
            received.extend_from_slice(&chunk[..n]);
            if let Some(end) = received.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&received[..end]);
                let length = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if received.len() >= end + 4 + length {
                    break;
                }
            }
        }
        std::thread::sleep(delay);
        let _ = socket.write_all(&reply);
    });
    (url, thread)
}
fn wire(url: String) -> HttpRequest {
    HttpRequest {
        url,
        headers: vec![("Content-Type".into(), "application/json".into())],
        body: "{}".into(),
    }
}
#[test]
fn http_adapter_rejects_redirects_and_bounds_streamed_and_declared_bodies() {
    tauri::async_runtime::block_on(async {
        let target = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        let (url, server) = server(format!("HTTP/1.1 302 Found\r\nLocation: http://{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", target.local_addr().unwrap()).into_bytes(), Duration::ZERO);
        let response = HttpTransport
            .send(wire(url), Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(response.status, 302);
        server.join().unwrap();
        assert!(target.accept().is_err());
        let (url, server) = self::server(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                RESPONSE_LIMIT + 1
            )
            .into_bytes(),
            Duration::ZERO,
        );
        assert!(HttpTransport
            .send(wire(url), Duration::from_secs(2))
            .await
            .err()
            .unwrap()
            .contains("size limit"));
        server.join().unwrap();
        let mut reply = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        reply.extend(vec![b'x'; RESPONSE_LIMIT + 1]);
        let (url, server) = self::server(reply, Duration::ZERO);
        assert!(HttpTransport
            .send(wire(url), Duration::from_secs(2))
            .await
            .err()
            .unwrap()
            .contains("size limit"));
        server.join().unwrap();
    });
}
#[test]
fn http_adapter_success_timeout_and_errors_do_not_echo_provider_bodies() {
    tauri::async_runtime::block_on(async {
        let body = r#"{"choices":[{"finish_reason":"stop","message":{"content":"OK"}}]}"#;
        let (url, server) = server(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_bytes(),
            Duration::ZERO,
        );
        let response = HttpTransport
            .send(wire(url), Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(
            parse_response(Provider::Custom, response.status, &response.body)
                .unwrap()
                .text,
            "OK"
        );
        server.join().unwrap();
        let (url, server) = self::server(
            b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 7\r\nConnection: close\r\n\r\nPRIVATE"
                .to_vec(),
            Duration::ZERO,
        );
        let response = HttpTransport
            .send(wire(url), Duration::from_secs(2))
            .await
            .unwrap();
        assert!(response.body.is_empty());
        server.join().unwrap();
        let (url, server) = self::server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec(),
            Duration::from_millis(250),
        );
        assert!(HttpTransport
            .send(wire(url), Duration::from_millis(100))
            .await
            .err()
            .unwrap()
            .contains("timed out after"));
        server.join().unwrap();
    });
}

#[test]
fn markdown_export_uses_cached_text_and_local_provenance_only() {
    let flightlens_core::Artifact::Config(d) = flightlens_core::analyze(
        "# Betaflight / STM32F405 4.5.0\nset pilot_name = PRIVATE-PILOT\n",
        "[Backup](https://invalid.test)\nInjected",
        "PRIVATE-PATH",
    )
    .unwrap() else {
        panic!("config")
    };
    let request = SummaryRequest::Inspector {
        config_id: d.id.clone(),
        rate_profile: 0,
        pid_profile: 1,
    };
    let service = service(false);
    let settings = settings();
    let mut summary = LlmSummary {
        text: "Hello\n\n![image](https://invalid.test)\n```\n<script>bad</script>".into(),
        provider: Provider::Custom,
        model: "model".into(),
        prompt_version: 1,
        generated_at: "1789680000".into(),
        cached: false,
    };
    assert!(cached_export(&service, &settings, &request, &[&d], &summary).is_err());
    let key = identity(&prepare(&request, &[&d]).unwrap(), &settings);
    service
        .inner
        .lock()
        .unwrap()
        .cache
        .insert(key, summary.clone());
    let text = cached_export(&service, &settings, &request, &[&d], &summary).unwrap();
    assert!(text.contains(&d.hash));
    assert!(text.contains("rate 1 (CLI 0), PID 2 (CLI 1)"));
    assert!(text.contains("Backup A: \\[Backup\\]"));
    assert!(text.contains("not a certified finding"));
    assert!(text.contains("\n    ![image]"));
    assert!(text.contains("\n    <script>"));
    assert!(!text.contains("PRIVATE-PILOT"));
    assert!(!text.contains("PRIVATE-PATH"));
    summary.text = "tampered".into();
    assert!(cached_export(&service, &settings, &request, &[&d], &summary).is_err());
    assert_eq!(utc_timestamp("0"), "1970-01-01T00:00:00Z");
    assert_eq!(utc_timestamp("951782400"), "2000-02-29T00:00:00Z");
    assert_eq!(utc_timestamp("4107542400"), "2100-03-01T00:00:00Z");
}

#[test]
fn comparison_preparation_binds_documents_profiles_and_baseline() {
    let mut docs = documents();
    let mut request = request();
    let original = prepare(&request, &docs.iter().collect::<Vec<_>>()).unwrap();
    assert!(prepare(&request, &[]).is_err());
    assert!(prepare(&request, &[&docs[1], &docs[0]]).is_err());
    assert!(original.identity.contains(&docs[0].hash));
    assert!(!original.json.contains("PRIVATE"));
    assert!(!original.json.contains(&docs[0].hash));
    docs[0].hash = "changed-hash".into();
    let changed = prepare(&request, &docs.iter().collect::<Vec<_>>()).unwrap();
    assert_ne!(original.identity, changed.identity);
    let SummaryRequest::Diff { slots, .. } = &mut request else {
        unreachable!()
    };
    slots[0].pid_profile = 1;
    let profile = prepare_diff(&request).unwrap();
    assert_ne!(original.identity, profile.identity);
    let SummaryRequest::Diff { slots, .. } = &mut request else {
        unreachable!()
    };
    slots[0].rate_profile = 255;
    assert!(prepare_diff(&request).is_err());
    let SummaryRequest::Diff {
        slots, baseline, ..
    } = &mut request
    else {
        unreachable!()
    };
    slots[0].rate_profile = 0;
    slots.push(SummarySlot {
        config_id: "C".into(),
        rate_profile: 0,
        pid_profile: 0,
    });
    *baseline = None;
    let mut third = documents().remove(0);
    third.id = "C".into();
    docs.push(third);
    assert!(prepare(&request, &docs.iter().collect::<Vec<_>>()).is_err());
    for index in 0..3 {
        let SummaryRequest::Diff { baseline, rows, .. } = &mut request else {
            unreachable!()
        };
        *baseline = Some(index);
        rows[0].values = vec![Some("14".into()), None, Some("12".into())];
        let prepared = prepare(&request, &docs.iter().collect::<Vec<_>>()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&prepared.json).unwrap();
        assert_eq!(json["baseline"], index);
        assert_eq!(
            json["rows"][0]["values"],
            serde_json::json!(["14", null, "12"])
        );
    }
}

#[test]
fn comparison_preview_excludes_sensitive_rows_before_sending() {
    let mut request = request();
    let SummaryRequest::Diff { rows, .. } = &mut request else {
        unreachable!()
    };
    rows.push(DiffRowInput {
        section: "parameters".into(),
        key: "pilot_name".into(),
        scope: "global".into(),
        status: "changed".into(),
        values: vec![Some("SECRET-A".into()), Some("SECRET-B".into())],
        reason: Some("SECRET-REASON".into()),
    });
    let prepared = prepare_diff(&request).unwrap();
    assert_eq!(prepared.excluded.len(), 1);
    assert_eq!(prepared.excluded[0].key, "pilot_name");
    assert!(!prepared.json.contains("SECRET"));
    assert!(!prepared.prompt.user.contains("SECRET"));
    let service = service(false);
    let settings = settings();
    let shown = service.preview(&settings, prepared, false).unwrap();
    assert_eq!(shown.excluded.len(), 1);
    let pending = service
        .authorize(settings, prepare_diff(&request).unwrap(), false, false)
        .unwrap();
    assert!(!pending.wire.body.contains("SECRET"));
    let body: serde_json::Value = serde_json::from_str(&pending.wire.body).unwrap();
    assert_eq!(body["messages"][1]["content"], shown.user_prompt);
}

#[test]
fn comparison_export_requires_exact_cache_and_lists_all_provenance() {
    let mut docs = documents();
    let mut request = request();
    let service = service(false);
    let settings = settings();
    let summary = LlmSummary {
        text: "<script>literal</script>\n![image](https://invalid.test)".into(),
        provider: Provider::Custom,
        model: "synthetic-model".into(),
        prompt_version: 1,
        generated_at: "1789680000".into(),
        cached: false,
    };
    for count in [2, 3] {
        if count == 3 {
            let mut third = documents().remove(0);
            third.id = "C".into();
            third.title = "[Third]\nInjected".into();
            third.hash = "third-hash".into();
            docs.push(third);
            let SummaryRequest::Diff {
                slots,
                rows,
                baseline,
            } = &mut request
            else {
                unreachable!()
            };
            slots.push(SummarySlot {
                config_id: "C".into(),
                rate_profile: 1,
                pid_profile: 2,
            });
            rows[0].values.push(None);
            *baseline = Some(2);
        }
        let refs = docs.iter().collect::<Vec<_>>();
        assert!(cached_export(&service, &settings, &request, &refs, &summary).is_err());
        let key = identity(&prepare(&request, &refs).unwrap(), &settings);
        service
            .inner
            .lock()
            .unwrap()
            .cache
            .insert(key, summary.clone());
        let text = cached_export(&service, &settings, &request, &refs, &summary).unwrap();
        assert!(text.contains("- Mode: Compare"));
        for (i, d) in docs.iter().enumerate() {
            assert!(text.contains(&format!(
                "- Backup {}: {} (SHA-256 {})",
                ["A", "B", "C"][i],
                markdown_label(&d.title),
                markdown_label(&d.hash)
            )));
        }
        assert!(text.contains("\n    <script>literal</script>"));
        assert!(text.contains("not a certified finding"));
        assert!(!text.contains("PRIVATE-PATH"));
        if count == 3 {
            assert!(text.contains("- Baseline: Backup C"));
            assert!(text.contains("rate 2 (CLI 1), PID 3 (CLI 2)"));
            assert!(text.contains("\\[Third\\] Injected"));
        }
        let mut changed = request.clone();
        let SummaryRequest::Diff { slots, .. } = &mut changed else {
            unreachable!()
        };
        slots[0].pid_profile = 1;
        assert!(cached_export(&service, &settings, &changed, &refs, &summary).is_err());
        assert!(cached_export(&service, &settings, &request, &refs[..1], &summary).is_err());
        let mut tampered = summary.clone();
        tampered.text = "changed".into();
        assert!(cached_export(&service, &settings, &request, &refs, &tampered).is_err());
    }
    docs[0].hash = "replacement".into();
    assert!(cached_export(
        &service,
        &settings,
        &request,
        &docs.iter().collect::<Vec<_>>(),
        &summary
    )
    .is_err());
}
