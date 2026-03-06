use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, REFERER, USER_AGENT};
use serde_json::Value;

use crate::custom_types::size_unit_types::SizeUnit;
use crate::custom_types::supermarket_types::Supermarket;
use crate::models::super_market_item::SuperMarketItem;
use crate::models::token::Token;
use crate::protocols::food_stuff_common_protocol::FoodStuffCommonsProtocol;

pub struct FoodStuff {
    referer: &'static str,
    category_suffix: &'static str,
    supermarket: Supermarket,
}

impl FoodStuff {
    pub fn new_world() -> Self {
        Self {
            referer: "https://www.newworld.co.nz",
            category_suffix: "NI",
            supermarket: Supermarket::NewWorld,
        }
    }

    pub fn pack_n_save() -> Self {
        Self {
            referer: "https://www.paknsave.co.nz",
            category_suffix: "SI",
            supermarket: Supermarket::PakNSave,
        }
    }
}

impl FoodStuffCommonsProtocol for FoodStuff {
    fn build_headers(&self, token: Option<Token>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(token) = token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token.token)).unwrap(),
            );
        }
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "accept-language",
            HeaderValue::from_static("en-GB,en-US;q=0.9,en;q=0.8"),
        );
        headers.insert("cache-control", HeaderValue::from_static("no-cache"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            REFERER,
            HeaderValue::from_static(self.referer),
        );
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers
    }

    fn build_category_filter(&self, store_id: &str, category_path: &[String]) -> String {
        let mut filter = format!("stores:{}", store_id);
        for (i, category) in category_path.iter().enumerate() {
            filter.push_str(&format!(" AND category{}{}:\"{}\"", i, self.category_suffix, category));
        }
        filter
    }

    fn parse_token(&self, json: &Value) -> Option<Token> {
        if let Some(token) = json["access_token"].as_str()
            && let Some(expiry_time_string) = json["expires_time"].as_str()
            && let Ok(expiry_time) = DateTime::parse_from_rfc3339(expiry_time_string)
        {
            Some(Token {
                token: token.to_string(),
                expiry_time: expiry_time.with_timezone(&Utc),
            })
        } else {
            None
        }
    }

    fn parse_products(&self, json: Value, category_display: String) -> Vec<SuperMarketItem> {
        let mut items: Vec<SuperMarketItem> = Vec::new();
        if let Some(products) = json["products"].as_array()
            && !products.is_empty()
        {
            for product in products {
                if let Some(id) = product["productId"].as_str()
                    && let Some(name) = product["name"].as_str()
                    && let Some(price_cents) = product["singlePrice"]["price"].as_i64()
                {
                    let brand_name = product["brand"].as_str().unwrap_or("").to_string();
                    let size = product["displayName"].as_str().and_then(SizeUnit::parse);
                    let image_id = id.split('-').next().unwrap_or(id);
                    let image_url = format!(
                        "https://a.fsimg.co.nz/product/retail/fan/image/400x400/{}.png",
                        image_id
                    );

                    items.push(SuperMarketItem {
                        id: id.to_string(),
                        name: name.to_string(),
                        supermarket: self.supermarket,
                        image_url,
                        price: price_cents as f64 / 100.0,
                        brand_name,
                        size,
                        category: category_display.clone(),
                    });
                }
            }
        }
        items
    }
}
