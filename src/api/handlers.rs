use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::database::Database;
use crate::services::{
    process_shopping_list,
    ShoppingListRequest,
};
use crate::services::paginated_list_service::{get_list_for_page, PaginatedItemRequest};
use crate::services::search_list_service::{perform_search, SearchListRequest};
use crate::services::shopping_list_by_id_service::{find_best_list, process_shopping_list_by_ids, ShoppingListByIDRequest};
use crate::supermarkets::woolworth_fetcher::fetch_woolworths_store_locations;
use crate::utils::geo::haversine_distance_km;

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

// -----------------------------------------------------------------------------
// Woolworths Store Locations
// -----------------------------------------------------------------------------

const MAX_DISTANCE_KM: f64 = 20.0;

#[derive(Deserialize)]
pub struct NearbyStoresQuery {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Serialize)]
pub struct NearbyWoolworthsStore {
    pub id: String,
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    pub distance_km: f64,
}

/// Handler for GET /api/woolworths-stores?latitude=...&longitude=...
///
/// Returns Woolworths stores within 20km of the user's location.
/// This is for showing proximity only — pricing is uniform across all stores.
pub async fn woolworths_stores(
    query: web::Query<NearbyStoresQuery>,
) -> impl Responder {
    let all_stores = match fetch_woolworths_store_locations().await {
        Ok(stores) => stores,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch Woolworths stores: {}", e)
            }));
        }
    };

    let mut nearby: Vec<NearbyWoolworthsStore> = all_stores
        .into_iter()
        .filter_map(|store| {
            let distance = haversine_distance_km(
                query.latitude,
                query.longitude,
                store.latitude,
                store.longitude,
            );
            if distance <= MAX_DISTANCE_KM {
                Some(NearbyWoolworthsStore {
                    id: store.id,
                    name: store.name,
                    address: store.address,
                    latitude: store.latitude,
                    longitude: store.longitude,
                    distance_km: (distance * 100.0).round() / 100.0,
                })
            } else {
                None
            }
        })
        .collect();

    nearby.sort_by(|a, b| a.distance_km.partial_cmp(&b.distance_km).unwrap());

    HttpResponse::Ok().json(nearby)
}
