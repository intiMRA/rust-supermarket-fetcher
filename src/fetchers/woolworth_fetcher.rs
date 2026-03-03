use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde_json::Value;

use crate::custom_types::error::FetchError;
use crate::custom_types::supermarket_types::Supermarket;
use crate::models::super_market_item::SuperMarketItem;
use crate::protocols::super_market_fetcher_protocol::SuperMarketFetcherProtocol;

pub struct WoolworthFetcher {
    client: Client,
}

impl WoolworthFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://www.woolworths.co.nz/shop/browse"),
        );
        headers.insert(
            "x-requested-with",
            HeaderValue::from_static("OnlineShopping.WebApp"),
        );
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers
    }
}

impl Default for WoolworthFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SuperMarketFetcherProtocol for WoolworthFetcher {
    async fn get_auth(&self) -> Result<Option<String>, FetchError> {
        Ok(None)
    }

    async fn get_items(&self) -> Result<Vec<SuperMarketItem>, FetchError> {
        let categories = self.get_categories(true).await?;
        let mut items: Vec<SuperMarketItem> = Vec::new();
        for category in categories {
            let category_items = self.get_items_for_category(&category).await?;
            items.extend(category_items);
        }
        Ok(items)
    }

    async fn get_categories(&self, top_level_only: bool) -> Result<Vec<String>, FetchError> {
        let headers = self.build_headers();
        let response = self
            .client
            .get("https://www.woolworths.co.nz/api/v1/shell")
            .headers(headers)
            .send()
            .await?;

        let json: Value = response.json().await?;

        let mut categories = Vec::new();
        if let Some(specials) = json["specials"].as_array() {
            for special in specials {
                if let Some(url) = special["url"].as_str() {
                    categories.push(url.to_string());
                }
                if top_level_only {
                    continue;
                }
                if let Some(facets) = special["dasFacets"].as_array() {
                    for facet in facets {
                        if let Some(shelves) = facet["shelfResponses"].as_array() {
                            for shelf in shelves {
                                if let Some(url) = shelf["url"].as_str() {
                                    categories.push(url.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(categories)
    }

    async fn get_items_for_category(
        &self,
        category: &str,
    ) -> Result<Vec<SuperMarketItem>, FetchError> {
        let mut items: Vec<SuperMarketItem> = Vec::new();
        let headers = self.build_headers();

        println!("fetching {}...", category);
        let mut page = 1;
        loop {
            let fetch_url = format!(
                "https://www.woolworths.co.nz/api/v1/products?dasFilter=Department%3B%3B{}%3Btrue&target=browse&inStockProductsOnly=false&size=120&page={}",
                category, page
            );
            let response = self
                .client
                .get(&fetch_url)
                .headers(headers.clone())
                .send()
                .await?;

            if !response.status().is_success() {
                eprintln!("Failed to fetch: {}", response.status());
                break;
            }

            let json: Value = response.json().await?;
            if let Some(json_items) = json["products"]["items"].as_array()
                && !json_items.is_empty()
            {
                for json_item in json_items {
                    if let Some(id) = json_item["barcode"].as_str()
                        && let Some(name) = json_item["name"].as_str()
                        && let Some(image_url) = json_item["images"]["big"].as_str()
                        && let Some(price) = json_item["price"]["salePrice"].as_f64()
                        && let Some(brand_name) = json_item["brand"].as_str()
                    {
                        // TODO: parse size
                        items.push(SuperMarketItem {
                            id: id.to_string(),
                            name: name.to_string(),
                            supermarket: Supermarket::Woolworth,
                            image_url: image_url.to_string(),
                            price,
                            brand_name: brand_name.to_string(),
                            size: None,
                            category: category.to_string(),
                        });
                    }
                }
            } else {
                break;
            }
            page += 1;
        }
        Ok(items)
    }
}
