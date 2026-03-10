use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::database::{Database, Queries};
use crate::matching::fuzzy_matcher::Product;
use crate::matching::semantic_matcher::find_matching_products_semantic;
use crate::utils::geo::haversine_distance_km;

/// Maximum distance in km for NewWorld and PakNSave stores.
const MAX_DISTANCE_KM: f64 = 20.0;

/// Number of top matches to return per item.
const TOP_N_MATCHES: usize = 3;

/// Number of candidates to fetch from BM25 search.
const BM25_CANDIDATE_LIMIT: usize = 100;

/// Supermarket IDs from the database.
const SUPERMARKET_ID_WOOLWORTH: i32 = 3;

/// Weight for BM25 score in hybrid ranking (0.0 to 1.0).
const BM25_WEIGHT: f64 = 0.4;

/// Weight for semantic score in hybrid ranking.
const SEMANTIC_WEIGHT: f64 = 0.2;

/// Weight for price score in hybrid ranking.
const PRICE_WEIGHT: f64 = 0.4;

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

/// Process a shopping list request using hybrid BM25 + semantic matching.
///
/// Strategy:
/// 1. BM25 (keyword search): Fast, handles exact matches well ("milk" → "Fresh Milk")
/// 2. Semantic (embeddings): Understands meaning ("butter" → "Anchor Butter")
/// 3. Combined scoring: BM25 (40%) + Semantic (20%) + Price (40%)
pub fn process_shopping_list(
    request: &ShoppingListRequest,
    db: &Database,
) -> ShoppingListResponse {
    let queries = Queries::new(db);

    // Step 1: Find nearby stores
    let nearby_stores = find_stores_to_query(
        &queries,
        request.latitude,
        request.longitude,
    );

    if nearby_stores.is_empty() {
        return ShoppingListResponse {
            items: request.items.iter().map(|term| ShoppingListItem {
                search_term: term.clone(),
                top_matches: Vec::new(),
            }).collect(),
        };
    }

    let store_ids: Vec<String> = nearby_stores.iter().map(|s| s.id.clone()).collect();
    let store_distance_map: HashMap<String, f64> = nearby_stores
        .iter()
        .map(|s| (s.id.clone(), s.distance_km))
        .collect();
    let store_name_map: HashMap<String, String> = nearby_stores
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    // Step 2: Process each item using hybrid search
    let items: Vec<ShoppingListItem> = request
        .items
        .iter()
        .map(|search_term| {
            process_single_item(
                search_term,
                &queries,
                &store_ids,
                &store_distance_map,
                &store_name_map,
            )
        })
        .collect();

    ShoppingListResponse { items }
}

/// Process a single shopping list item using category-first, then BM25.
fn process_single_item(
    search_term: &str,
    queries: &Queries<'_>,
    store_ids: &[String],
    store_distance_map: &HashMap<String, f64>,
    store_name_map: &HashMap<String, String>,
) -> ShoppingListItem {
    // Strategy: Category-first for generic terms, BM25 for specific products
    // "milk" → Fresh Milk category (not "chocolate milk bars")
    // "anchor milk 2l" → BM25 search (specific product)

    let category_ids = queries.find_matching_category_ids(search_term);

    let candidates = if !category_ids.is_empty() {
        // Found matching category - search within it
        let category_products = queries.search_products_in_categories_and_stores(
            &category_ids,
            store_ids,
        );

        if !category_products.is_empty() {
            // Convert to Product format with default BM25 score
            category_products
                .into_iter()
                .map(|p| Product {
                    product_name: p.product_name,
                    brand: p.brand,
                    price: p.price,
                    supermarket: p.supermarket,
                    store_name: p.store_name,
                    store_id: p.store_id,
                    store_latitude: p.store_latitude,
                    store_longitude: p.store_longitude,
                    similarity_score: 0.7, // Good base score for category match
                })
                .collect::<Vec<_>>()
        } else {
            // Category exists but no products in nearby stores, try BM25
            get_bm25_candidates(search_term, queries, store_ids)
        }
    } else {
        // No category match - use BM25 for specific product search
        get_bm25_candidates(search_term, queries, store_ids)
    };

    if candidates.is_empty() {
        return ShoppingListItem {
            search_term: search_term.to_string(),
            top_matches: Vec::new(),
        };
    }

    // Step 2: Apply semantic matching to get semantic scores
    let semantic_matches = find_matching_products_semantic(search_term, &candidates, 0.0);

    // Create lookup map for semantic scores
    let semantic_scores: HashMap<String, f64> = semantic_matches
        .into_iter()
        .map(|p| {
            let key = format!("{}|{}|{}", p.product_name, p.store_id, p.price);
            (key, p.similarity_score)
        })
        .collect();

    // Step 3: Calculate hybrid scores and sort
    let max_price = candidates
        .iter()
        .map(|p| p.price)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let mut scored_products: Vec<(Product, f64)> = candidates
        .into_iter()
        .map(|p| {
            let key = format!("{}|{}|{}", p.product_name, p.store_id, p.price);
            let semantic_score = semantic_scores.get(&key).copied().unwrap_or(0.0);
            let bm25_score = p.similarity_score; // We stored BM25 here earlier
            let price_score = 1.0 - (p.price / max_price);

            let hybrid_score = (bm25_score * BM25_WEIGHT)
                + (semantic_score * SEMANTIC_WEIGHT)
                + (price_score * PRICE_WEIGHT);

            (p, hybrid_score)
        })
        .collect();

    // Sort by hybrid score descending
    scored_products.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Step 4: Convert to response format
    let mut seen_products: HashSet<String> = HashSet::new();
    let top_matches: Vec<MatchedProduct> = scored_products
        .into_iter()
        .filter_map(|(p, score)| {
            let key = p.product_name.to_lowercase();
            if seen_products.contains(&key) {
                return None;
            }
            seen_products.insert(key);

            let distance = store_distance_map.get(&p.store_id).copied().unwrap_or(0.0);
            let store_name = store_name_map.get(&p.store_id).cloned().unwrap_or(p.store_name);

            Some(MatchedProduct {
                product_name: p.product_name,
                brand: p.brand,
                price: p.price,
                supermarket: p.supermarket,
                store_name,
                distance_km: (distance * 10.0).round() / 10.0,
                similarity_score: (score * 100.0).round() / 100.0,
            })
        })
        .take(TOP_N_MATCHES)
        .collect();

    // Final fallback: if no results after category search, try BM25 without category
    if top_matches.is_empty() {
        let bm25_candidates = get_bm25_candidates(search_term, queries, store_ids);
        if !bm25_candidates.is_empty() {
            return process_candidates_to_matches(
                search_term,
                bm25_candidates,
                store_distance_map,
                store_name_map,
            );
        }
    }

    ShoppingListItem {
        search_term: search_term.to_string(),
        top_matches,
    }
}

/// Convert candidates to final matches with scoring.
fn process_candidates_to_matches(
    search_term: &str,
    candidates: Vec<Product>,
    store_distance_map: &HashMap<String, f64>,
    store_name_map: &HashMap<String, String>,
) -> ShoppingListItem {
    if candidates.is_empty() {
        return ShoppingListItem {
            search_term: search_term.to_string(),
            top_matches: Vec::new(),
        };
    }

    let semantic_matches = find_matching_products_semantic(search_term, &candidates, 0.0);
    let semantic_scores: HashMap<String, f64> = semantic_matches
        .iter()
        .map(|p| {
            let key = format!("{}|{}|{}", p.product_name, p.store_id, p.price);
            (key, p.similarity_score)
        })
        .collect();

    let max_price = candidates.iter().map(|p| p.price).fold(0.0_f64, f64::max).max(1.0);

    let mut scored_products: Vec<(Product, f64)> = candidates
        .into_iter()
        .map(|p| {
            let key = format!("{}|{}|{}", p.product_name, p.store_id, p.price);
            let semantic_score = semantic_scores.get(&key).copied().unwrap_or(0.0);
            let bm25_score = p.similarity_score;
            let price_score = 1.0 - (p.price / max_price);
            let hybrid_score = (bm25_score * BM25_WEIGHT)
                + (semantic_score * SEMANTIC_WEIGHT)
                + (price_score * PRICE_WEIGHT);
            (p, hybrid_score)
        })
        .collect();

    scored_products.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let mut seen_products: HashSet<String> = HashSet::new();
    let top_matches: Vec<MatchedProduct> = scored_products
        .into_iter()
        .filter_map(|(p, score)| {
            let key = p.product_name.to_lowercase();
            if seen_products.contains(&key) {
                return None;
            }
            seen_products.insert(key);

            let distance = store_distance_map.get(&p.store_id).copied().unwrap_or(0.0);
            let store_name = store_name_map.get(&p.store_id).cloned().unwrap_or(p.store_name);

            Some(MatchedProduct {
                product_name: p.product_name,
                brand: p.brand,
                price: p.price,
                supermarket: p.supermarket,
                store_name,
                distance_km: (distance * 10.0).round() / 10.0,
                similarity_score: (score * 100.0).round() / 100.0,
            })
        })
        .take(TOP_N_MATCHES)
        .collect();

    ShoppingListItem {
        search_term: search_term.to_string(),
        top_matches,
    }
}

/// Get candidates using BM25 full-text search.
fn get_bm25_candidates(
    search_term: &str,
    queries: &Queries<'_>,
    store_ids: &[String],
) -> Vec<Product> {
    let bm25_results = queries.search_products_bm25(search_term, store_ids, BM25_CANDIDATE_LIMIT);

    if bm25_results.is_empty() {
        // BM25 returned nothing, try LIKE-based text search
        let db_products = queries.search_products_in_stores(search_term, store_ids);
        return db_products
            .into_iter()
            .map(|p| Product {
                product_name: p.product_name,
                brand: p.brand,
                price: p.price,
                supermarket: p.supermarket,
                store_name: p.store_name,
                store_id: p.store_id,
                store_latitude: p.store_latitude,
                store_longitude: p.store_longitude,
                similarity_score: 0.5,
            })
            .collect();
    }

    // Normalize BM25 scores to 0-1 range
    let min_bm25 = bm25_results.iter().map(|r| r.bm25_score).fold(f64::INFINITY, f64::min);
    let max_bm25 = bm25_results.iter().map(|r| r.bm25_score).fold(f64::NEG_INFINITY, f64::max);
    let bm25_range = (max_bm25 - min_bm25).abs().max(0.001);

    bm25_results
        .into_iter()
        .map(|r| {
            let normalized_bm25 = 1.0 - ((r.bm25_score - min_bm25) / bm25_range);
            Product {
                product_name: r.product_name,
                brand: r.brand,
                price: r.price,
                supermarket: r.supermarket,
                store_name: r.store_name,
                store_id: r.store_id,
                store_latitude: r.store_latitude,
                store_longitude: r.store_longitude,
                similarity_score: normalized_bm25,
            }
        })
        .collect()
}

/// Find stores to query for products.
fn find_stores_to_query(
    queries: &Queries<'_>,
    user_lat: f64,
    user_lon: f64,
) -> Vec<NearbyStore> {
    let mut stores_to_query = Vec::new();
    let db_stores = queries.get_all_stores();

    for store in db_stores {
        if store.supermarket_id == SUPERMARKET_ID_WOOLWORTH {
            stores_to_query.push(NearbyStore {
                id: store.id,
                name: store.name,
                distance_km: 0.0,
            });
        } else {
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
