use crate::services::metrics::REGISTRY;
use lazy_static::lazy_static;
use prometheus::{CounterVec, Gauge, GaugeVec, Opts};
use std::fs;
use std::time::Duration;

lazy_static! {
    pub static ref NODE_CPU_SECONDS_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "node_cpu_seconds_total",
            "Seconds the CPUs spent in each mode"
        ),
        &["cpu", "mode"]
    )
    .unwrap();
    pub static ref NODE_MEMORY_BYTES: GaugeVec = GaugeVec::new(
        Opts::new("node_memory_bytes", "Memory information in bytes"),
        &["type"]
    )
    .unwrap();
    pub static ref NODE_LOAD1: Gauge = Gauge::new("node_load1", "1m load average").unwrap();
    pub static ref NODE_LOAD5: Gauge = Gauge::new("node_load5", "5m load average").unwrap();
    pub static ref NODE_LOAD15: Gauge = Gauge::new("node_load15", "15m load average").unwrap();
    pub static ref NODE_NETWORK_RECEIVE_BYTES_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "node_network_receive_bytes_total",
            "Network device statistic receive_bytes"
        ),
        &["device"]
    )
    .unwrap();
    pub static ref NODE_NETWORK_TRANSMIT_BYTES_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "node_network_transmit_bytes_total",
            "Network device statistic transmit_bytes"
        ),
        &["device"]
    )
    .unwrap();
    pub static ref NODE_BOOT_TIME_SECONDS: Gauge =
        Gauge::new("node_boot_time_seconds", "Node boot time, in unixtime.").unwrap();
}

pub fn init_metrics() {
    let _ = REGISTRY.register(Box::new(NODE_CPU_SECONDS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(NODE_MEMORY_BYTES.clone()));
    let _ = REGISTRY.register(Box::new(NODE_LOAD1.clone()));
    let _ = REGISTRY.register(Box::new(NODE_LOAD5.clone()));
    let _ = REGISTRY.register(Box::new(NODE_LOAD15.clone()));
    let _ = REGISTRY.register(Box::new(NODE_NETWORK_RECEIVE_BYTES_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(NODE_NETWORK_TRANSMIT_BYTES_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(NODE_BOOT_TIME_SECONDS.clone()));
}

fn u64_diff_to_add(current_val: u64, prev_val: u64) -> f64 {
    if current_val >= prev_val {
        (current_val - prev_val) as f64
    } else {
        current_val as f64
    }
}

pub async fn run_node_metrics_collector(interval_sec: u64, proc_path: String) {
    rocket::info!(
        "[NODE] Starting node metrics collector for path: {}...",
        proc_path
    );
    let mut interval = rocket::tokio::time::interval(Duration::from_secs(interval_sec));

    let mut net_rx_prev: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut net_tx_prev: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut cpu_prev: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    let clock_ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;

    loop {
        interval.tick().await;

        // 1. /proc/loadavg
        if let Ok(content) = fs::read_to_string(format!("{}/loadavg", proc_path)) {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Ok(l1) = parts[0].parse::<f64>() {
                    NODE_LOAD1.set(l1);
                }
                if let Ok(l5) = parts[1].parse::<f64>() {
                    NODE_LOAD5.set(l5);
                }
                if let Ok(l15) = parts[2].parse::<f64>() {
                    NODE_LOAD15.set(l15);
                }
            }
        }

        // 2. /proc/meminfo
        if let Ok(content) = fs::read_to_string(format!("{}/meminfo", proc_path)) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let key = parts[0].replace(":", "");
                    if let Ok(mut val) = parts[1].parse::<f64>() {
                        if parts.len() == 3 && parts[2] == "kB" {
                            val *= 1024.0;
                        }
                        NODE_MEMORY_BYTES.with_label_values(&[&key]).set(val);
                    }
                }
            }
        }

        // 3. /proc/net/dev
        if let Ok(content) = fs::read_to_string(format!("{}/net/dev", proc_path)) {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 17 {
                    let device = parts[0].replace(":", "");
                    if let Ok(rx) = parts[1].parse::<u64>() {
                        let prev = *net_rx_prev.get(&device).unwrap_or(&rx);
                        NODE_NETWORK_RECEIVE_BYTES_TOTAL
                            .with_label_values(&[&device])
                            .inc_by(u64_diff_to_add(rx, prev));
                        net_rx_prev.insert(device.clone(), rx);
                    }
                    if let Ok(tx) = parts[9].parse::<u64>() {
                        let prev = *net_tx_prev.get(&device).unwrap_or(&tx);
                        NODE_NETWORK_TRANSMIT_BYTES_TOTAL
                            .with_label_values(&[&device])
                            .inc_by(u64_diff_to_add(tx, prev));
                        net_tx_prev.insert(device.clone(), tx);
                    }
                }
            }
        }

        // 4. /proc/stat (CPU and boot time)
        if let Ok(content) = fs::read_to_string(format!("{}/stat", proc_path)) {
            let modes = [
                "user",
                "nice",
                "system",
                "idle",
                "iowait",
                "irq",
                "softirq",
                "steal",
                "guest",
                "guest_nice",
            ];
            for line in content.lines() {
                if line.starts_with("btime ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2
                        && let Ok(btime) = parts[1].parse::<f64>()
                    {
                        NODE_BOOT_TIME_SECONDS.set(btime);
                    }
                } else if line.starts_with("cpu") && !line.starts_with("cpu ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let cpu_name = parts[0];
                    for (i, &mode) in modes.iter().enumerate() {
                        if parts.len() > i + 1
                            && let Ok(val) = parts[i + 1].parse::<u64>()
                        {
                            let key = format!("{}_{}", cpu_name, mode);
                            let prev = *cpu_prev.get(&key).unwrap_or(&val);
                            let diff_ticks = u64_diff_to_add(val, prev);
                            let diff_secs = diff_ticks / clock_ticks;
                            NODE_CPU_SECONDS_TOTAL
                                .with_label_values(&[cpu_name, mode])
                                .inc_by(diff_secs);
                            cpu_prev.insert(key, val);
                        }
                    }
                }
            }
        }
    }
}
