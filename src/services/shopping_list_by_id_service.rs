use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    pub fewest_trips: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct ShoppingListByIDResponse {
    items: Vec<ProductByIdProduct>,
}

#[derive(Debug, Serialize)]
pub struct BestListResponse {
    items: Vec<BestListProduct>,
    total_price: f64,
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
                let (distance_km, store_name, lat, lon) = match store_map.get(&p.store_id) {
                    Some(s) => (s.distance_km, s.name.clone(), s.latitude, s.longitude),
                    None => (0.0, p.store_name, p.store_latitude, p.store_longitude),
                };

                let info = SupermarketInfo {
                    supermarket: p.supermarket,
                    store_name,
                    distance_km: (distance_km * 10.0).round() / 10.0,
                    price: p.price,
                    image_url: p.image_url.clone(),
                    latitude: lat,
                    longitude: lon,
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

    let mut best_list: Vec<BestListProduct> = Vec::new();
    let mut required_supermarkets: Vec<SupermarketInfo> = Vec::new();
    let mut multi_supermarket_items: Vec<ProductByIdProduct> = Vec::new();

    for item in &id_response.items {
        if item.supermarket_info.len() == 1 {
            best_list.push(BestListProduct {
                product_id: item.product_id,
                product_name: item.product_name.clone(),
                brand: item.brand.clone(),
                size_unit: item.size_unit.clone(),
                size_value: item.size_value,
                supermarket_info: item.supermarket_info[0].clone(),
            });
            if !required_supermarkets.iter().any(|s| s.store_name == item.supermarket_info[0].store_name) {
                required_supermarkets.push(item.supermarket_info[0].clone());
            }
        } else {
            multi_supermarket_items.push(item.clone());
        }
    }

    if multi_supermarket_items.is_empty() {
        let total_price = calculate_list_price(&best_list, &[]);
        return BestListResponse { items: best_list, total_price };
    }

    // Collect all unique stores across multi-supermarket items
    let mut unique_stores: Vec<SupermarketInfo> = Vec::new();
    for item in &multi_supermarket_items {
        for s in &item.supermarket_info {
            if !unique_stores.iter().any(|u| u.store_name == s.store_name) {
                unique_stores.push(s.clone());
            }
        }
    }

    let num_stores = unique_stores.len();

    // Safeguard: if too many unique stores, fall back to greedy (cheapest per item)
    if num_stores > 20 {
        for item in &multi_supermarket_items {
            let cheapest = item.supermarket_info.iter()
                .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
                .unwrap();
            best_list.push(BestListProduct {
                product_id: item.product_id,
                product_name: item.product_name.clone(),
                brand: item.brand.clone(),
                size_unit: item.size_unit.clone(),
                size_value: item.size_value,
                supermarket_info: cheapest.clone(),
            });
        }
        let total_price = calculate_list_price(&best_list, &[]);
        return BestListResponse { items: best_list, total_price };
    }

    // Enumerate all subsets of unique stores (bitmask)
    let total_subsets = 1u32 << num_stores;
    let mut best_cost = f64::INFINITY;
    let mut best_assignment: Option<Vec<BestListProduct>> = None;

    for mask in 1..total_subsets {
        // Check coverage: every item must have at least one store in this subset
        let mut covered = true;
        let mut assignment: Vec<BestListProduct> = Vec::with_capacity(multi_supermarket_items.len());

        for item in &multi_supermarket_items {
            let mut cheapest: Option<&SupermarketInfo> = None;
            for s in &item.supermarket_info {
                // Check if this store is in the current subset
                let store_idx = unique_stores.iter().position(|u| u.store_name == s.store_name).unwrap();
                if mask & (1 << store_idx) != 0 {
                    if cheapest.is_none() || s.price < cheapest.unwrap().price {
                        cheapest = Some(s);
                    }
                }
            }
            match cheapest {
                Some(s) => {
                    assignment.push(BestListProduct {
                        product_id: item.product_id,
                        product_name: item.product_name.clone(),
                        brand: item.brand.clone(),
                        size_unit: item.size_unit.clone(),
                        size_value: item.size_value,
                        supermarket_info: s.clone(),
                    });
                }
                None => {
                    covered = false;
                    break;
                }
            }
        }

        if !covered {
            continue;
        }

        let cost = calculate_list_price(&assignment, &required_supermarkets);
        if request.fewest_trips.unwrap_or(false) {
            let store_count = count_total_stores(&assignment, &required_supermarkets);
            let best_count = best_assignment.as_ref()
                .map_or(i32::MAX, |a| count_total_stores(a, &required_supermarkets));
            if store_count < best_count || (store_count == best_count && cost < best_cost) {
                best_cost = cost;
                best_assignment = Some(assignment);
            }
        } else if cost < best_cost {
            best_cost = cost;
            best_assignment = Some(assignment);
        }
    }

    if let Some(assignment) = best_assignment {
        best_list.extend(assignment);
    }

    let total_price = calculate_list_price(&best_list, &[]);
    BestListResponse { items: best_list, total_price }
}

fn count_total_stores(assignment: &[BestListProduct], required_supermarkets: &[SupermarketInfo]) -> i32 {
    let mut store_names: HashSet<&str> = HashSet::new();
    for s in required_supermarkets {
        store_names.insert(&s.store_name);
    }
    for item in assignment {
        store_names.insert(&item.supermarket_info.store_name);
    }
    store_names.len() as i32
}
fn calculate_list_price(item_list: &[BestListProduct], initial_supermarkets: &[SupermarketInfo]) -> f64 {
    let mut used_supermarkets: Vec<&SupermarketInfo> = initial_supermarkets.iter().collect();
    let mut price = 0.0;
    for item in item_list {
        if !used_supermarkets.iter().any(|s| s.store_name == item.supermarket_info.store_name) {
            // Petrol cost = minimum distance from any already-visited store (incremental detour).
            // If no stores visited yet, use distance from user.
            let petrol_distance = if used_supermarkets.is_empty() {
                item.supermarket_info.distance_km
            } else {
                used_supermarkets.iter()
                    .map(|s| haversine_distance_km(
                        s.latitude, s.longitude,
                        item.supermarket_info.latitude, item.supermarket_info.longitude,
                    ))
                    .fold(f64::INFINITY, f64::min)
                    .min(item.supermarket_info.distance_km)
            };
            price += petrol_distance * PETROL_PRICE;
            used_supermarkets.push(&item.supermarket_info);
        }
        price += item.supermarket_info.price;
    }
    price
}
