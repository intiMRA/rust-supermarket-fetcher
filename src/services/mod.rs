pub mod shopping_list_service;

pub use shopping_list_service::{
    ShoppingListRequest,
    ShoppingListResponse,
    ShoppingListItem,
    MatchedProduct,
    process_shopping_list,
};
