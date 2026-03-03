use crate::fetchers::fetch_controller::FetchController;

pub mod custom_types;
pub mod fetchers;
pub mod models;
pub mod protocols;

#[tokio::main]
async fn main() {
    let controller = FetchController::new();
    controller.run().await;
}
