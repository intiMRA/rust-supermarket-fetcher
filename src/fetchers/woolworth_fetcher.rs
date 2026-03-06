use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, REFERER, USER_AGENT};
use serde_json::Value;

use crate::custom_types::error::FetchError;
use crate::custom_types::size_unit_types::SizeUnit;
use crate::custom_types::supermarket_types::Supermarket;
use crate::logger::Logger;
use crate::models::category::{Category, flatten_category_paths};
use crate::models::store::Store;
use crate::models::super_market_item::SuperMarketItem;
use crate::models::token::Token;
use crate::protocols::super_market_fetcher_protocol::SuperMarketFetcherProtocol;

// -----------------------------------------------------------------------------
// Category Parsing Helpers
// -----------------------------------------------------------------------------

fn to_slug(name: &str) -> String {
    name.to_lowercase()
        .replace(" & ", "-")
        .replace(", ", "-")
        .replace(' ', "-")
}

fn parse_shelves(shelves: &Value) -> Vec<Category> {
    let mut children = Vec::new();
    if let Some(shelf_arr) = shelves.as_array() {
        for shelf in shelf_arr {
            if let Some(url) = shelf["url"].as_str() {
                children.push(Category {
                    name: url.to_string(),
                    children: Vec::new(),
                });
            }
        }
    }
    children
}

fn parse_aisles(facets: &Value) -> Vec<Category> {
    let mut children = Vec::new();
    if let Some(facet_arr) = facets.as_array() {
        for facet in facet_arr {
            let shelves = parse_shelves(&facet["shelfResponses"]);
            if let Some(name) = facet["name"].as_str() {
                children.push(Category {
                    name: to_slug(name),
                    children: shelves,
                });
            }
        }
    }
    children
}

// -----------------------------------------------------------------------------
// Struct Definition
// -----------------------------------------------------------------------------

pub struct WoolworthFetcher {
    client: Client,
    categories: Option<Vec<Category>>,
    logger: Logger,
}

// -----------------------------------------------------------------------------
// Constructor & HTTP Helpers
// -----------------------------------------------------------------------------

impl WoolworthFetcher {
    pub fn new(logger: Logger) -> Self {
        Self {
            client: Client::new(),
            categories: None,
            logger,
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

// -----------------------------------------------------------------------------
// Category Helpers
// -----------------------------------------------------------------------------

impl WoolworthFetcher {
    async fn get_category_trace(
        &mut self,
        category_name: &str,
        store_id: Option<&str>,
    ) -> Result<Vec<String>, FetchError> {
        let categories = self.get_categories(store_id).await?;
        for category in &categories {
            let trace = category.get_trace(category_name);
            if !trace.is_empty() {
                return Ok(trace);
            }
        }
        Ok(Vec::new())
    }
}

// -----------------------------------------------------------------------------
// Internal Fetch Methods
// -----------------------------------------------------------------------------

impl WoolworthFetcher {
    async fn fetch_items_for_category_path(
        &self,
        category_path: &[String],
    ) -> Result<Vec<SuperMarketItem>, FetchError> {
        let category_display = category_path.join(" > ");
        self.logger.fetching_category(&category_display);

        let headers = self.build_headers();

        // Build dasFilter parameters based on path depth
        let mut filters = String::new();
        let filter_types = ["Department", "Aisle", "Shelf"];
        for (i, cat) in category_path.iter().enumerate() {
            if i < filter_types.len() {
                filters.push_str(&format!(
                    "&dasFilter={}%3B%3B{}%3Bfalse",
                    filter_types[i], cat
                ));
            }
        }

        let mut page = 1;
        let mut items: Vec<SuperMarketItem> = Vec::new();

        loop {
            let fetch_url = format!(
                "https://www.woolworths.co.nz/api/v1/products?{}&target=browse&inStockProductsOnly=false&size=120&page={}",
                filters.trim_start_matches('&'),
                page
            );

            let response = self
                .client
                .get(&fetch_url)
                .headers(headers.clone())
                .send()
                .await
                .map_err(FetchError::Request)?;

            if !response.status().is_success() {
                self.logger.error(&format!("Failed to fetch: {}", response.status()));
                break;
            }

            let json: Value = response.json().await.map_err(FetchError::Request)?;

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
                        let size = json_item["size"]["volumeSize"]
                            .as_str()
                            .and_then(SizeUnit::parse);

                        items.push(SuperMarketItem {
                            id: id.to_string(),
                            name: name.to_string(),
                            supermarket: Supermarket::Woolworth,
                            image_url: image_url.to_string(),
                            price,
                            brand_name: brand_name.to_string(),
                            size,
                            category: category_display.clone(),
                        });
                    }
                }
            } else {
                break;
            }
            page += 1;
        }

        self.logger.fetched_category(items.len(), &category_display);
        Ok(items)
    }
}

// -----------------------------------------------------------------------------
// Protocol Implementation
// -----------------------------------------------------------------------------

#[async_trait]
impl SuperMarketFetcherProtocol for WoolworthFetcher {
    // --- Authentication ---

    async fn get_auth(&self) -> Result<Option<Token>, FetchError> {
        Ok(None)
    }

    // --- Stores ---

    async fn get_stores(&self) -> Result<Vec<Store>, FetchError> {
        Ok(Vec::new())
    }

    // --- Categories ---

    async fn get_categories(&mut self, _store_id: Option<&str>) -> Result<Vec<Category>, FetchError> {
        if let Some(categories) = &self.categories {
            return Ok(categories.clone());
        }

        self.logger.fetching("categories");
        let headers = self.build_headers();
        let response = self
            .client
            .get("https://www.woolworths.co.nz/api/v1/shell")
            .headers(headers)
            .send()
            .await
            .map_err(FetchError::Request)?;

        let json: Value = response.json().await.map_err(FetchError::Request)?;

        let mut categories = Vec::new();
        if let Some(specials) = json["specials"].as_array() {
            for special in specials {
                if let Some(url) = special["url"].as_str() {
                    let aisles = parse_aisles(&special["dasFacets"]);
                    categories.push(Category {
                        name: url.to_string(),
                        children: aisles,
                    });
                }
            }
        }

        self.logger.fetched(categories.len(), "top-level categories");
        self.categories = Some(categories.clone());
        Ok(categories)
    }

    // --- Items ---

    async fn get_items(&mut self, _store_id: Option<&str>) -> Result<Vec<SuperMarketItem>, FetchError> {
        self.logger.fetching("all items");

        let categories = self.get_categories(None).await?;
        let category_paths = flatten_category_paths(&categories);

        self.logger.found(category_paths.len(), "categories to fetch");

        let mut items: Vec<SuperMarketItem> = Vec::new();
        for category_path in &category_paths {
            let category_items = self.fetch_items_for_category_path(category_path).await?;
            items.extend(category_items);
        }

        self.logger.fetched(items.len(), "total items");
        Ok(items)
    }

    async fn get_items_for_category(
        &mut self,
        _store_id: Option<&str>,
        category: &str,
    ) -> Result<Vec<SuperMarketItem>, FetchError> {
        let category_path = self.get_category_trace(category, None).await?;
        self.fetch_items_for_category_path(&category_path).await
    }
}
