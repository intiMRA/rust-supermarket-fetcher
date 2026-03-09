use actix_web::{web, HttpResponse, Responder};
use tokio::sync::Mutex;

use crate::database::Database;
use crate::services::{
    process_shopping_list,
    ShoppingListRequest,
};

/// Application state shared across handlers.
pub struct AppState {
    pub db: Mutex<Database>,
}

/// Handler for POST /api/shopping-list
///
/// Accepts a shopping list with items and user location, returns the top 3
/// cheapest options for each item across supermarkets.
///
/// NewWorld and PakNSave are filtered by distance (20km radius).
/// Woolworths items are always included (uniform pricing across all stores).
pub async fn shopping_list(
    data: web::Data<AppState>,
    request: web::Json<ShoppingListRequest>,
) -> impl Responder {
    let db = data.db.lock().await;
    let response = process_shopping_list(&request, &db);

    HttpResponse::Ok().json(response)
}

/// Health check endpoint
pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "SuperMarketChecker API"
    }))
}
