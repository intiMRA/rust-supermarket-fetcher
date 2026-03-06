use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::Value;

use crate::custom_types::error::FetchError;
use crate::models::category::{Category, find_trace, flatten_category_paths};
use crate::models::store::{Store, StoresResponse};
use crate::models::super_market_item::SuperMarketItem;
use crate::models::token::Token;
use crate::protocols::food_stuff_common_protocol::FoodStuffCommonsProtocol;
use crate::protocols::logger_protocol::LoggerProtocol;
use crate::protocols::super_market_fetcher_protocol::SuperMarketFetcherProtocol;

const DEFAULT_STORE_ID: &str = "21ecaaed-0749-4492-985e-4bb7ba43d59c";

// -----------------------------------------------------------------------------
// Struct Definition
// -----------------------------------------------------------------------------

pub struct PackNSaveFetcher {
    client: Client,
    token: Option<Token>,
    categories: Option<Vec<Category>>,
    logger: Box<dyn LoggerProtocol>,
    commons: Box<dyn FoodStuffCommonsProtocol>,
}

// -----------------------------------------------------------------------------
// Constructor
// -----------------------------------------------------------------------------

impl PackNSaveFetcher {
    pub fn new(logger: Box<dyn LoggerProtocol>, commons: Box<dyn FoodStuffCommonsProtocol>) -> Self {
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

impl PackNSaveFetcher {
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

impl PackNSaveFetcher {
    async fn fetch_items_for_category_path(
        &self,
        store_id: &str,
        category_path: &[String],
    ) -> Result<Vec<SuperMarketItem>, FetchError> {
        let category_display = category_path.join(" > ");
        self.logger.fetching_category(&category_display);

        let token = self.get_auth().await?;
        let headers = self.commons.build_headers(token);
        let filter = self.commons.build_category_filter(store_id, category_path);

        let mut page = 0;
        let mut items: Vec<SuperMarketItem> = Vec::new();

        loop {
            let body = serde_json::json!({
                "algoliaQuery": {
                    "attributesToHighlight": [],
                    "attributesToRetrieve": ["productID", "Type", "sponsored", "category0SI", "category1SI", "category2SI"],
                    "facets": ["brand", "onPromotion", "productFacets", "tobacco"],
                    "filters": filter,
                    "highlightPostTag": "__/ais-highlight__",
                    "highlightPreTag": "__ais-highlight__",
                    "hitsPerPage": 5000,
                    "maxValuesPerFacet": 5000,
                    "page": page,
                    "analyticsTags": ["fs#WEB:desktop"]
                },
                "algoliaFacetQueries": [],
                "storeId": store_id,
                "hitsPerPage": 50,
                "page": page,
                "sortOrder": "SI_POPULARITY_ASC",
                "tobaccoQuery": false,
                "precisionMedia": {
                    "adDomain": "CATEGORY_PAGE",
                    "adPositions": [4, 8, 12],
                    "publishImpressionEvent": false,
                    "disableAds": true
                }
            });

            let response = self
                .client
                .post("https://api-prod.paknsave.co.nz/v1/edge/search/paginated/products")
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
                .map_err(FetchError::Request)?;

            if !response.status().is_success() {
                self.logger.error(&format!("Failed to fetch: {}", response.status()));
                break;
            }

            let json: Value = response.json().await.map_err(FetchError::Request)?;
            let parsed_products = self.commons.parse_products(json, category_display.clone());
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
// Protocol Implementation
// -----------------------------------------------------------------------------

#[async_trait]
impl SuperMarketFetcherProtocol for PackNSaveFetcher {
    // --- Authentication ---

    async fn get_auth(&self) -> Result<Option<Token>, FetchError> {
        if let Some(token) = &self.token {
            if token.expiry_time > Utc::now() {
                return Ok(Some(token.clone()));
            }
        }

        let headers = self.commons.build_headers(None);
        let response = self
            .client
            .post("https://www.paknsave.co.nz/api/user/get-current-user")
            .headers(headers)
            .send()
            .await
            .map_err(FetchError::Request)?;

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

        let response = self
            .client
            .get("https://api-prod.paknsave.co.nz/v1/edge/store")
            .headers(headers)
            .send()
            .await
            .map_err(FetchError::Request)?;

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

        let response = self
            .client
            .get(format!(
                "https://api-prod.paknsave.co.nz/v1/edge/store/{}/categories",
                store_id
            ))
            .headers(headers)
            .send()
            .await
            .map_err(FetchError::Request)?;

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
        let category_paths = flatten_category_paths(&categories);

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
