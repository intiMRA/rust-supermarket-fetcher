use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::option::Option;

use crate::database::{Database, ProductWithPriceAndStore, Queries};
use crate::services::common_models::list_commons::{SupermarketInfo};
pub(crate) use crate::services::common_models::nearby_store::NearbyStore;
use crate::services::common_models::response_product::{BestListProduct, ProductByIdProduct};
use crate::services::utils::common_logic;
use crate::utils::geo::haversine_distance_km;
const PETROL_PRICE: f64 = 0.26;
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

#[derive(Debug, Serialize)]
pub struct BestListResponse {
    items: Vec<BestListProduct>,
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
                    image_url: p.image_url.clone(),
                    latitude: p.store_latitude,
                    longitude: p.store_longitude,
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

pub fn find_best_list(
    request: &ShoppingListByIDRequest,
    db: &Database,
) -> BestListResponse {
    let id_response = process_shopping_list_by_ids(request, db);

    let mut best_list = Vec::new();

    let mut required_supermarkets = Vec::new();

    let mut multi_supermarket_items = Vec::new();

    for item in &id_response.items {
        let mut cheapest_supermarket: Option<SupermarketInfo> = None;
        for supermarket in &item.supermarket_info {
            if cheapest_supermarket.is_none() || supermarket.price < cheapest_supermarket.as_ref().unwrap().price {
                cheapest_supermarket = Some(supermarket.clone());
            }
        }
        let best_item = BestListProduct {
            product_id: item.clone().product_id,
            product_name: item.clone().product_name,
            brand: item.clone().brand,
            size_unit: item.clone().size_unit,
            size_value: item.clone().size_value,
            supermarket_info: cheapest_supermarket.unwrap(),
        };
        if item.supermarket_info.len() == 1 {
            best_list.push(best_item.clone());
            if !required_supermarkets.iter().any(|s: &SupermarketInfo| s.store_name == item.supermarket_info[0].store_name) {
                required_supermarkets.push(item.supermarket_info[0].clone());
            }
        }
        else {
            multi_supermarket_items.push(item.clone());
        }
    }
    if best_list.len() == id_response.items.len() {
        return BestListResponse {
            items: best_list
        }
    }
    let mut item_lists: Vec<Vec<BestListProduct>> = vec![vec![]];

    for item in multi_supermarket_items {
        let mut next_generation_of_lists = Vec::new();

        for supermarket in &item.supermarket_info {
            let best_item = BestListProduct {
                product_id: item.product_id,
                product_name: item.product_name.clone(),
                brand: item.brand.clone(),
                size_unit: item.size_unit.clone(),
                size_value: item.size_value,
                supermarket_info: supermarket.clone(),
            };

            for existing_list in &item_lists {
                let mut new_list = existing_list.clone();
                new_list.push(best_item.clone());
                next_generation_of_lists.push(new_list);
            }
        }
        item_lists = next_generation_of_lists;
    }
    let mut cheapest_list = None;
    for item_list in item_lists {
        if cheapest_list.is_none() {
            cheapest_list = Some((item_list.clone(), f64::INFINITY));
        }
        let mut used_supermarkets = required_supermarkets.clone();
        let mut price = 0.0;
        for item in &item_list {
            if !used_supermarkets.iter().any(|s: &SupermarketInfo| s.store_name == item.supermarket_info.store_name) {
                let mut should_apply_petrol_fee = true;
                for used_supermarket in &used_supermarkets {
                    let distance = haversine_distance_km(
                        used_supermarket.latitude,
                        used_supermarket.longitude,
                        item.supermarket_info.latitude,
                        item.supermarket_info.longitude,
                    );
                    if distance <= 0.3 {
                        should_apply_petrol_fee = false;
                        break;
                    }
                }
                if should_apply_petrol_fee {
                    price += item.supermarket_info.distance_km * PETROL_PRICE;
                }
                used_supermarkets.push(item.supermarket_info.clone());
        }
            price += item.supermarket_info.price;
        }
        if price < cheapest_list.as_ref().unwrap().1 {
            cheapest_list = Some((item_list.clone(), price));
        }
    }
    for item in &cheapest_list.unwrap().0 {
        best_list.push(item.clone());
    }
    BestListResponse {
        items: best_list
    }
}
