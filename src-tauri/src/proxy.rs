use crate::{app_error::AppError, domain::ProxyConfig};
use std::ffi::OsString;
use url::Url;

pub fn validate_proxy_url(value: &str) -> Result<(), AppError> {
    let parsed =
        Url::parse(value).map_err(|_| AppError::new("invalid_proxy_url", "代理 URL 格式无效"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AppError::new(
            "invalid_proxy_url",
            "代理 URL 只能使用 http 或 https 且必须包含主机",
        ));
    }
    Ok(())
}

pub fn build_child_environment(
    proxy: &ProxyConfig,
    parent: &[(OsString, OsString)],
) -> Vec<(OsString, OsString)> {
    if !proxy.enabled {
        return parent.to_vec();
    }

    let mut result = parent.to_vec();
    set_value(&mut result, "HTTP_PROXY", &proxy.url);
    set_value(&mut result, "HTTPS_PROXY", &proxy.url);
    set_value(&mut result, "NODE_USE_ENV_PROXY", "1");
    result
}

fn set_value(environment: &mut Vec<(OsString, OsString)>, name: &str, value: &str) {
    environment.retain(|(key, _)| !key.to_string_lossy().eq_ignore_ascii_case(name));
    environment.push((OsString::from(name), OsString::from(value)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_proxy() -> ProxyConfig {
        ProxyConfig {
            enabled: true,
            url: "http://127.0.0.1:7897".into(),
        }
    }

    fn disabled_proxy() -> ProxyConfig {
        ProxyConfig {
            enabled: false,
            url: "http://127.0.0.1:7897".into(),
        }
    }

    fn value(env: &[(OsString, OsString)], name: &str) -> Option<String> {
        env.iter()
            .find(|(key, _)| key.to_string_lossy().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.to_string_lossy().into_owned())
    }

    #[test]
    fn enabled_proxy_adds_only_approved_variables() {
        let parent = vec![(OsString::from("NO_PROXY"), OsString::from("127.0.0.1"))];
        let env = build_child_environment(&enabled_proxy(), &parent);
        assert_eq!(
            value(&env, "HTTP_PROXY"),
            Some("http://127.0.0.1:7897".into())
        );
        assert_eq!(
            value(&env, "HTTPS_PROXY"),
            Some("http://127.0.0.1:7897".into())
        );
        assert_eq!(value(&env, "NODE_USE_ENV_PROXY"), Some("1".into()));
        assert_eq!(value(&env, "NO_PROXY"), Some("127.0.0.1".into()));
        assert_eq!(value(&env, "ALL_PROXY"), None);
    }

    #[test]
    fn enabled_proxy_replaces_case_insensitive_existing_values() {
        let parent = vec![(OsString::from("http_proxy"), OsString::from("old"))];
        let env = build_child_environment(&enabled_proxy(), &parent);
        assert_eq!(
            value(&env, "HTTP_PROXY"),
            Some("http://127.0.0.1:7897".into())
        );
        assert_eq!(
            env.iter()
                .filter(|(key, _)| key.to_string_lossy().eq_ignore_ascii_case("HTTP_PROXY"))
                .count(),
            1
        );
    }

    #[test]
    fn disabled_proxy_does_not_add_or_remove_environment_values() {
        let parent = vec![(OsString::from("HTTP_PROXY"), OsString::from("inherited"))];
        assert_eq!(build_child_environment(&disabled_proxy(), &parent), parent);
    }

    #[test]
    fn invalid_proxy_scheme_is_rejected() {
        let error =
            validate_proxy_url("socks5://127.0.0.1:7897").expect_err("socks5 is outside the MVP");
        assert_eq!(error.code, "invalid_proxy_url");
    }
}
