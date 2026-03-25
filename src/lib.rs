pub mod config;
pub mod handlers;
pub mod services;

use config::Config;
use rocket::fairing::AdHoc;
use rocket::{Build, Rocket};
use services::docker;

pub fn build_app() -> Rocket<Build> {
    let config = Config::new();

    if config.enable_docker_metrics {
        services::init_metrics();
    }
    if config.enable_raspi_metrics {
        services::raspi::init_metrics();
    }
    if config.enable_node_metrics {
        services::node::init_metrics();
    }

    let docker_client = docker::connect().expect("Failed to connect to Docker daemon");

    rocket::build()
        .manage(config.clone())
        .configure(
            rocket::Config::figment()
                .merge(("port", config.port))
                .merge(("address", config.host.clone())),
        )
        .manage(docker_client)
        .attach(AdHoc::on_liftoff("Metrics Collector Task", |rocket| {
            Box::pin(async move {
                let config = rocket.state::<Config>().cloned().unwrap_or_default();

                if config.enable_raspi_metrics {
                    let interval = config.metrics_interval_sec;
                    rocket::tokio::spawn(async move {
                        services::raspi::run_metrics_collector(interval).await;
                    });
                }

                if config.enable_node_metrics {
                    let interval = config.metrics_interval_sec;
                    let proc_path = config.host_proc.clone();
                    rocket::tokio::spawn(async move {
                        services::node::run_node_metrics_collector(interval, proc_path).await;
                    });
                }

                if config.enable_docker_metrics {
                    let Some(docker) = rocket.state::<bollard::Docker>().cloned() else {
                        rocket::error!("Docker state not found, metrics collector not started");
                        return;
                    };
                    let interval = config.metrics_interval_sec;

                    rocket::tokio::spawn(async move {
                        docker::run_metrics_collector(docker, interval).await;
                    });
                }
            })
        }))
        .mount("/", handlers::routes())
}
