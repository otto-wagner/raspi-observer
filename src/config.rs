#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub environment: Environment,
    pub host_proc: String,
    pub enable_docker_metrics: bool,
    pub enable_raspi_metrics: bool,
    pub enable_node_metrics: bool,
    pub metrics_interval_sec: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Development,
    Production,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            environment: Environment::Production,
            host_proc: "/proc".to_string(),
            enable_docker_metrics: true,
            enable_raspi_metrics: true,
            enable_node_metrics: true,
            metrics_interval_sec: 15,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let env = std::env::var("ENVIRONMENT")
            .map(|e| match e.as_str() {
                "development" => Environment::Development,
                _ => Environment::Production,
            })
            .unwrap_or(Environment::Production);

        let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = std::env::var("SERVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let host_proc = std::env::var("HOST_PROC").unwrap_or_else(|_| "/proc".to_string());

        let enable_docker_metrics = std::env::var("ENABLE_DOCKER_METRICS")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);

        let enable_raspi_metrics = std::env::var("ENABLE_RASPI_METRICS")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);

        let enable_node_metrics = std::env::var("ENABLE_NODE_METRICS")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);

        let metrics_interval_sec = std::env::var("METRICS_INTERVAL_SEC")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15);

        Self {
            host,
            port,
            environment: env,
            host_proc,
            enable_docker_metrics,
            enable_raspi_metrics,
            enable_node_metrics,
            metrics_interval_sec,
        }
    }

    pub fn server_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}
