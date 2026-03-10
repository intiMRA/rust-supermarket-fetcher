use tokio::sync::Mutex;

use actix_web::{web, App, HttpServer};

use crate::api::handlers::AppState;
use crate::database::{run_sample_queries, Database};
use crate::supermarkets::fetch_controller::FetchController;

pub mod api;
pub mod custom_types;
pub mod database;
pub mod supermarkets;
pub mod matching;
pub mod services;
pub mod utils;
pub mod loggers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "fetch" => {
                let controller = FetchController::new();
                controller.run().await;
                Ok(())
            }
            "query" => {
                let db = Database::open("data/supermarket.db")
                    .expect("Failed to open database. Run 'cargo run -- fetch' first.");
                run_sample_queries(&db);
                Ok(())
            }
            "serve" => {
                run_server().await
            }
            _ => {
                print_usage();
                Ok(())
            }
        }
    } else {
        print_usage();
        Ok(())
    }
}

async fn run_server() -> std::io::Result<()> {
    let db = Database::open("data/supermarket.db")
        .expect("Failed to open database. Run 'cargo run -- fetch' first.");

    let app_state = web::Data::new(AppState {
        db: Mutex::new(db),
    });

    println!("Starting SuperMarket Checker API server...");
    println!("Listening on http://127.0.0.1:8080");
    println!();
    println!("Available endpoints:");
    println!("  POST /api/shopping-list  - Compare prices for a shopping list");
    println!("  GET  /api/health         - Health check");
    println!();
    println!("Example request:");
    println!(r#"  curl -X POST http://127.0.0.1:8080/api/shopping-list \"#);
    println!(r#"    -H "Content-Type: application/json" \"#);
    println!(r#"    -d '{{"items": ["milk", "bread"], "latitude": -36.8485, "longitude": 174.7633}}'"#);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .configure(api::configure_routes)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

fn print_usage() {
    println!("SuperMarket Price Checker");
    println!("=========================");
    println!();
    println!("Usage:");
    println!("  cargo run -- fetch   # Fetch prices from all supermarkets");
    println!("  cargo run -- query   # Run sample database queries");
    println!("  cargo run -- serve   # Start the REST API server");
    println!();
    println!("Database: data/supermarket.db");
}
