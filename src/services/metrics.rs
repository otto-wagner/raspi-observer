use std::sync::Once;

use lazy_static::lazy_static;
use prometheus::{CounterVec, IntGaugeVec, Opts, Registry};

#[derive(Debug, Clone)]
pub struct DockerContainerMetrics {
    pub container_id: String,
    pub container_name: String,
    pub image: String,
    pub state: i64,
    pub health_status: Option<i64>,
    pub restart_count: u64,
    pub last_exit_code: i64,
    pub uptime_seconds: i64,
    pub mem_limit_bytes: i64,
    pub cpu_seconds_total: f64,
    pub mem_usage_bytes: i64,
    pub net_receive_bytes_total: u64,
    pub net_transmit_bytes_total: u64,
    pub blkio_read_bytes_total: u64,
    pub blkio_write_bytes_total: u64,
}

static METRICS_INIT: Once = Once::new();

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref DOCKER_CONTAINER_STATE: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "docker_container_state",
            "Running state: 1=running, 0=stopped/exited"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_HEALTH_STATUS: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "docker_container_health_status",
            "Healthcheck: 1=healthy, 0=unhealthy, 2=starting"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_RESTART_COUNT: CounterVec = CounterVec::new(
        Opts::new("docker_container_restart_count", "Total number of restarts"),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_LAST_EXIT_CODE: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "docker_container_last_exit_code",
            "Last exit code (0=clean, 137=OOM/SIGKILL)"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_UPTIME_SECONDS: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "docker_container_uptime_seconds",
            "Seconds since container started (0=stopped)"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_MEM_LIMIT_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "docker_container_mem_limit_bytes",
            "Configured memory limit in bytes (0=unlimited)"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_CPU_SECONDS_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "docker_container_cpu_seconds_total",
            "Cumulative CPU time in seconds"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_MEM_USAGE_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "docker_container_mem_usage_bytes",
            "Memory working set in bytes (excl. cache)"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_NET_RECEIVE_BYTES_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "docker_container_net_receive_bytes_total",
            "Cumulative network RX bytes"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_NET_TRANSMIT_BYTES_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "docker_container_net_transmit_bytes_total",
            "Cumulative network TX bytes"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_BLKIO_READ_BYTES_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "docker_container_blkio_read_bytes_total",
            "Cumulative block I/O read bytes"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
    pub static ref DOCKER_CONTAINER_BLKIO_WRITE_BYTES_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "docker_container_blkio_write_bytes_total",
            "Cumulative block I/O write bytes"
        ),
        &["container_id", "container_name", "image"]
    )
    .expect("metric can be created");
}

pub fn init_metrics() {
    METRICS_INIT.call_once(|| {
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_STATE.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_HEALTH_STATUS.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_RESTART_COUNT.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_LAST_EXIT_CODE.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_UPTIME_SECONDS.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_MEM_LIMIT_BYTES.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_CPU_SECONDS_TOTAL.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_MEM_USAGE_BYTES.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_NET_RECEIVE_BYTES_TOTAL.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_NET_TRANSMIT_BYTES_TOTAL.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_BLKIO_READ_BYTES_TOTAL.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(DOCKER_CONTAINER_BLKIO_WRITE_BYTES_TOTAL.clone()))
            .expect("collector can be registered");
    });
}

pub fn update_docker_container_metrics(samples: Vec<DockerContainerMetrics>) {
    // Poll-basierter Collector: bei jedem Lauf den aktuellen Satz vollständig neu schreiben.
    DOCKER_CONTAINER_STATE.reset();
    DOCKER_CONTAINER_HEALTH_STATUS.reset();
    DOCKER_CONTAINER_RESTART_COUNT.reset();
    DOCKER_CONTAINER_LAST_EXIT_CODE.reset();
    DOCKER_CONTAINER_UPTIME_SECONDS.reset();
    DOCKER_CONTAINER_MEM_LIMIT_BYTES.reset();
    DOCKER_CONTAINER_CPU_SECONDS_TOTAL.reset();
    DOCKER_CONTAINER_MEM_USAGE_BYTES.reset();
    DOCKER_CONTAINER_NET_RECEIVE_BYTES_TOTAL.reset();
    DOCKER_CONTAINER_NET_TRANSMIT_BYTES_TOTAL.reset();
    DOCKER_CONTAINER_BLKIO_READ_BYTES_TOTAL.reset();
    DOCKER_CONTAINER_BLKIO_WRITE_BYTES_TOTAL.reset();

    for sample in samples {
        let labels = [
            sample.container_id.as_str(),
            sample.container_name.as_str(),
            sample.image.as_str(),
        ];

        DOCKER_CONTAINER_STATE
            .with_label_values(&labels)
            .set(sample.state);
        if let Some(hs) = sample.health_status {
            DOCKER_CONTAINER_HEALTH_STATUS
                .with_label_values(&labels)
                .set(hs);
        }
        DOCKER_CONTAINER_LAST_EXIT_CODE
            .with_label_values(&labels)
            .set(sample.last_exit_code);
        DOCKER_CONTAINER_UPTIME_SECONDS
            .with_label_values(&labels)
            .set(sample.uptime_seconds);
        DOCKER_CONTAINER_MEM_LIMIT_BYTES
            .with_label_values(&labels)
            .set(sample.mem_limit_bytes);
        DOCKER_CONTAINER_MEM_USAGE_BYTES
            .with_label_values(&labels)
            .set(sample.mem_usage_bytes);

        DOCKER_CONTAINER_RESTART_COUNT
            .with_label_values(&labels)
            .inc_by(sample.restart_count as f64);
        DOCKER_CONTAINER_CPU_SECONDS_TOTAL
            .with_label_values(&labels)
            .inc_by(sample.cpu_seconds_total);
        DOCKER_CONTAINER_NET_RECEIVE_BYTES_TOTAL
            .with_label_values(&labels)
            .inc_by(sample.net_receive_bytes_total as f64);
        DOCKER_CONTAINER_NET_TRANSMIT_BYTES_TOTAL
            .with_label_values(&labels)
            .inc_by(sample.net_transmit_bytes_total as f64);
        DOCKER_CONTAINER_BLKIO_READ_BYTES_TOTAL
            .with_label_values(&labels)
            .inc_by(sample.blkio_read_bytes_total as f64);
        DOCKER_CONTAINER_BLKIO_WRITE_BYTES_TOTAL
            .with_label_values(&labels)
            .inc_by(sample.blkio_write_bytes_total as f64);
    }
}
