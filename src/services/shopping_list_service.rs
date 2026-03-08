use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::database::{Database, Queries};
use crate::matching::fuzzy_matcher::find_matching_products;
use crate::utils::geo::haversine_distance_km;

/// Default similarity threshold for fuzzy matching (60%).
const DEFAULT_SIMILARITY_THRESHOLD: f64 = 0.6;

/// Maximum distance in km for NewWorld and PakNSave stores.
const MAX_DISTANCE_KM: f64 = 20.0;

/// Number of top matches to return per item.
const TOP_N_MATCHES: usize = 3;

/// Supermarket IDs from the database.
const SUPERMARKET_ID_WOOLWORTH: i32 = 3;

/// Request payload for shopping list processing.
#[derive(Debug, Deserialize)]
pub struct ShoppingListRequest {
    pub items: Vec<String>,
    pub latitude: f64,
    pub longitude: f64,
}

/// A matched product in the response.
#[derive(Debug, Serialize)]
pub struct MatchedProduct {
    pub product_name: String,
    pub brand: String,
    pub price: f64,
    pub supermarket: String,
    pub store_name: String,
    pub distance_km: f64,
    pub similarity_score: f64,
}

/// A single item from the shopping list with its matches.
#[derive(Debug, Serialize)]
pub struct ShoppingListItem {
    pub search_term: String,
    pub top_matches: Vec<MatchedProduct>,
}

/// Response payload for shopping list processing.
#[derive(Debug, Serialize)]
pub struct ShoppingListResponse {
    pub items: Vec<ShoppingListItem>,
}

/// Information about a store to query for products.
#[derive(Clone)]
struct NearbyStore {
    id: String,
    name: String,
    distance_km: f64,
}

/// Process a shopping list request and find the cheapest matches.
///
/// This function:
/// 1. Finds nearby stores from DB (NewWorld/PakNSave filtered by 20km, Woolworths always included)
/// 2. Only queries products from those stores
/// 3. Returns top 3 cheapest matches per item
pub fn process_shopping_list(
    request: &ShoppingListRequest,
    db: &Database,
) -> ShoppingListResponse {
    let queries = Queries::new(db);

    // Step 1: Find stores to query
    // - NewWorld and PakNSave: filter by 20km radius
    // - Woolworths: always include (uniform pricing, no physical store filtering)
    let nearby_stores = find_stores_to_query(
        &queries,
        request.latitude,
        request.longitude,
    );

    // If no stores found, return empty results
    if nearby_stores.is_empty() {
        return ShoppingListResponse {
            items: request.items.iter().map(|term| ShoppingListItem {
                search_term: term.clone(),
                top_matches: Vec::new(),
            }).collect(),
        };
    }

    // Collect store IDs and create distance lookup map
    let store_ids: Vec<String> = nearby_stores.iter().map(|s| s.id.clone()).collect();
    let store_distance_map: HashMap<String, f64> = nearby_stores
        .iter()
        .map(|s| (s.id.clone(), s.distance_km))
        .collect();
    let store_name_map: HashMap<String, String> = nearby_stores
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    // Step 2: Process each item in the shopping list
    let items: Vec<ShoppingListItem> = request
        .items
        .iter()
        .map(|search_term| {
            // Query products only from selected stores
            let db_products = queries.search_products_in_stores(search_term, &store_ids);

            // Convert to fuzzy matcher format
            let products_for_matching: Vec<(String, String, f64, String, String, String, f64, f64)> =
                db_products
                    .iter()
                    .map(|p| {
                        (
                            p.product_name.clone(),
                            p.brand.clone(),
                            p.price,
                            p.supermarket.clone(),
                            p.store_name.clone(),
                            p.store_id.clone(),
                            p.store_latitude,
                            p.store_longitude,
                        )
                    })
                    .collect();

            // Find matching products using fuzzy matching
            let matches = find_matching_products(
                search_term,
                &products_for_matching,
                DEFAULT_SIMILARITY_THRESHOLD,
            );

            // Convert to response format with pre-calculated distances
            let mut filtered_matches: Vec<MatchedProduct> = matches
                .into_iter()
                .map(|m| {
                    let distance = store_distance_map
                        .get(&m.store_id)
                        .copied()
                        .unwrap_or(0.0);
                    let store_name = store_name_map
                        .get(&m.store_id)
                        .cloned()
                        .unwrap_or(m.store_name);

                    MatchedProduct {
                        product_name: m.product_name,
                        brand: m.brand,
                        price: m.price,
                        supermarket: m.supermarket,
                        store_name,
                        distance_km: (distance * 10.0).round() / 10.0,
                        similarity_score: (m.similarity_score * 100.0).round() / 100.0,
                    }
                })
                .collect();

            // Sort by price ascending
            filtered_matches.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

            // Deduplicate by product name, keeping the cheapest option per product
            let mut seen_products: HashSet<String> = HashSet::new();
            let unique_matches: Vec<MatchedProduct> = filtered_matches
                .into_iter()
                .filter(|m| {
                    let key = m.product_name.to_lowercase();
                    if seen_products.contains(&key) {
                        false
                    } else {
                        seen_products.insert(key);
                        true
                    }
                })
                .take(TOP_N_MATCHES)
                .collect();

            ShoppingListItem {
                search_term: search_term.clone(),
                top_matches: unique_matches,
            }
        })
        .collect();

    ShoppingListResponse { items }
}

/// Find stores to query for products.
///
/// - NewWorld and PakNSave: filtered by 20km radius from user location
/// - Woolworths: always included (uses "default" store, uniform pricing nationwide)
fn find_stores_to_query(
    queries: &Queries<'_>,
    user_lat: f64,
    user_lon: f64,
) -> Vec<NearbyStore> {
    let mut stores_to_query = Vec::new();

    // Get all stores from database
    let db_stores = queries.get_all_stores();

    for store in db_stores {
        if store.supermarket_id == SUPERMARKET_ID_WOOLWORTH {
            // Woolworths: always include (uniform pricing, no distance filter)
            stores_to_query.push(NearbyStore {
                id: store.id,
                name: store.name,
                distance_km: 0.0, // No distance concept for Woolworths
            });
        } else {
            // NewWorld and PakNSave: filter by distance
            let distance = haversine_distance_km(
                user_lat,
                user_lon,
                store.latitude,
                store.longitude,
            );

            if distance <= MAX_DISTANCE_KM {
                stores_to_query.push(NearbyStore {
                    id: store.id,
                    name: store.name,
                    distance_km: distance,
                });
            }
        }
    }

    stores_to_query
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_deserialization() {
        let json = r#"{
            "items": ["milk", "bread"],
            "latitude": -36.8485,
            "longitude": 174.7633
        }"#;

        let request: ShoppingListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.items.len(), 2);
        assert_eq!(request.items[0], "milk");
        assert!(request.latitude < 0.0);
    }
}
