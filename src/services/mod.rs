pub mod shopping_list_service;
pub mod paginated_list_service;
pub mod common_models;
pub mod utils;
pub mod shopping_list_by_id_service;

pub use shopping_list_service::{
    ShoppingListRequest,
    process_shopping_list,
};
