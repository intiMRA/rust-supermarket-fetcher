use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::database::{Database, Queries};
use crate::matching::semantic_matcher::{find_matching_products_semantic, Product};
use crate::services::common_models::response_product::MatchedProduct;
pub(crate) use crate::services::common_models::nearby_store::NearbyStore;
use crate::services::common_models::shopping_list_item::ShoppingListItem;
use crate::services::utils::common_logic;

/// Number of top matches to return per item.
const TOP_N_MATCHES: usize = 5;

/// Number of candidates to fetch from BM25 search.
const BM25_CANDIDATE_LIMIT: usize = 100;

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

/// Price info from a specific supermarket/store.
#[derive(Debug, Serialize, Clone)]
pub struct SupermarketInfo {
    pub supermarket: String,
    pub store_name: String,
    pub distance_km: f64,
    pub price: f64,
}

/// Response payload for shopping list processing.
#[derive(Debug, Serialize)]
pub struct ShoppingListResponse {
    pub items: Vec<ShoppingListItem>,
}

/// Process a shopping list request using hybrid BM25 + semantic matching.
///
/// Strategy:
/// 1. BM25 (keyword search): Fast, handles exact matches well ("milk" → "Fresh Milk")
/// 2. Semantic (embeddings): Understands meaning ("butter" → "Anchor Butter")
/// 3. Combined scoring: BM25 (40%) + Semantic (20%) + Price (40%)
/// 4. Group by product: Return deduplicated products with prices from all stores
pub fn process_shopping_list(
    request: &ShoppingListRequest,
    db: &Database,
) -> ShoppingListResponse {
    let queries = Queries::new(db);

    // Step 1: Find nearby stores
    let nearby_stores =  common_logic::find_stores_to_query(
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
    let store_map: HashMap<String, &NearbyStore> = nearby_stores
        .iter()
        .map(|s| (s.id.clone(), s))
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
                &store_map,
            )
        })
        .collect();

    ShoppingListResponse { items }
}

/// Process a single shopping list item.
fn process_single_item(
    search_term: &str,
    queries: &Queries<'_>,
    store_ids: &[String],
    store_map: &HashMap<String, &NearbyStore>,
) -> ShoppingListItem {
    // Get candidates from both category search and BM25, then combine
    let mut candidates: Vec<Product> = Vec::new();
    let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

    // First, try category-based search
    let category_ids = queries.find_matching_category_ids(search_term);
    if !category_ids.is_empty() {
        let category_products = queries.search_products_in_categories_and_stores(
            &category_ids,
            store_ids,
        );

        for p in category_products {
            let key = format!("{}|{}|{}", p.product_name, p.store_id, p.price);
            if seen_keys.insert(key) {
                candidates.push(Product {
                    product_name: p.product_name,
                    brand: p.brand,
                    size_value: p.size_value,
                    size_unit: p.size_unit,
                    price: p.price,
                    supermarket: p.supermarket,
                    store_name: p.store_name,
                    store_id: p.store_id,
                    store_latitude: p.store_latitude,
                    store_longitude: p.store_longitude,
                    similarity_score: 0.7,
                });
            }
        }
    }

    // Always also get BM25 candidates to ensure we have enough results
    let bm25_candidates = get_bm25_candidates(search_term, queries, store_ids);
    for p in bm25_candidates {
        let key = format!("{}|{}|{}", p.product_name, p.store_id, p.price);
        if seen_keys.insert(key) {
            candidates.push(p);
        }
    }

    if candidates.is_empty() {
        return ShoppingListItem {
            search_term: search_term.to_string(),
            top_matches: Vec::new(),
        };
    }

    // Apply semantic matching
    let semantic_matches = find_matching_products_semantic(search_term, &candidates, 0.0);
    let semantic_scores: HashMap<String, f64> = semantic_matches
        .into_iter()
        .map(|p| {
            let key = format!("{}|{}|{}", p.product_name, p.store_id, p.price);
            (key, p.similarity_score)
        })
        .collect();

    // Calculate hybrid scores and store in similarity_score field
    let max_price = candidates
        .iter()
        .map(|p| p.price)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let scored_products: Vec<Product> = candidates
        .into_iter()
        .map(|mut p| {
            let key = format!("{}|{}|{}", p.product_name, p.store_id, p.price);
            let semantic_score = semantic_scores.get(&key).copied().unwrap_or(0.0);
            let bm25_score = p.similarity_score;
            let price_score = 1.0 - (p.price / max_price);

            let hybrid_score = (bm25_score * BM25_WEIGHT)
                + (semantic_score * SEMANTIC_WEIGHT)
                + (price_score * PRICE_WEIGHT);

            // Update store_name from store_map if available
            if let Some(store_info) = store_map.get(&p.store_id) {
                p.store_name = store_info.name.clone();
            }

            p.similarity_score = hybrid_score;
            p
        })
        .collect();

    // Group by product name + size (lowercased for deduplication)
    // This ensures "Chocolate Milk 300ml" and "Chocolate Milk 750ml" are separate products
    let mut product_groups: HashMap<String, Vec<Product>> = HashMap::new();
    for p in scored_products {
        let key = format!("{}|{}|{}", p.product_name.to_lowercase(), p.size_value, p.size_unit.to_lowercase());
        product_groups.entry(key).or_default().push(p);
    }

    // Convert groups to MatchedProduct with supermarket_info array
    let mut grouped_products: Vec<MatchedProduct> = product_groups
        .into_iter()
        .map(|(_, mut group)| {
            // Sort stores by price within each product
            group.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

            // Take the best score from the group
            let best_score = group.iter().map(|p| p.similarity_score).fold(0.0_f64, f64::max);
            let product_name = group[0].product_name.clone();
            let brand = group[0].brand.clone();
            let size_value = group[0].size_value;
            let size_unit = group[0].size_unit.clone();

            // Collect store prices, deduplicated by store_id (keep cheapest per store)
            let mut store_prices: HashMap<String, SupermarketInfo> = HashMap::new();
            for p in group {
                let distance_km = store_map
                    .get(&p.store_id)
                    .map(|s| s.distance_km)
                    .unwrap_or(0.0);

                let info = SupermarketInfo {
                    supermarket: p.supermarket,
                    store_name: p.store_name,
                    distance_km: (distance_km * 10.0).round() / 10.0,
                    price: p.price,
                };

                // Only insert if this store hasn't been seen or has a lower price
                store_prices
                    .entry(p.store_id)
                    .and_modify(|existing| {
                        if info.price < existing.price {
                            *existing = info.clone();
                        }
                    })
                    .or_insert(info);
            }

            // Sort by price
            let mut supermarket_info: Vec<SupermarketInfo> = store_prices.into_values().collect();
            supermarket_info.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

            MatchedProduct {
                product_name,
                brand,
                size_value,
                size_unit,
                similarity_score: (best_score * 100.0).round() / 100.0,
                supermarket_info,
            }
        })
        .collect();

    // Sort by best score descending
    grouped_products.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

    // Take top N
    let top_matches: Vec<MatchedProduct> = grouped_products
        .into_iter()
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
        let db_products = queries.search_products_in_stores(search_term, store_ids);
        return db_products
            .into_iter()
            .map(|p| Product {
                product_name: p.product_name,
                brand: p.brand,
                size_value: p.size_value,
                size_unit: p.size_unit,
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
                size_value: r.size_value,
                size_unit: r.size_unit,
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

    #[test]
    fn test_supermarket_info_serialization() {
        let info = SupermarketInfo {
            supermarket: "PakNSave".to_string(),
            store_name: "Pak'n Save Albany".to_string(),
            distance_km: 2.5,
            price: 5.99,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("PakNSave"));
        assert!(json.contains("5.99"));
    }

    #[test]
    fn test_matched_product_serialization() {
        let product = MatchedProduct {
            product_name: "Anchor Milk 2L".to_string(),
            brand: "Anchor".to_string(),
            size_value: 0.0,
            size_unit: "Liter".to_string(),
            similarity_score: 0.95,
            supermarket_info: vec![
                SupermarketInfo {
                    supermarket: "PakNSave".to_string(),
                    store_name: "Pak'n Save Albany".to_string(),
                    distance_km: 2.5,
                    price: 5.99,
                },
                SupermarketInfo {
                    supermarket: "NewWorld".to_string(),
                    store_name: "New World Mt Eden".to_string(),
                    distance_km: 3.1,
                    price: 6.29,
                },
            ],
        };

        let json = serde_json::to_string(&product).unwrap();
        assert!(json.contains("supermarket_info"));
        assert!(json.contains("PakNSave"));
        assert!(json.contains("NewWorld"));
    }
}
