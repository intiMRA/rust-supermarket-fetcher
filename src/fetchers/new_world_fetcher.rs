use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, REFERER, USER_AGENT};
use serde_json::Value;

use crate::custom_types::error::FetchError;
use crate::custom_types::size_unit_types::SizeUnit;
use crate::custom_types::supermarket_types::Supermarket;
use crate::logger::Logger;
use crate::models::category::{Category, flatten_category_paths};
use crate::models::store::{Store, StoresResponse};
use crate::models::super_market_item::SuperMarketItem;
use crate::models::token::Token;
use crate::protocols::super_market_fetcher_protocol::SuperMarketFetcherProtocol;

const DEFAULT_STORE_ID: &str = "60928d93-06fa-4d8f-92a6-8c359e7e846d";

// -----------------------------------------------------------------------------
// Struct Definition
// -----------------------------------------------------------------------------

pub struct NewWorldFetcher {
    client: Client,
    token: Option<Token>,
    categories: Option<Vec<Category>>,
    logger: Logger,
}

// -----------------------------------------------------------------------------
// Constructor & HTTP Helpers
// -----------------------------------------------------------------------------

impl NewWorldFetcher {
    pub fn new(logger: Logger) -> Self {
        Self {
            client: Client::new(),
            token: None,
            categories: None,
            logger,
        }
    }

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
            HeaderValue::from_static("https://www.newworld.co.nz"),
        );
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers
    }
}

// -----------------------------------------------------------------------------
// Category Helpers
// -----------------------------------------------------------------------------

impl NewWorldFetcher {
    fn build_category_filter(store_id: &str, category_path: &[String]) -> String {
        let mut filter = format!("stores:{}", store_id);
        for (i, category) in category_path.iter().enumerate() {
            filter.push_str(&format!(" AND category{}NI:\"{}\"", i, category));
        }
        filter
    }

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

impl NewWorldFetcher {
    async fn fetch_items_for_category_path(
        &self,
        store_id: &str,
        category_path: &[String],
    ) -> Result<Vec<SuperMarketItem>, FetchError> {
        let category_display = category_path.join(" > ");
        self.logger.fetching_category(&category_display);

        let token = self.get_auth().await?;
        let headers = self.build_headers(token);
        let filter = Self::build_category_filter(store_id, category_path);

        let mut page = 0;
        let mut items: Vec<SuperMarketItem> = Vec::new();

        loop {
            let body = serde_json::json!({
                "algoliaQuery": {
                    "attributesToHighlight": [],
                    "attributesToRetrieve": ["productID", "Type", "sponsored", "category0NI", "category1NI", "category2NI", "barCode"],
                    "facets": ["brand", "category1NI", "onPromotion", "productFacets", "tobacco", "code"],
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
                "sortOrder": "NI_POPULARITY_ASC",
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
                .post("https://api-prod.newworld.co.nz/v1/edge/search/paginated/products")
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
                            supermarket: Supermarket::NewWorld,
                            image_url,
                            price: price_cents as f64 / 100.0,
                            brand_name,
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
impl SuperMarketFetcherProtocol for NewWorldFetcher {
    // --- Authentication ---

    async fn get_auth(&self) -> Result<Option<Token>, FetchError> {
        if let Some(token) = &self.token {
            if token.expiry_time > Utc::now() {
                return Ok(Some(token.clone()));
            }
        }

        let headers = self.build_headers(None);
        let response = self
            .client
            .post("https://www.newworld.co.nz/api/user/get-current-user")
            .headers(headers)
            .send()
            .await
            .map_err(FetchError::Request)?;

        let json: Value = response.json().await.map_err(FetchError::Request)?;

        if let Some(token) = json["access_token"].as_str()
            && let Some(expiry_time_string) = json["expires_time"].as_str()
            && let Ok(expiry_time) = DateTime::parse_from_rfc3339(expiry_time_string)
        {
            Ok(Some(Token {
                token: token.to_string(),
                expiry_time: expiry_time.with_timezone(&Utc),
            }))
        } else {
            Err(FetchError::MissingToken)
        }
    }

    // --- Stores ---

    async fn get_stores(&self) -> Result<Vec<Store>, FetchError> {
        self.logger.fetching("stores");

        let token = self.get_auth().await?;
        let headers = self.build_headers(token);

        let response = self
            .client
            .get("https://api-prod.newworld.co.nz/v1/edge/store")
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
        let headers = self.build_headers(token);

        let response = self
            .client
            .get(format!(
                "https://api-prod.newworld.co.nz/v1/edge/store/{}/categories",
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
