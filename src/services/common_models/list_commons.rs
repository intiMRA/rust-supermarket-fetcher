use serde::Serialize;

/// Price info from a specific supermarket/store.
#[derive(Debug, Serialize, Clone)]
#[derive(PartialEq)]
pub struct SupermarketInfo {
    pub supermarket: String,
    pub store_name: String,
    pub distance_km: f64,
    pub price: f64,
    pub image_url: String,
    pub latitude: f64,
    pub longitude: f64,
}