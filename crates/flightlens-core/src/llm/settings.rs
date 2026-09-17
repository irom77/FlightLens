use super::{LlmError, Provider};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LlmSettings {
    pub enabled: bool,
    pub provider: Provider,
    pub model: String,
    pub base_url: Option<String>,
    pub timeout_seconds: u32,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self::for_provider(Provider::Gemini)
    }
}

impl LlmSettings {
    pub fn for_provider(provider: Provider) -> Self {
        Self {
            enabled: false,
            provider,
            model: provider.default_model().into(),
            base_url: (provider == Provider::Custom).then(|| "http://localhost:11434".into()),
            timeout_seconds: 60,
        }
    }

    /// Must be used on both settings-file reads and incoming settings writes.
    pub fn validated(&self) -> Result<Self, LlmError> {
        let mut settings = self.clone();
        let model = &settings.model;
        if model.is_empty()
            || model.len() > 200
            || !model.bytes().all(|c| {
                c.is_ascii_alphanumeric()
                    || matches!(c, b'-' | b'_' | b'.')
                    || (self.provider != Provider::Gemini && matches!(c, b'/' | b':'))
            })
            || model.split('/').any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(LlmError::InvalidModel);
        }
        settings.base_url = match (settings.provider, settings.base_url.as_deref()) {
            (Provider::Custom, Some(url)) => Some(validate_base_url(url)?),
            (Provider::Custom, None) | (_, Some(_)) => return Err(LlmError::InvalidBaseUrl),
            (_, None) => None,
        };
        settings.timeout_seconds = settings.timeout_seconds.clamp(5, 180);
        Ok(settings)
    }
}

/// Strict ASCII URL subset, deliberately rejecting browser URL repairs (slashes,
/// whitespace, encoded hosts and abbreviated/numeric IPv4). No DNS or network IO.
/// The shell must also disable redirects so a validated endpoint stays the target.
pub fn validate_base_url(input: &str) -> Result<String, LlmError> {
    let invalid = LlmError::InvalidBaseUrl;
    if input.len() > 2048
        || !input.is_ascii()
        || input
            .bytes()
            .any(|c| c.is_ascii_control() || c.is_ascii_whitespace())
        || input.contains(['@', '?', '#', '\\'])
    {
        return Err(invalid);
    }
    let (scheme, rest) = input.split_once("://").ok_or(invalid)?;
    let scheme = scheme.to_ascii_lowercase();
    if !matches!(scheme.as_str(), "https" | "http") {
        return Err(invalid);
    }
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (host, port, loopback) = if let Some(ipv6) = authority.strip_prefix('[') {
        let (host, suffix) = ipv6.split_once(']').ok_or(invalid)?;
        let ip: Ipv6Addr = host.parse().map_err(|_| invalid)?;
        let port = if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix(':').ok_or(invalid)?)
        };
        (format!("[{ip}]"), port, ip.is_loopback())
    } else {
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(h, p)| (h, Some(p)));
        if host.is_empty() || host.len() > 253 {
            return Err(invalid);
        }
        let loopback = if let Ok(ip) = host.parse::<Ipv4Addr>() {
            ip.is_loopback()
        } else {
            // Numeric final labels trigger WHATWG's alternative IPv4 parsing.
            let last = host.rsplit('.').next().ok_or(invalid)?;
            if last.is_empty()
                || last.bytes().all(|c| c.is_ascii_digit())
                || last.to_ascii_lowercase().starts_with("0x")
            {
                return Err(invalid);
            }
            if !host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            }) {
                return Err(invalid);
            }
            host.eq_ignore_ascii_case("localhost")
        };
        (host.to_ascii_lowercase(), port, loopback)
    };
    if let Some(port) = port {
        if port.is_empty()
            || !port.bytes().all(|c| c.is_ascii_digit())
            || !matches!(port.parse::<u16>(), Ok(1..=u16::MAX))
        {
            return Err(invalid);
        }
    }
    if scheme == "http" && !loopback {
        return Err(invalid);
    }
    // Endpoint prefixes only: no encoded separators or dot-segment repairs.
    if !path
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'/' | b'-' | b'_' | b'.' | b'~'))
        || path.split('/').any(|part| matches!(part, "." | ".."))
    {
        return Err(invalid);
    }
    let port = port.map(|p| format!(":{p}")).unwrap_or_default();
    let path = path.trim_end_matches('/');
    Ok(if path.is_empty() {
        format!("{scheme}://{host}{port}")
    } else {
        format!("{scheme}://{host}{port}/{path}")
    })
}
