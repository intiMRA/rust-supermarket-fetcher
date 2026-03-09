use crate::custom_types::error::FetchError;
use crate::supermarkets::models::super_market_item::SuperMarketItem;
use crate::supermarkets::models::category::Category;
use async_trait::async_trait;
use crate::supermarkets::models::store::Store;
use crate::supermarkets::models::token::Token;

#[async_trait]
pub trait SuperMarketFetcherTrait: Send + Sync {
    async fn get_auth(&self) -> Result<Option<Token>, FetchError>;
    async fn get_items(&mut self, store_id: Option<&str>) -> Result<Vec<SuperMarketItem>, FetchError>;
    async fn get_categories(&mut self, store_id: Option<&str>) -> Result<Vec<Category>, FetchError>;
    async fn get_items_for_category(
        &mut self,
        store_id: Option<&str>,
        category: &str,
    ) -> Result<Vec<SuperMarketItem>, FetchError>;
    async fn get_stores(&self) -> Result<Vec<Store>, FetchError>;
}
