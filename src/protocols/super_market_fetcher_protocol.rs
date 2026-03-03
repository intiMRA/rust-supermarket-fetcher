use crate::custom_types::error::FetchError;
use crate::models::super_market_item::SuperMarketItem;
use async_trait::async_trait;

#[async_trait]
pub trait SuperMarketFetcherProtocol: Send + Sync {
    async fn get_auth(&self) -> Result<Option<String>, FetchError>;
    async fn get_items(&self) -> Result<Vec<SuperMarketItem>, FetchError>;
    async fn get_categories(&self, top_level_only: bool) -> Result<Vec<String>, FetchError>;
    async fn get_items_for_category(
        &self,
        category: &str,
    ) -> Result<Vec<SuperMarketItem>, FetchError>;
}
