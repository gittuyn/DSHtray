use dshtray_lib::logging::{log_dsh_line, redact_url};

#[test]
fn proxy_credentials_are_redacted_in_log_url() {
    let redacted = redact_url("http://user:secret@127.0.0.1:7897");
    assert_eq!(redacted, "http://***:***@127.0.0.1:7897");
    assert!(!redacted.contains("secret"));
}

#[test]
fn dsh_output_redacts_token_like_values() {
    let fixture_value = ["fixture", "redacted", "value"].join("-");
    let line = format!("Authorization: Bearer {}", fixture_value);
    let safe = log_dsh_line("stdout", &line);
    assert!(!safe.contains(&fixture_value));
    assert!(safe.contains("[REDACTED]"));
}
