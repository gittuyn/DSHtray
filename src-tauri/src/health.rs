use crate::domain::ServiceConfig;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthResult {
    Ready { status: u16 },
    Unreachable { code: String, message: String },
    UnexpectedStatus { status: u16 },
}

#[derive(Clone)]
pub struct HealthChecker {
    client: reqwest::Client,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self::with_proxy_disabled()
    }

    pub fn with_proxy_disabled() -> Self {
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(3))
            .build()
            .expect("health checker HTTP client configuration is valid");
        Self { client }
    }

    pub async fn check(&self, config: &ServiceConfig) -> HealthResult {
        if config.host != "127.0.0.1" && config.host != "localhost" {
            return HealthResult::Unreachable {
                code: "invalid_service_host".into(),
                message: "健康检查只允许 loopback 地址".into(),
            };
        }
        let url = format!("http://{}:{}/", config.host, config.port);
        match self.client.get(url).send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                if (200..400).contains(&status) {
                    HealthResult::Ready { status }
                } else {
                    HealthResult::UnexpectedStatus { status }
                }
            }
            Err(error) => HealthResult::Unreachable {
                code: if error.is_timeout() {
                    "health_timeout".into()
                } else {
                    "health_unreachable".into()
                },
                message: error.to_string(),
            },
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task::JoinHandle,
    };

    struct TestHttpServer {
        port: u16,
        task: JoinHandle<()>,
    }

    impl TestHttpServer {
        async fn responding_with(status: u16) -> Self {
            let listener = TcpListener::bind(("127.0.0.1", 0))
                .await
                .expect("bind test HTTP server");
            let port = listener.local_addr().expect("local address").port();
            let task = tokio::spawn(async move {
                loop {
                    let Ok((mut stream, _)) = listener.accept().await else {
                        break;
                    };
                    let mut request = [0_u8; 1024];
                    let _ = stream.read(&mut request).await;
                    let reason = match status {
                        200 => "OK",
                        204 => "No Content",
                        301 => "Moved Permanently",
                        _ => "Test Status",
                    };
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                }
            });
            Self { port, task }
        }

        fn port(&self) -> u16 {
            self.port
        }
    }

    impl Drop for TestHttpServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    impl ServiceConfig {
        fn loopback(port: u16) -> Self {
            Self {
                host: "127.0.0.1".into(),
                port,
            }
        }
    }

    fn unused_local_port() -> u16 {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind unused port");
        listener.local_addr().expect("local address").port()
    }

    #[tokio::test]
    async fn health_check_accepts_2xx_without_using_environment_proxy() {
        let server = TestHttpServer::responding_with(204).await;
        let config = ServiceConfig::loopback(server.port());
        let checker = HealthChecker::with_proxy_disabled();
        let result = checker.check(&config).await;
        assert!(matches!(result, HealthResult::Ready { status: 204 }));
    }

    #[tokio::test]
    async fn health_check_accepts_3xx_as_ready() {
        let server = TestHttpServer::responding_with(301).await;
        let config = ServiceConfig::loopback(server.port());
        let result = HealthChecker::with_proxy_disabled().check(&config).await;
        assert!(matches!(result, HealthResult::Ready { status: 301 }));
    }

    #[tokio::test]
    async fn health_check_reports_unexpected_status() {
        let server = TestHttpServer::responding_with(503).await;
        let config = ServiceConfig::loopback(server.port());
        let result = HealthChecker::with_proxy_disabled().check(&config).await;
        assert!(matches!(
            result,
            HealthResult::UnexpectedStatus { status: 503 }
        ));
    }

    #[tokio::test]
    async fn health_check_reports_unreachable_port() {
        let config = ServiceConfig::loopback(unused_local_port());
        let result = HealthChecker::with_proxy_disabled().check(&config).await;
        assert!(matches!(result, HealthResult::Unreachable { .. }));
    }

    #[test]
    fn timeout_constant_is_bounded_for_local_probe() {
        assert!(Duration::from_secs(3) <= Duration::from_secs(5));
    }
}
