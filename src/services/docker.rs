use bollard::Docker;
use bollard::models::ContainerSummary;
use bollard::models::HealthStatusEnum;
use bollard::query_parameters::{ListContainersOptions, StatsOptions};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;

use crate::services::metrics::{DockerContainerMetrics, update_docker_container_metrics};

pub fn connect() -> Result<Docker, bollard::errors::Error> {
    rocket::info!("[DOCKER] Attempting to connect to local Docker daemon...");
    match Docker::connect_with_local_defaults() {
        Ok(docker) => {
            rocket::info!("[DOCKER] Successfully connected to Docker daemon.");
            Ok(docker)
        }
        Err(e) => {
            rocket::error!("[DOCKER] Failed to connect to Docker daemon: {}", e);
            Err(e)
        }
    }
}

fn health_status_to_int(status: Option<HealthStatusEnum>) -> Option<i64> {
    match status {
        Some(HealthStatusEnum::HEALTHY) => Some(1),
        Some(HealthStatusEnum::STARTING) => Some(2),
        Some(HealthStatusEnum::UNHEALTHY) => Some(0),
        _ => None,
    }
}

fn parse_started_at_seconds(started_at: Option<String>, running: bool) -> i64 {
    if !running {
        return 0;
    }

    let Some(ts) = started_at else {
        return 0;
    };

    match DateTime::parse_from_rfc3339(&ts) {
        Ok(parsed) => {
            let now = Utc::now();
            let seconds = now
                .signed_duration_since(parsed.with_timezone(&Utc))
                .num_seconds();
            seconds.max(0)
        }
        Err(_) => 0,
    }
}

fn normalize_container_name(summary: &ContainerSummary) -> String {
    summary
        .names
        .as_ref()
        .and_then(|names| names.first())
        .map(|name| name.trim_start_matches('/').to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn safe_i64_from_u64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

async fn collect_single_container_metrics(
    docker: &Docker,
    container: ContainerSummary,
) -> Result<Option<DockerContainerMetrics>, bollard::errors::Error> {
    let container_id = container.id.clone().unwrap_or_default();
    if container_id.is_empty() {
        rocket::warn!("[DOCKER] Skipping container with empty ID");
        return Ok(None);
    }

    let container_name = normalize_container_name(&container);
    let image = container.image.clone().unwrap_or_default();

    rocket::debug!(
        "[DOCKER] Processing container: {} ({})",
        container_name,
        container_id
    );

    let inspect = match docker.inspect_container(&container_id, None).await {
        Ok(i) => i,
        Err(e) => {
            rocket::error!(
                "[DOCKER] Failed to inspect container {} ({}): {}",
                container_name,
                container_id,
                e
            );
            return Err(e);
        }
    };

    let state = inspect.state;
    let host_config = inspect.host_config;

    let running = state.as_ref().and_then(|s| s.running).unwrap_or(false);

    let state_metric = if running { 1 } else { 0 };
    let health_metric = health_status_to_int(
        state
            .as_ref()
            .and_then(|s| s.health.as_ref())
            .and_then(|h| h.status),
    );
    let restart_count = inspect.restart_count.unwrap_or(0).max(0) as u64;
    let last_exit_code = state.as_ref().and_then(|s| s.exit_code).unwrap_or(0);
    let uptime_seconds =
        parse_started_at_seconds(state.as_ref().and_then(|s| s.started_at.clone()), running);
    let mem_limit_bytes = host_config
        .as_ref()
        .and_then(|h| h.memory)
        .unwrap_or(0)
        .max(0);

    let mut cpu_seconds_total = 0.0;
    let mut mem_usage_bytes = 0_i64;
    let mut net_receive_bytes_total = 0_u64;
    let mut net_transmit_bytes_total = 0_u64;
    let mut blkio_read_bytes_total = 0_u64;
    let mut blkio_write_bytes_total = 0_u64;

    if running {
        let stats_options = StatsOptions {
            stream: false,
            one_shot: true,
        };
        let mut stats_stream = docker.stats(&container_id, Some(stats_options));

        if let Some(stats_result) = stats_stream.next().await {
            let stats = match stats_result {
                Ok(s) => s,
                Err(e) => {
                    rocket::error!(
                        "[DOCKER] Failed to fetch stats for container {} ({}): {}",
                        container_name,
                        container_id,
                        e
                    );
                    return Err(e);
                }
            };

            cpu_seconds_total = stats
                .cpu_stats
                .as_ref()
                .and_then(|cpu| cpu.cpu_usage.as_ref())
                .and_then(|usage| usage.total_usage)
                .map(|nanoseconds| nanoseconds as f64 / 1_000_000_000.0)
                .unwrap_or(0.0);

            if let Some(memory) = stats.memory_stats.as_ref() {
                let usage = memory.usage.unwrap_or(0);
                let cache = memory
                    .stats
                    .as_ref()
                    .and_then(|map| {
                        map.get("inactive_file")
                            .copied()
                            .or_else(|| map.get("cache").copied())
                    })
                    .unwrap_or(0);
                mem_usage_bytes = safe_i64_from_u64(usage.saturating_sub(cache));
            }

            if let Some(networks) = stats.networks.as_ref() {
                for net in networks.values() {
                    net_receive_bytes_total =
                        net_receive_bytes_total.saturating_add(net.rx_bytes.unwrap_or(0));
                    net_transmit_bytes_total =
                        net_transmit_bytes_total.saturating_add(net.tx_bytes.unwrap_or(0));
                }
            }

            if let Some(blkio) = stats.blkio_stats.as_ref()
                && let Some(entries) = blkio.io_service_bytes_recursive.as_ref()
            {
                for entry in entries {
                    let value = entry.value.unwrap_or(0);
                    match entry.op.as_deref() {
                        Some("Read") | Some("read") => {
                            blkio_read_bytes_total = blkio_read_bytes_total.saturating_add(value)
                        }
                        Some("Write") | Some("write") => {
                            blkio_write_bytes_total = blkio_write_bytes_total.saturating_add(value)
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(Some(DockerContainerMetrics {
        container_id,
        container_name,
        image,
        state: state_metric,
        health_status: health_metric,
        restart_count,
        last_exit_code,
        uptime_seconds,
        mem_limit_bytes,
        cpu_seconds_total,
        mem_usage_bytes,
        net_receive_bytes_total,
        net_transmit_bytes_total,
        blkio_read_bytes_total,
        blkio_write_bytes_total,
    }))
}

pub async fn collect_container_metrics(
    docker: &Docker,
) -> Result<Vec<DockerContainerMetrics>, bollard::errors::Error> {
    let list_options = ListContainersOptions {
        all: true,
        filters: None,
        ..Default::default()
    };
    let containers: Vec<ContainerSummary> = docker.list_containers(Some(list_options)).await?;

    let start_time = std::time::Instant::now();
    let total_containers = containers.len();

    rocket::debug!(
        "[DOCKER] Found {} containers, starting metrics collection...",
        total_containers
    );

    // Process containers with limited concurrency to avoid overloading the Docker socket
    let stream =
        futures_util::stream::iter(containers.into_iter().map(|container| async move {
            collect_single_container_metrics(docker, container).await
        }))
        .buffer_unordered(5);

    let results: Vec<_> = stream.collect().await;

    let mut out = Vec::new();
    for result in results {
        match result {
            Ok(Some(metrics)) => out.push(metrics),
            Ok(None) => {} // Skipped container without ID
            Err(e) => {
                rocket::error!("[DOCKER] Failed to fetch stats for a container: {}", e);
            }
        }
    }

    let elapsed = start_time.elapsed();
    rocket::info!(
        "[DOCKER] Collected metrics for {} containers in {:?}",
        total_containers,
        elapsed
    );

    Ok(out)
}

pub async fn run_metrics_collector(docker: Docker, interval_sec: u64) {
    rocket::info!("[DOCKER] Starting Docker metrics collector loop...");
    let mut interval = rocket::tokio::time::interval(std::time::Duration::from_secs(interval_sec));
    interval.set_missed_tick_behavior(rocket::tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        match collect_container_metrics(&docker).await {
            Ok(samples) => update_docker_container_metrics(samples),
            Err(error) => {
                rocket::error!("[DOCKER] Docker metrics collection failed: {}", error);
            }
        }
    }
}
