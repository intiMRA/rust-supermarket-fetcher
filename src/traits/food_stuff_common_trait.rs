use reqwest::header::HeaderMap;
use serde_json::Value;

use crate::custom_types::supermarket_types::Supermarket;
use crate::models::category::Category;
use crate::models::super_market_item::SuperMarketItem;
use crate::models::token::Token;

pub trait FoodStuffCommonsTrait: Send + Sync {
    fn supermarket(&self) -> Supermarket;
    fn build_headers(&self, token: Option<Token>) -> HeaderMap;
    fn build_category_filter(&self, store_id: &str, category_path: &[String]) -> String;
    fn build_search_body(&self, store_id: &str, filter: &str, page: u32) -> Value;
    fn parse_token(&self, json: &Value) -> Option<Token>;
    fn parse_products(&self, products: Value, fallback_category: &Category) -> Vec<SuperMarketItem>;
}
