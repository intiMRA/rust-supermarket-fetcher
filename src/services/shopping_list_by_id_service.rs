use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::database::{Database, ProductWithPriceAndStore, Queries};
use crate::services::common_models::list_commons::{SupermarketInfo};
pub(crate) use crate::services::common_models::nearby_store::NearbyStore;
use crate::services::common_models::response_product::ProductByIdProduct;
use crate::services::utils::common_logic;
/// Request payload for shopping list processing.
#[derive(Debug, Deserialize)]
pub struct ShoppingListByIDRequest {
    pub items: Vec<String>,
    pub latitude: f64,
    pub longitude: f64,
}
#[derive(Debug, Serialize)]
pub struct ShoppingListByIDResponse {
    items: Vec<ProductByIdProduct>,
}

/// Process a shopping list request using hybrid BM25 + semantic matching.
///
/// Strategy:
/// 1. BM25 (keyword search): Fast, handles exact matches well ("milk" → "Fresh Milk")
/// 2. Semantic (embeddings): Understands meaning ("butter" → "Anchor Butter")
/// 3. Combined scoring: BM25 (40%) + Semantic (20%) + Price (40%)
/// 4. Group by product: Return deduplicated products with prices from all stores
pub fn process_shopping_list_by_ids(
    request: &ShoppingListByIDRequest,
    db: &Database,
) -> ShoppingListByIDResponse {
    let queries = Queries::new(db);

    // Step 1: Find nearby stores
    let nearby_stores =  common_logic::find_stores_to_query(
        &queries,
        request.latitude,
        request.longitude,
    );

    if nearby_stores.is_empty() {
        return ShoppingListByIDResponse {
            items: Vec::new()
        };
    }

    let store_ids: Vec<String> = nearby_stores.iter().map(|s| s.id.clone()).collect();
    let store_map: HashMap<String, &NearbyStore> = nearby_stores
        .iter()
        .map(|s| (s.id.clone(), s))
        .collect();
    
    let products = queries.get_products_by_ids(&store_ids, &request.items);
    let mut product_groups: HashMap<String, Vec<ProductWithPriceAndStore>> = HashMap::new();
    for p in products {
        let key = format!("{}|{}|{}", p.product_name.to_lowercase(), p.size_value, p.size_unit.to_lowercase());
        product_groups.entry(key).or_default().push(p);
    }

    // Convert groups to MatchedProduct with supermarket_info array
    let grouped_products: Vec<ProductByIdProduct> = product_groups
        .into_iter()
        .map(|(_, mut group)| {
            // Sort stores by price within each product
            group.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            
            let product_id = group[0].product_id.clone();
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

            ProductByIdProduct {
                product_id,
                product_name,
                brand,
                size_value,
                size_unit,
                supermarket_info,
            }
        })
        .collect();
    ShoppingListByIDResponse { items: grouped_products }
}