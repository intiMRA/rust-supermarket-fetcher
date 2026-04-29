use actix_web::{web, HttpResponse, Responder};
use tokio::sync::Mutex;

use crate::database::Database;
use crate::services::{
    process_shopping_list,
    ShoppingListRequest,
};
use crate::services::paginated_list_service::{get_list_for_page, PaginatedItemRequest};
use crate::services::search_list_service::{perform_search, SearchListRequest};
use crate::services::shopping_list_by_id_service::{find_best_list, process_shopping_list_by_ids, ShoppingListByIDRequest};

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

pub async fn shopping_list_by_ids(
    data: web::Data<AppState>,
    request: web::Json<ShoppingListByIDRequest>,
) -> impl Responder {
    let db = data.db.lock().await;
    let response = process_shopping_list_by_ids(&request, &db);

    HttpResponse::Ok().json(response)
}

pub async fn best_list_by_ids(
    data: web::Data<AppState>,
    request: web::Json<ShoppingListByIDRequest>,
) -> impl Responder {
    let db = data.db.lock().await;
    let response = find_best_list(&request, &db);

    HttpResponse::Ok().json(response)
}

pub async fn paginated_list(
    data: web::Data<AppState>,
    request: web::Json<PaginatedItemRequest>
) -> impl Responder {
    let db = data.db.lock().await;
    let response = get_list_for_page(&request, &db);
    HttpResponse::Ok().json(response)
}

pub async fn search(
    data: web::Data<AppState>,
    request: web::Json<SearchListRequest>
) -> impl Responder {
    let db = data.db.lock().await;
    let response = perform_search(&request, &db);
    HttpResponse::Ok().json(response)
}

/// Health check endpoint
pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "SuperMarketChecker API"
    }))
}
