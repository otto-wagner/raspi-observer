use crate::services::REGISTRY;
use prometheus::{Encoder, TextEncoder};

#[rocket::get("/metrics")]
pub fn metrics() -> Result<String, rocket::http::Status> {
    let metric_families = REGISTRY.gather();
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        rocket::error!("Failed to encode metrics: {}", e);
        return Err(rocket::http::Status::InternalServerError);
    }

    match String::from_utf8(buffer) {
        Ok(body) => Ok(body),
        Err(e) => {
            rocket::error!("Failed to build metrics body: {}", e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}
