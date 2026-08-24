use dshtray_lib::{
    config::ConfigStore,
    domain::{AppConfig, TargetId},
};

#[test]
fn defaults_match_approved_product_decisions() {
    let config = AppConfig::defaults();
    assert!(config.manager.start_on_login);
    assert!(!config.manager.start_dsh_on_login);
    assert!(config.manager.close_to_tray);
    assert_eq!(config.active_target, TargetId::Source);
    assert_eq!(config.service.host, "127.0.0.1");
    assert_eq!(config.service.port, 3080);
    assert!(config.proxy.enabled);
    assert_eq!(config.proxy.url, "http://127.0.0.1:7897");
}

#[test]
fn invalid_host_is_rejected() {
    let mut config = AppConfig::defaults();
    config.service.host = "0.0.0.0".into();
    let error = config.validate().expect_err("non-loopback host must fail");
    assert_eq!(error.code, "invalid_service_host");
}

#[test]
fn invalid_proxy_scheme_is_rejected() {
    let mut config = AppConfig::defaults();
    config.proxy.url = "socks5://127.0.0.1:7897".into();
    let error = config.validate().expect_err("socks5 is outside the MVP");
    assert_eq!(error.code, "invalid_proxy_url");
}

#[test]
fn empty_default_target_is_allowed_until_first_run_is_completed() {
    let config = AppConfig::defaults();
    config
        .validate()
        .expect("empty targets are valid for first run");
    let error = config
        .validate_active_target()
        .expect_err("active source target still needs a path");
    assert_eq!(error.code, "target_not_configured");
}

#[test]
fn corrupt_config_is_backed_up_and_defaults_are_returned() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("config.json");
    std::fs::write(&path, b"{not-json").expect("write corrupt config");
    let loaded = ConfigStore::load(&path).expect("corruption recovery");
    assert!(loaded.recovered);
    assert!(loaded
        .backup_path
        .as_ref()
        .is_some_and(|backup| backup.exists()));
    assert_eq!(loaded.config.service.port, 3080);
}

#[test]
fn save_then_load_round_trips_camel_case_json() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("config.json");
    let original = AppConfig::defaults();
    ConfigStore::save(&path, &original).expect("save config");
    let loaded = ConfigStore::load(&path).expect("load config");
    assert_eq!(loaded.config, original);
    let json = std::fs::read_to_string(path).expect("read config");
    assert!(json.contains("startDshOnLogin"));
}
