use serde::Serialize;
use crate::services::shopping_list_service::SupermarketInfo;

/// A matched product in the response with prices from multiple stores.
#[derive(Debug, Serialize)]
pub struct MatchedProduct {
    pub product_name: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub similarity_score: f64,
    pub supermarket_info: Vec<SupermarketInfo>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedProduct {
    pub product_name: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub supermarket_info: Vec<SupermarketInfo>,
}