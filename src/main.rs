use raspi_observer::build_app;
use rocket::launch;

#[launch]
fn rocket() -> _ {
    build_app()
}
