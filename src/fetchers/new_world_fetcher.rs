use std::time::Duration;
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;

use crate::custom_types::error::FetchError;
use crate::models::category::{Category, find_trace, top_level_category_paths};
use crate::models::store::{Store, StoresResponse};
use crate::models::super_market_item::SuperMarketItem;
use crate::models::token::Token;
use crate::traits::food_stuff_common_trait::FoodStuffCommonsTrait;
use crate::traits::logger_trait::LoggerTrait;
use crate::traits::super_market_fetcher_trait::SuperMarketFetcherTrait;

const DEFAULT_STORE_ID: &str = "60928d93-06fa-4d8f-92a6-8c359e7e846d";

// -----------------------------------------------------------------------------
// Struct Definition
// -----------------------------------------------------------------------------

pub struct NewWorldFetcher<L: LoggerTrait, C: FoodStuffCommonsTrait> {
    client: Client,
    token: Option<Token>,
    categories: Option<Vec<Category>>,
    logger: L,
    commons: C,
}

// -----------------------------------------------------------------------------
// Constructor
// -----------------------------------------------------------------------------

impl <L: LoggerTrait, C: FoodStuffCommonsTrait>NewWorldFetcher<L,C> {
    pub fn new(logger: L, commons: C) -> Self {
        Self {
            client: Client::new(),
            token: None,
            categories: None,
            logger,
            commons,
        }
    }
}

// -----------------------------------------------------------------------------
// Category Helpers
// -----------------------------------------------------------------------------

impl <L: LoggerTrait, C: FoodStuffCommonsTrait>NewWorldFetcher<L, C> {
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

impl <L: LoggerTrait, C: FoodStuffCommonsTrait>NewWorldFetcher<L, C> {
    fn is_rate_limited(status: u16) -> bool {
        matches!(status, 429 | 403 | 503)
    }

    async fn fetch_items_for_category_path(
        &self,
        store_id: &str,
        category_path: &[String],
    ) -> Result<Vec<SuperMarketItem>, FetchError> {
        let category_display = category_path.join(" > ");
        self.logger.fetching_category(&category_display);

        // Fallback category if item doesn't have categoryTrees
        let fallback_category = Category {
            display_name: category_display.clone(),
            slug: category_path.last().cloned().unwrap_or_default(),
            children: Vec::new(),
            supermarket: self.commons.supermarket(),
        };

        let token = self.get_auth().await?;
        let headers = self.commons.build_headers(token);
        let filter = self.commons.build_category_filter(store_id, category_path);

        let mut page = 0;
        let mut items: Vec<SuperMarketItem> = Vec::new();

        loop {
            let body = self.commons.build_search_body(store_id, &filter, page);

            // Add delay between requests to avoid rate limiting
            sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;

            let mut retry_count = 0;
            let response = loop {
                let result = self
                    .client
                    .post("https://api-prod.newworld.co.nz/v1/edge/search/paginated/products")
                    .headers(headers.clone())
                    .json(&body)
                    .send()
                    .await;

                match result {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        if Self::is_rate_limited(status) {
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
            let parsed_products = self.commons.parse_products(json, &fallback_category);
            if parsed_products.is_empty() {
                break;
            } else {
                items.extend(parsed_products);
                page += 1;
            }
        }

        self.logger.fetched_category(items.len(), &category_display);
        Ok(items)
    }
}

// -----------------------------------------------------------------------------
// Trait Implementation
// -----------------------------------------------------------------------------

#[async_trait]
impl <L: LoggerTrait, C: FoodStuffCommonsTrait>SuperMarketFetcherTrait for NewWorldFetcher<L, C> {
    // --- Authentication ---

    async fn get_auth(&self) -> Result<Option<Token>, FetchError> {
        if let Some(token) = &self.token {
            if token.expiry_time > Utc::now() {
                return Ok(Some(token.clone()));
            }
        }

        let headers = self.commons.build_headers(None);
        let mut retry_count = 0;
        let response = loop {
            let result = self
                .client
                .post("https://www.newworld.co.nz/api/user/get-current-user")
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
                    self.logger.error(&format!("Auth network error: {} - retrying ({}/{})", e, retry_count, MAX_RETRIES));
                    sleep(Duration::from_secs(2u64.pow(retry_count))).await;
                    continue;
                }
            }
        };

        let json: Value = response.json().await.map_err(FetchError::Request)?;

        match self.commons.parse_token(&json) {
            Some(token) => Ok(Some(token)),
            None => Err(FetchError::MissingToken),
        }
    }

    // --- Stores ---

    async fn get_stores(&self) -> Result<Vec<Store>, FetchError> {
        self.logger.fetching("stores");

        let token = self.get_auth().await?;
        let headers = self.commons.build_headers(token);

        let mut retry_count = 0;
        let response = loop {
            let result = self
                .client
                .get("https://api-prod.newworld.co.nz/v1/edge/store")
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
                    self.logger.error(&format!("Stores network error: {} - retrying ({}/{})", e, retry_count, MAX_RETRIES));
                    sleep(Duration::from_secs(2u64.pow(retry_count))).await;
                    continue;
                }
            }
        };

        let stores_response: StoresResponse = response.json().await.map_err(FetchError::Request)?;

        self.logger.fetched(stores_response.stores.len(), "stores");
        Ok(stores_response.stores)
    }

    // --- Categories ---

    async fn get_categories(&mut self, store_id: Option<&str>) -> Result<Vec<Category>, FetchError> {
        if let Some(categories) = &self.categories {
            return Ok(categories.clone());
        }

        self.logger.fetching("categories");
        let store_id = store_id.unwrap_or(DEFAULT_STORE_ID);
        let token = self.get_auth().await?;
        let headers = self.commons.build_headers(token);

        let url = format!(
            "https://api-prod.newworld.co.nz/v1/edge/store/{}/categories",
            store_id
        );
        let mut retry_count = 0;
        let response = loop {
            let result = self
                .client
                .get(&url)
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

        let categories: Vec<Category> = response.json().await.map_err(FetchError::Request)?;

        self.logger.fetched(categories.len(), "top-level categories");
        self.categories = Some(categories.clone());
        Ok(categories)
    }

    // --- Items ---

    async fn get_items(&mut self, store_id: Option<&str>) -> Result<Vec<SuperMarketItem>, FetchError> {
        self.logger.fetching("all items");

        let store_id = store_id.unwrap_or(DEFAULT_STORE_ID);
        let categories = self.get_categories(Some(store_id)).await?;
        let category_paths = top_level_category_paths(&categories);

        self.logger.found(category_paths.len(), "categories to fetch");

        let mut items: Vec<SuperMarketItem> = Vec::new();
        for category_path in &category_paths {
            let category_items = self.fetch_items_for_category_path(store_id, category_path).await?;
            items.extend(category_items);
        }

        self.logger.fetched(items.len(), "total items");
        Ok(items)
    }

    async fn get_items_for_category(
        &mut self,
        store_id: Option<&str>,
        category: &str,
    ) -> Result<Vec<SuperMarketItem>, FetchError> {
        let store_id = store_id.unwrap_or(DEFAULT_STORE_ID);
        let category_path = self.get_category_trace(category, Some(store_id)).await?;
        self.fetch_items_for_category_path(store_id, &category_path).await
    }
}
