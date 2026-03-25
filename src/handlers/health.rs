#[rocket::get("/health")]
pub fn health() -> &'static str {
    "ok"
}
