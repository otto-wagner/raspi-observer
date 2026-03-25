# raspi-observer

`raspi-observer` ist ein schlanker Prometheus-Exporter in Rust.
Er verbindet sich über `bollard` mit dem lokalen Docker-Daemon, liest Container-Infos aus und stellt sie unter `/metrics` bereit.
Zusätzlich werden hardwarenahe Metriken für den Raspberry Pi gesammelt, sowie Standard-Host-Metriken (Node-Exporter Ersatz) aus `/proc`.

## Endpunkte

- `GET /health` -> `ok`
- `GET /metrics` -> Prometheus-Metriken

Standard-Port: `8080` (konfigurierbar per `SERVER_PORT`)

## Exportierte Metriken

Alle Metriken verwenden die Labels:

- `container_id`
- `container_name`
- `image`

### Gauges

- `docker_container_state` - Running state: `1=running`, `0=stopped/exited`
- `docker_container_health_status` - Healthcheck: `1=healthy`, `0=unhealthy`, `2=starting`
- `docker_container_last_exit_code` - letzter Exit-Code
- `docker_container_uptime_seconds` - Sekunden seit Container-Start (`0` wenn gestoppt)
- `docker_container_mem_limit_bytes` - konfiguriertes Memory-Limit in Bytes (`0` = unlimited)
- `docker_container_mem_usage_bytes` - Memory Working Set in Bytes (ohne Cache)

### Counter

- `docker_container_restart_count` - Anzahl Restarts
- `docker_container_cpu_seconds_total` - kumulierte CPU-Zeit in Sekunden
- `docker_container_net_receive_bytes_total` - kumulierte RX-Bytes
- `docker_container_net_transmit_bytes_total` - kumulierte TX-Bytes
- `docker_container_blkio_read_bytes_total` - kumulierte Block-I/O Read-Bytes
- `docker_container_blkio_write_bytes_total` - kumulierte Block-I/O Write-Bytes

### Raspberry Pi Metriken (Gauges)

- `raspi_temperature_celsius` - CPU-Temperatur in Celsius
- `raspi_gpu_temperature_celsius` - GPU-Temperatur in Celsius
- `raspi_thermal_trip_celsius` - Temperatur-Grenzwerte per Type (z.B. aktiv/kritisch)
- `raspi_cpu_clock_hz` - ARM-Taktfrequenz in Hz
- `raspi_core_voltage_volts` - Kernspannung in Volt
- `raspi_throttled` - Throttled-Bitmask der CPU
- `raspi_gpu_memory_mb` - GPU-Speicher in MB
- `raspi_arm_memory_mb` - ARM-Speicher in MB
- `raspi_info` - Hardware-Informationen (Labels: `model`, `revision`, `serial`)
- `raspi_disk_usage_percent` - Root FS Auslastung in Prozent

### Node Metriken

- `node_cpu_seconds_total` - Sekunden der CPUs in jedem Modus (CounterVec)
- `node_memory_bytes` - Speicherinformationen in Bytes (GaugeVec)
- `node_load1`, `node_load5`, `node_load15` - Load Average für 1m, 5m, 15m (Gauges)
- `node_network_receive_bytes_total` - Empfangene Netzwerktraffic pro Device (CounterVec)
- `node_network_transmit_bytes_total` - Gesendeter Netzwerktraffic pro Device (CounterVec)
- `node_boot_time_seconds` - Node Bootzeit in Unix-Time (Gauge)

## Konfiguration

Umgebungsvariablen:

- `SERVER_HOST` (Default: `127.0.0.1`)
- `SERVER_PORT` (Default: `8080`)
- `ENVIRONMENT` (`development` oder `production`, Default: `development`)
- `HOST_PROC` (Pfad zum Proc-Verzeichnis, Default: `/proc`)
- `ENABLE_DOCKER_METRICS` (Default: `true`)
- `ENABLE_RASPI_METRICS` (Default: `true`)
- `ENABLE_NODE_METRICS` (Default: `true`)
- `METRICS_INTERVAL_SEC` (Standard-Intervall für Metriken, Default: `15`)

## Lokal starten

Voraussetzungen:

- Rust Toolchain
- laufender Docker-Daemon
- Zugriff auf Docker-Socket

```zsh
make run
```

## Tech-Stack

- `lazy_static` - globale, lazy initialisierte Singletons für Registry und Metrik-Collector
- `prometheus` ( Feature `process`) - Definition, Registrierung und Export der Prometheus-Metriken
- `rocket` ( Feature `json`) - HTTP-Server für die Endpunkte `/health` und `/metrics`
- `bollard` - Docker API Client für Container-Listing, Inspect und Runtime-Stats
- `chrono` ( default-features = false`, Feature `clock`) - Zeitberechnungen, z. B. Uptime seit Container-Start
- `futures-util` - Stream-Utilities für asynchrones Lesen der Docker-Stats
