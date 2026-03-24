use crate::services::metrics::REGISTRY;
use lazy_static::lazy_static;
use prometheus::{Gauge, GaugeVec, Opts};
use std::fs;
use std::process::Command;
use std::time::Duration;

lazy_static! {
    pub static ref RASPI_TEMPERATURE: Gauge =
        Gauge::new("raspi_temperature_celsius", "CPU-Temperatur in Celsius").unwrap();
    pub static ref RASPI_GPU_TEMPERATURE: Gauge =
        Gauge::new("raspi_gpu_temperature_celsius", "GPU-Temperatur in Celsius").unwrap();
    pub static ref RASPI_THERMAL_TRIP: GaugeVec = GaugeVec::new(
        Opts::new("raspi_thermal_trip_celsius", "Temperatur-Grenzwerte"),
        &["type"]
    )
    .unwrap();
    pub static ref RASPI_CPU_CLOCK: Gauge =
        Gauge::new("raspi_cpu_clock_hz", "ARM-Taktfrequenz in Hz").unwrap();
    pub static ref RASPI_CORE_VOLTAGE: Gauge =
        Gauge::new("raspi_core_voltage_volts", "Kernspannung in Volt").unwrap();
    pub static ref RASPI_THROTTLED: Gauge =
        Gauge::new("raspi_throttled", "Throttled-Bitmask").unwrap();
    pub static ref RASPI_GPU_MEMORY: Gauge =
        Gauge::new("raspi_gpu_memory_mb", "GPU-Speicher in MB").unwrap();
    pub static ref RASPI_ARM_MEMORY: Gauge =
        Gauge::new("raspi_arm_memory_mb", "ARM-Speicher in MB").unwrap();
    pub static ref RASPI_INFO: GaugeVec = GaugeVec::new(
        Opts::new("raspi_info", "Hardware-Informationen"),
        &["model", "revision", "serial"]
    )
    .unwrap();
    pub static ref RASPI_DISK_USAGE: Gauge =
        Gauge::new("raspi_disk_usage_percent", "Root FS usage percent").unwrap();
}

pub fn init_metrics() {
    let _ = REGISTRY.register(Box::new(RASPI_TEMPERATURE.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_GPU_TEMPERATURE.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_THERMAL_TRIP.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_CPU_CLOCK.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_CORE_VOLTAGE.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_THROTTLED.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_GPU_MEMORY.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_ARM_MEMORY.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_INFO.clone()));
    let _ = REGISTRY.register(Box::new(RASPI_DISK_USAGE.clone()));
}

fn vcgencmd(args: &[&str]) -> Option<String> {
    let out = Command::new("/opt/vc/bin/vcgencmd")
        .args(args)
        .output()
        .or_else(|_| Command::new("vcgencmd").args(args).output())
        .or_else(|_| {
            Command::new("chroot")
                .arg("/host")
                .arg("vcgencmd")
                .args(args)
                .output()
        })
        .or_else(|_| Command::new("/host/usr/bin/vcgencmd").args(args).output());

    match out {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        _ => None,
    }
}

pub async fn run_metrics_collector(interval_sec: u64) {
    rocket::info!("[RASPI] Starting Raspberry Pi metrics collector...");
    let mut interval = rocket::tokio::time::interval(Duration::from_secs(interval_sec));
    loop {
        interval.tick().await;

        match fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
            Ok(temp_str) => {
                if let Ok(t) = temp_str.trim().parse::<f64>() {
                    let temp_c = t / 1000.0;
                    RASPI_TEMPERATURE.set(temp_c);
                } else {
                    rocket::warn!("[RASPI] Error parsing temperature: {}", temp_str.trim());
                }
            }
            Err(e) => {
                rocket::warn!(
                    "[RASPI] Failed to read /sys/class/thermal/thermal_zone0/temp: {}",
                    e
                );
            }
        }

        match vcgencmd(&["measure_temp"]) {
            Some(gpu_temp) => {
                if let Some(temp_str) = gpu_temp
                    .strip_prefix("temp=")
                    .and_then(|s| s.strip_suffix("'C"))
                {
                    if let Ok(t) = temp_str.parse::<f64>() {
                        RASPI_GPU_TEMPERATURE.set(t);
                    } else {
                        rocket::warn!("[RASPI] Error parsing GPU temperature: {}", temp_str);
                    }
                } else {
                    rocket::warn!("[RASPI] Unexpected GPU temp format: {}", gpu_temp);
                }
            }
            None => {
                rocket::warn!("[RASPI] vcgencmd measure_temp failed");
            }
        }

        match vcgencmd(&["measure_clock", "arm"]) {
            Some(clock) => {
                if let Some(val_str) = clock.split('=').nth(1) {
                    if let Ok(hz) = val_str.parse::<f64>() {
                        RASPI_CPU_CLOCK.set(hz);
                    } else {
                        rocket::warn!("[RASPI] Error parsing CPU clock: {}", val_str);
                    }
                } else {
                    rocket::warn!("[RASPI] Unexpected CPU clock format: {}", clock);
                }
            }
            None => {
                rocket::warn!("[RASPI] vcgencmd measure_clock arm failed");
            }
        }

        match vcgencmd(&["measure_volts", "core"]) {
            Some(volts) => {
                if let Some(val_str) = volts
                    .strip_prefix("volt=")
                    .and_then(|s| s.strip_suffix("V"))
                {
                    if let Ok(v) = val_str.parse::<f64>() {
                        RASPI_CORE_VOLTAGE.set(v);
                    } else {
                        rocket::warn!("[RASPI] Error parsing core volts: {}", val_str);
                    }
                } else {
                    rocket::warn!("[RASPI] Unexpected core volts format: {}", volts);
                }
            }
            None => {
                rocket::warn!("[RASPI] vcgencmd measure_volts core failed");
            }
        }

        let out = Command::new("df")
            .arg("/host")
            .output()
            .or_else(|_| Command::new("df").arg("/").output());
        match out {
            Ok(output) => {
                let s = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = s.lines().collect();
                if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 5 {
                        if let Ok(p) = parts[4].replace("%", "").parse::<f64>() {
                            RASPI_DISK_USAGE.set(p);
                        } else {
                            rocket::warn!("[RASPI] Error parsing disk usage: {}", parts[4]);
                        }
                    } else {
                        rocket::warn!("[RASPI] Unexpected df output parts: {:?}", parts);
                    }
                } else {
                    rocket::warn!("[RASPI] Unexpected df output format: \n{}", s);
                }
            }
            Err(e) => {
                rocket::warn!("[RASPI] df command failed: {}", e);
            }
        }
    }
}
