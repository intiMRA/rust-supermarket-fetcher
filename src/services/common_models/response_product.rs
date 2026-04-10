use serde::Serialize;
use crate::services::common_models::list_commons::SupermarketInfo;

/// A matched product in the response with prices from multiple stores.
#[derive(Debug, Serialize)]
pub struct MatchedProduct {
    pub product_id: i32,
    pub product_name: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub similarity_score: f64,
    pub supermarket_info: Vec<SupermarketInfo>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedProduct {
    pub product_id: i64,
    pub product_name: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub supermarket_info: Vec<SupermarketInfo>,
}

#[derive(Debug, Serialize)]
pub struct ProductByIdProduct {
    pub product_id: i32,
    pub product_name: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub supermarket_info: Vec<SupermarketInfo>,
}