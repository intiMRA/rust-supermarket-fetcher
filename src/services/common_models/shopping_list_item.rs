use serde::Serialize;
use crate::services::common_models::response_product::MatchedProduct;

/// A single item from the shopping list with its matches.
#[derive(Debug, Serialize)]
pub struct ShoppingListItem {
    pub search_term: String,
    pub top_matches: Vec<MatchedProduct>,
}