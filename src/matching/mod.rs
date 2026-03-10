pub mod fuzzy_matcher;
pub mod semantic_matcher;

pub use fuzzy_matcher::{Product, find_matching_products, find_best_matches};
pub use semantic_matcher::{find_matching_products_semantic, find_best_matches_semantic};
