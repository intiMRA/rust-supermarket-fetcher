use std::time::Duration;
use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, REFERER, USER_AGENT};
use serde_json::Value;
use tokio::time::sleep;

use crate::custom_types::error::FetchError;
use crate::loggers::empty_brand_logger::log_empty_brand;
use crate::supermarkets::size_unit_types::SizeUnit;
use crate::supermarkets::supermarket_types::Supermarket;
use crate::supermarkets::models::category::{Category, find_trace, top_level_category_paths};
use crate::loggers::logger_trait::LoggerTrait;
use crate::supermarkets::models::store::Store;
use crate::supermarkets::models::super_market_item::SuperMarketItem;
use crate::supermarkets::models::token::Token;
use crate::supermarkets::super_market_fetcher_trait::SuperMarketFetcherTrait;

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
                    display_name: String::new(),
                    slug: url.to_string(),
                    children: Vec::new(),
                    supermarket: Supermarket::Woolworth,
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
            if let Some(aisle_name) = facet["name"].as_str() {
                children.push(Category {
                    display_name: String::new(),
                    slug: to_slug(aisle_name),
                    children: shelves,
                    supermarket: Supermarket::Woolworth,
                });
            }
        }
    }
    children
}

// -----------------------------------------------------------------------------
// Struct Definition
// -----------------------------------------------------------------------------

pub struct WoolworthFetcher<L: LoggerTrait> {
    client: Client,
    categories: Option<Vec<Category>>,
    logger: L,
}

// -----------------------------------------------------------------------------
// Constructor & HTTP Helpers
// -----------------------------------------------------------------------------

impl<L: LoggerTrait> WoolworthFetcher<L> {
    pub fn new(logger: L) -> Self {
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

impl<L: LoggerTrait> WoolworthFetcher<L> {
    async fn get_category_trace(
        &mut self,
        category_name: &str,
        store_id: Option<&str>,
    ) -> Result<Vec<String>, FetchError> {
        let categories = self.get_categories(store_id).await?;
        Ok(find_trace(&categories, category_name))
    }
}

// -----------------------------------------------------------------------------
// Internal Fetch Methods
// -----------------------------------------------------------------------------

const REQUEST_DELAY_MS: u64 = 100;
const MAX_RETRIES: u32 = 3;

fn is_rate_limited(status: u16) -> bool {
    matches!(status, 429 | 403 | 503)
}

impl<L: LoggerTrait> WoolworthFetcher<L> {

    fn parse_breadcrumb_category(json: &Value) -> Category {
        let breadcrumb = &json["breadcrumb"];
        let department = breadcrumb["department"]["name"].as_str().unwrap_or("");
        let aisle = breadcrumb["aisle"]["name"].as_str().unwrap_or("");
        let shelf = breadcrumb["shelf"]["name"].as_str().unwrap_or("");

        let mut parts = vec![];
        if !department.is_empty() { parts.push(department); }
        if !aisle.is_empty() { parts.push(aisle); }
        if !shelf.is_empty() { parts.push(shelf); }

        let display_name = parts.join(" > ");
        let slug = parts.last().map(|s| s.to_string()).unwrap_or_default();

        Category {
            display_name,
            slug,
            children: Vec::new(),
            supermarket: Supermarket::Woolworth,
        }
    }

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
        for (i, cat) in category_path.into_iter().enumerate() {
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

            // Add delay between requests to avoid rate limiting
            sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;

            let mut retry_count = 0;
            let response = loop {
                let result = self
                    .client
                    .get(&fetch_url)
                    .headers(headers.clone())
                    .send()
                    .await;

                match result {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        if is_rate_limited(status) {
                            self.logger.rate_limit_warning(status, &category_display);
                            retry_count += 1;
                            if retry_count >= MAX_RETRIES {
                                return Err(FetchError::RateLimited(status));
                            }
                            self.logger.retrying(retry_count, MAX_RETRIES);
                            // Exponential backoff: 2s, 4s, 8s...
                            sleep(Duration::from_secs(2u64.pow(retry_count))).await;
                            continue;
                        }
                        break resp;
                    }
                    Err(e) => {
                        // Retry on network errors (connection reset, incomplete message, etc.)
                        retry_count += 1;
                        if retry_count >= MAX_RETRIES {
                            return Err(FetchError::Request(e));
                        }
                        self.logger.error(&format!("Network error: {} - retrying ({}/{})", e, retry_count, MAX_RETRIES));
                        sleep(Duration::from_secs(2u64.pow(retry_count))).await;
                        continue;
                    }
                }
            };

            if !response.status().is_success() {
                self.logger.error(&format!("Failed to fetch: {}", response.status()));
                break;
            }

            let json: Value = response.json().await.map_err(FetchError::Request)?;

            // Parse category from breadcrumb
            let category = Self::parse_breadcrumb_category(&json);

            if let Some(json_items) = json["products"]["items"].as_array()
                && !json_items.is_empty()
            {
                for json_item in json_items {
                    if let Some(id) = json_item["barcode"].as_str()
                        && let Some(name) = json_item["name"].as_str()
                        && let Some(image_url) = json_item["images"]["big"].as_str()
                        && let Some(price) = json_item["price"]["salePrice"].as_f64()
                    {
                        let brand_name = json_item["brand"].as_str().unwrap_or("").to_string();

                        // Log original server JSON when brand is empty
                        if brand_name.is_empty() {
                            let original_json = serde_json::to_string_pretty(json_item)
                                .unwrap_or_else(|_| json_item.to_string());
                            log_empty_brand(&original_json, id, Supermarket::Woolworth.name());
                        }

                        let size = SizeUnit::parse(json_item["size"]["volumeSize"]
                            .as_str()
                            .unwrap_or("Unknown"));

                        items.push(SuperMarketItem {
                            id: id.to_string(),
                            name: name.to_string(),
                            supermarket: Supermarket::Woolworth,
                            image_url: image_url.to_string(),
                            price,
                            brand_name,
                            size,
                            category: category.clone(),
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
// Woolworths Store Locations (CDX API)
// -----------------------------------------------------------------------------

/// Fetch all Woolworths physical store locations from the CDX site-location API.
///
/// Uses curl as a subprocess because the CDX API (behind Akamai) blocks
/// reqwest connections when concurrent requests are hitting woolworths.co.nz.
pub async fn fetch_woolworths_store_locations() -> Result<Vec<Store>, FetchError> {
    let url = "https://api.cdx.nz/site-location/api/v1/sites?latitude=-41.2924&longitude=174.7787&maxResults=10000";
    println!("[Woolworths] Requesting store locations from CDX API...");

    let output = tokio::process::Command::new("curl")
        .args([
            "-s",
            "--max-time", "30",
            url,
            "-H", "accept: application/json, text/plain, */*",
            "-H", "accept-language: en-GB,en-US;q=0.9,en;q=0.8",
            "-H", "origin: https://www.woolworths.co.nz",
            "-H", "referer: https://www.woolworths.co.nz/",
            "-H", "sec-ch-ua: \"Not:A-Brand\";v=\"99\", \"Google Chrome\";v=\"145\", \"Chromium\";v=\"145\"",
            "-H", "sec-ch-ua-mobile: ?0",
            "-H", "sec-ch-ua-platform: \"macOS\"",
            "-H", "sec-fetch-dest: empty",
            "-H", "sec-fetch-mode: cors",
            "-H", "sec-fetch-site: cross-site",
            "-H", "user-agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36",
        ])
        .output()
        .await
        .map_err(|e| FetchError::UnexpectedResponse(format!("Failed to run curl: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FetchError::UnexpectedResponse(format!("curl failed: {}", stderr)));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&body)
        .map_err(|e| FetchError::UnexpectedResponse(format!("Invalid JSON from CDX API: {}", e)))?;

    let mut stores = Vec::new();
    if let Some(site_details) = json["siteDetail"].as_array() {
        for detail in site_details {
            let site = &detail["site"];
            if let (Some(id), Some(name), Some(lat), Some(lon)) = (
                site["id"].as_i64(),
                site["name"].as_str(),
                site["latitude"].as_f64(),
                site["longitude"].as_f64(),
            ) {
                let address_line = site["addressLine1"].as_str().unwrap_or("");
                let suburb = site["suburb"].as_str().unwrap_or("");
                let address = if suburb.is_empty() {
                    address_line.to_string()
                } else {
                    format!("{}, {}", address_line, suburb)
                };

                stores.push(Store {
                    id: id.to_string(),
                    name: name.to_string(),
                    address,
                    latitude: lat,
                    longitude: lon,
                });
            }
        }
    }

    Ok(stores)
}

// -----------------------------------------------------------------------------
// Trait Implementation
// -----------------------------------------------------------------------------

#[async_trait]
impl<L: LoggerTrait + Send + Sync> SuperMarketFetcherTrait for WoolworthFetcher<L> {
    // --- Authentication ---

    async fn get_auth(&self) -> Result<Option<Token>, FetchError> {
        Ok(None)
    }

    // --- Stores ---
    // Woolworths has uniform pricing nationwide, so we use a single "default" store

    async fn get_stores(&self) -> Result<Vec<Store>, FetchError> {
        Ok(vec![Store {
            id: "default".to_string(),
            name: "Woolworths (All Stores)".to_string(),
            address: String::new(),
            latitude: 0.0,
            longitude: 0.0,
        }])
    }

    // --- Categories ---

    async fn get_categories(&mut self, _store_id: Option<&str>) -> Result<Vec<Category>, FetchError> {
        if let Some(categories) = &self.categories {
            return Ok(categories.clone());
        }

        self.logger.fetching("categories");
        let headers = self.build_headers();
        let mut retry_count = 0;
        let response = loop {
            let result = self
                .client
                .get("https://www.woolworths.co.nz/api/v1/shell")
                .headers(headers.clone())
                .send()
                .await;

            match result {
                Ok(resp) => break resp,
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        return Err(FetchError::Request(e));
                    }
                    self.logger.error(&format!("Categories network error: {} - retrying ({}/{})", e, retry_count, MAX_RETRIES));
                    sleep(Duration::from_secs(2u64.pow(retry_count))).await;
                    continue;
                }
            }
        };

        let json: Value = response.json().await.map_err(FetchError::Request)?;

        let mut categories = Vec::new();
        if let Some(specials) = json["specials"].as_array() {
            for special in specials {
                if let Some(url) = special["url"].as_str() {
                    let aisles = parse_aisles(&special["dasFacets"]);
                    categories.push(Category {
                        display_name: String::new(),
                        slug: url.to_string(),
                        children: aisles,
                        supermarket: Supermarket::Woolworth,
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
        let category_paths = top_level_category_paths(&categories);

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
