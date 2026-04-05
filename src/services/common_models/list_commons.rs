use serde::Serialize;

/// Price info from a specific supermarket/store.
#[derive(Debug, Serialize, Clone)]
pub struct SupermarketInfo {
    pub supermarket: String,
    pub store_name: String,
    pub distance_km: f64,
    pub price: f64,
}