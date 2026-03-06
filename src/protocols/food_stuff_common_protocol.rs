use reqwest::header::HeaderMap;
use serde_json::Value;

use crate::models::super_market_item::SuperMarketItem;
use crate::models::token::Token;

pub trait FoodStuffCommonsProtocol: Send + Sync {
    fn build_headers(&self, token: Option<Token>) -> HeaderMap;
    fn build_category_filter(&self, store_id: &str, category_path: &[String]) -> String;
    fn parse_token(&self, json: &Value) -> Option<Token>;
    fn parse_products(&self, products: Value, category_display: String) -> Vec<SuperMarketItem>;
}
