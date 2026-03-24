pub mod health;
pub mod metrics;

use rocket::{Route, routes};

pub fn routes() -> Vec<Route> {
    routes![health::health, metrics::metrics, favicon]
}

#[rocket::get("/favicon.ico")]
pub fn favicon() {}
