use flightlens_core::llm::*;
use serde_json::{json, Value};

fn prompt() -> Prompt {
    Prompt {
        system: "Synthetic system instruction".into(),
        user: "Synthetic digest".into(),
    }
}

fn settings(provider: Provider) -> LlmSettings {
    let mut settings = LlmSettings::for_provider(provider);
    if provider == Provider::Custom {
        settings.model = "local-model:latest".into();
    }
    settings
}

#[test]
fn settings_are_disabled_by_default_and_round_trip_without_credentials() {
    for provider in [
        Provider::Gemini,
        Provider::Openai,
        Provider::Openrouter,
        Provider::Custom,
    ] {
        let settings = settings(provider);
        assert!(!settings.enabled);
        assert_eq!(settings.timeout_seconds, 60);
        let encoded = serde_json::to_string(&settings).unwrap();
        assert_eq!(
            serde_json::from_str::<LlmSettings>(&encoded)
                .unwrap()
                .validated()
                .unwrap(),
            settings
        );
        let mut value = serde_json::to_value(settings).unwrap();
        value["key"] = json!("synthetic-key");
        assert!(serde_json::from_value::<LlmSettings>(value).is_err());
    }
    assert_eq!(LlmSettings::default().provider, Provider::Gemini);
    assert!(LlmSettings::for_provider(Provider::Custom)
        .validated()
        .is_err());
}

#[test]
fn settings_clamp_timeouts_and_reject_invalid_stored_destinations() {
    let mut s = settings(Provider::Custom);
    for (input, expected) in [(0, 5), (5, 5), (60, 60), (180, 180), (u32::MAX, 180)] {
        s.timeout_seconds = input;
        assert_eq!(s.validated().unwrap().timeout_seconds, expected);
    }
    s.base_url = Some("http://remote.example".into());
    let stored = serde_json::to_string(&s).unwrap();
    assert_eq!(
        serde_json::from_str::<LlmSettings>(&stored)
            .unwrap()
            .validated(),
        Err(LlmError::InvalidBaseUrl)
    );
    s.base_url = None;
    assert_eq!(s.validated(), Err(LlmError::InvalidBaseUrl));
    for provider in [Provider::Gemini, Provider::Openai, Provider::Openrouter] {
        s = settings(provider);
        s.base_url = Some("https://other.example".into());
        assert_eq!(s.validated(), Err(LlmError::InvalidBaseUrl));
    }
}

#[test]
fn validates_absolute_urls_and_loopback_without_network_access() {
    for (input, expected) in [
        ("https://models.example/", "https://models.example"),
        (
            "HTTPS://MODELS.EXAMPLE/api///",
            "https://models.example/api",
        ),
        (
            "https://192.168.1.8:8443/prefix",
            "https://192.168.1.8:8443/prefix",
        ),
        ("http://LOCALHOST:11434/", "http://localhost:11434"),
        ("http://127.0.0.1:1234", "http://127.0.0.1:1234"),
        ("http://127.255.255.254", "http://127.255.255.254"),
        ("http://[::1]:11434", "http://[::1]:11434"),
        ("http://[0:0:0:0:0:0:0:1]/", "http://[::1]"),
        ("https://[2001:db8::1]/api", "https://[2001:db8::1]/api"),
    ] {
        assert_eq!(validate_base_url(input).unwrap(), expected, "{input}");
    }
}

#[test]
fn rejects_unsafe_or_ambiguous_urls_without_echoing_them() {
    for input in [
        "",
        "localhost:11434",
        "//localhost",
        "https:///api",
        "ftp://localhost",
        "http://models.example",
        "http://192.168.1.8",
        "http://localhost.example",
        "http://[::2]",
        "http://[::ffff:127.0.0.1]",
        "http://128.0.0.1",
        "https://user:secret@models.example",
        "https://@models.example",
        "https://models.example?secret=x",
        "https://models.example#secret",
        "https://models.example?",
        "https://models.example\\@localhost",
        "https://models.example\n",
        " https://models.example",
        "https://models.example:abc",
        "https://models.example:65536",
        "https://models.example:0",
        "https://models.example:0000",
        "https://models.example:",
        "https://models.example:+443",
        "https://models.example:443:80",
        "https://[::1",
        "https://[::1]bad",
        "https://[]",
        "https://%6cocalhost",
        "http://127.1",
        "http://2130706433",
        "http://0x7f000001",
        "http://0177.0.0.1",
        "http://localhost.",
        "https://bad..host",
        "https://-bad.example",
        "https://bad_.example",
        "https://例.example",
        "https://example.test/a/../b",
        "https://example.test/./b",
        "https://example.test/%2e%2e/b",
        "https://example.test/a b",
        "https://example.test/\t",
    ] {
        let error = validate_base_url(input).unwrap_err();
        assert_eq!(error, LlmError::InvalidBaseUrl, "{input}");
        assert!(!error.to_string().contains("secret"));
    }
}

#[test]
fn model_identifiers_cannot_inject_paths_queries_or_control_characters() {
    for provider in [
        Provider::Gemini,
        Provider::Openai,
        Provider::Openrouter,
        Provider::Custom,
    ] {
        for model in [
            "",
            " ",
            "../model",
            "a/../b",
            "model?key=x",
            "x#fragment",
            "x\ny",
            "a%2Fb",
        ] {
            let mut s = settings(provider);
            s.model = model.into();
            assert_eq!(
                build_request(&s, Some("synthetic-key"), &prompt()).unwrap_err(),
                LlmError::InvalidModel
            );
        }
    }
    let mut s = settings(Provider::Gemini);
    s.model = "models/gemini".into();
    assert_eq!(s.validated(), Err(LlmError::InvalidModel));
    s.model = "a".repeat(201);
    assert_eq!(s.validated(), Err(LlmError::InvalidModel));
}

#[test]
fn builds_gemini_request_with_key_only_in_header() {
    let request = build_request(
        &settings(Provider::Gemini),
        Some("synthetic-key"),
        &prompt(),
    )
    .unwrap();
    assert_eq!(
        request.url,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.8-flash:generateContent"
    );
    assert_eq!(
        request.headers,
        vec![
            ("Content-Type".into(), "application/json".into()),
            ("x-goog-api-key".into(), "synthetic-key".into())
        ]
    );
    let body: Value = serde_json::from_str(&request.body).unwrap();
    assert_eq!(
        body,
        json!({
            "systemInstruction": {"parts": [{"text": prompt().system}]},
            "contents": [{"role": "user", "parts": [{"text": prompt().user}]}],
            "generationConfig": {"temperature": 0.2, "maxOutputTokens": 1024}
        })
    );
    assert!(!request.body.contains("synthetic-key"));
}

#[test]
fn builds_compatible_requests_for_each_provider() {
    for (provider, url, model) in [
        (
            Provider::Openai,
            "https://api.openai.com/v1/chat/completions",
            "gpt-4.1-mini",
        ),
        (
            Provider::Openrouter,
            "https://openrouter.ai/api/v1/chat/completions",
            "openai/gpt-4.1-mini",
        ),
        (
            Provider::Custom,
            "http://localhost:11434/v1/chat/completions",
            "local-model:latest",
        ),
    ] {
        let request = build_request(&settings(provider), Some("synthetic-key"), &prompt()).unwrap();
        assert_eq!(request.url, url);
        assert!(request
            .headers
            .contains(&("Authorization".into(), "Bearer synthetic-key".into())));
        assert_eq!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "X-Title" && value == "FlightLens"),
            provider == Provider::Openrouter
        );
        assert!(!request
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("http-referer")));
        assert_eq!(
            serde_json::from_str::<Value>(&request.body).unwrap(),
            json!({
                "model": model, "messages": [{"role": "system", "content": prompt().system}, {"role": "user", "content": prompt().user}],
                "temperature": 0.2, "max_tokens": 1024
            })
        );
    }
    let mut s = settings(Provider::Custom);
    s.base_url = Some("https://models.example/prefix/".into());
    assert_eq!(
        build_request(&s, None, &prompt()).unwrap().url,
        "https://models.example/prefix/v1/chat/completions"
    );
}

#[test]
fn requires_cloud_keys_but_omits_empty_custom_auth() {
    for key in [None, Some("")] {
        for provider in [Provider::Gemini, Provider::Openai, Provider::Openrouter] {
            assert_eq!(
                build_request(&settings(provider), key, &prompt()).unwrap_err(),
                LlmError::MissingKey
            );
        }
        let request = build_request(&settings(Provider::Custom), key, &prompt()).unwrap();
        assert_eq!(request.headers.len(), 1);
    }
    for key in ["\r\nX-Injected: value", "a b", "\t", "é"] {
        assert_eq!(
            build_request(&settings(Provider::Custom), Some(key), &prompt()).unwrap_err(),
            LlmError::InvalidKey
        );
    }
}

#[test]
fn debug_output_never_contains_headers_payload_or_model_output() {
    for provider in [
        Provider::Gemini,
        Provider::Openai,
        Provider::Openrouter,
        Provider::Custom,
    ] {
        let request =
            build_request(&settings(provider), Some("synthetic-secret"), &prompt()).unwrap();
        let debug = format!("{request:?}");
        for hidden in [
            "synthetic-secret",
            "Synthetic digest",
            "Synthetic system instruction",
        ] {
            assert!(!debug.contains(hidden));
        }
    }
    assert!(!format!("{:?}", prompt()).contains("Synthetic"));
    assert!(!format!(
        "{:?}",
        Summary {
            text: "synthetic-output".into()
        }
    )
    .contains("synthetic-output"));
}

#[test]
fn decodes_gemini_text_parts_without_thoughts() {
    let response = json!({"candidates": [{"finishReason": "STOP", "content": {"parts": [
        {"text": "internal", "thought": true}, {"text": "First "}, {"text": "second."}
    ]}}]})
    .to_string();
    assert_eq!(
        parse_response(Provider::Gemini, 200, &response)
            .unwrap()
            .text,
        "First second."
    );
}

#[test]
fn decodes_compatible_text_verbatim() {
    for provider in [Provider::Openai, Provider::Openrouter, Provider::Custom] {
        let response = json!({"choices": [{"finish_reason": "stop", "message": {"content": "<b>untrusted</b>\n"}}]}).to_string();
        assert_eq!(
            parse_response(provider, 200, &response).unwrap().text,
            "<b>untrusted</b>\n"
        );
    }
}

#[test]
fn maps_http_errors_without_parsing_or_echoing_the_body() {
    for provider in [
        Provider::Gemini,
        Provider::Openai,
        Provider::Openrouter,
        Provider::Custom,
    ] {
        for (status, expected) in [
            (400, LlmError::RequestRejected),
            (401, LlmError::RejectedKey),
            (403, LlmError::RejectedKey),
            (404, LlmError::ModelNotFound),
            (429, LlmError::RateLimited),
            (500, LlmError::ProviderUnavailable),
            (503, LlmError::ProviderUnavailable),
            (302, LlmError::RequestRejected),
        ] {
            let error = parse_response(provider, status, "synthetic-secret").unwrap_err();
            assert_eq!(error, expected);
            assert!(!error.to_string().contains("synthetic-secret"));
        }
        assert_eq!(
            parse_response(
                provider,
                200,
                r#"{"error":{"message":"synthetic-secret","code":400}}"#
            ),
            Err(LlmError::RequestRejected)
        );
        assert_eq!(
            parse_response(provider, 200, "not JSON synthetic-secret"),
            Err(LlmError::InvalidResponse)
        );
        assert_eq!(
            parse_response(provider, 200, &"x".repeat(1_048_577)),
            Err(LlmError::InvalidResponse)
        );
    }
}

#[test]
fn gemini_reports_blocking_truncation_incomplete_and_empty_results() {
    assert_eq!(
        parse_response(
            Provider::Gemini,
            200,
            r#"{"promptFeedback":{"blockReason":"SAFETY"}}"#
        ),
        Err(LlmError::Blocked)
    );
    for (reason, expected) in [
        ("MAX_TOKENS", LlmError::Truncated),
        ("SAFETY", LlmError::Blocked),
        ("RECITATION", LlmError::Blocked),
        ("OTHER", LlmError::Incomplete),
    ] {
        let body = json!({"candidates": [{"finishReason": reason, "content": {"parts": [{"text": "Partial text"}]}}]}).to_string();
        assert_eq!(parse_response(Provider::Gemini, 200, &body), Err(expected));
    }
    for body in [
        r#"{"candidates":[]}"#,
        r#"{"candidates":[{"finishReason":"STOP","content":{"parts":[]}}]}"#,
        r#"{"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"  "}]}}]}"#,
    ] {
        assert_eq!(
            parse_response(Provider::Gemini, 200, body),
            Err(LlmError::EmptyResponse)
        );
    }
    assert_eq!(
        parse_response(
            Provider::Gemini,
            200,
            r#"{"candidates":[{"content":{"parts":[{"text":"unconfirmed"}]}}]}"#
        ),
        Err(LlmError::Incomplete)
    );
    assert_eq!(
        parse_response(
            Provider::Gemini,
            200,
            r#"{"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":42}]}}]}"#
        ),
        Err(LlmError::InvalidResponse)
    );
}

#[test]
fn compatible_reports_truncation_refusal_incomplete_and_empty_results() {
    for provider in [Provider::Openai, Provider::Openrouter, Provider::Custom] {
        for (reason, expected) in [
            ("length", LlmError::Truncated),
            ("content_filter", LlmError::Blocked),
            ("tool_calls", LlmError::Incomplete),
            ("other", LlmError::Incomplete),
        ] {
            let body = json!({"choices": [{"finish_reason": reason, "message": {"content": "Partial text"}}]}).to_string();
            assert_eq!(parse_response(provider, 200, &body), Err(expected));
        }
        for content in [Value::Null, json!(""), json!(" \n"), json!([])] {
            let body =
                json!({"choices": [{"finish_reason": "stop", "message": {"content": content}}]})
                    .to_string();
            assert_eq!(
                parse_response(provider, 200, &body),
                Err(LlmError::EmptyResponse)
            );
        }
        assert_eq!(
            parse_response(provider, 200, r#"{"choices":[]}"#),
            Err(LlmError::EmptyResponse)
        );
        assert_eq!(
            parse_response(
                provider,
                200,
                r#"{"choices":[{"message":{"content":"unconfirmed"}}]}"#
            ),
            Err(LlmError::Incomplete)
        );
        assert_eq!(
            parse_response(
                provider,
                200,
                r#"{"choices":[{"finish_reason":"stop","message":{"refusal":"refused"}}]}"#
            ),
            Err(LlmError::Blocked)
        );
    }
}
