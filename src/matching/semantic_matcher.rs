//! Semantic product search for query-time matching.
//!
//! Uses embeddings to find products that semantically match a search term.

use serde::Serialize;
use super::embedding::{cosine_similarity, Embeddable, EmbeddingService};

/// Product with search result information.
#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub product_id: i32,
    pub product_name: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub price: f64,
    pub supermarket: String,
    pub store_name: String,
    pub store_id: String,
    pub store_latitude: f64,
    pub store_longitude: f64,
    pub similarity_score: f64,
    pub image_url: String,
}

impl Embeddable for Product {
    fn to_embedding_text(&self) -> String {
        if self.brand.is_empty() {
            self.product_name.clone()
        } else {
            format!("{} {}", self.brand, self.product_name)
        }
    }
}

/// Trait for semantic product search.
pub trait SemanticSearch {
    type Item;

    /// Find items that semantically match the search term.
    fn find_matches(&self, search_term: &str, threshold: f64) -> Vec<Self::Item>;
}

/// Semantic product searcher.
pub struct ProductSearcher<'a> {
    products: &'a [Product],
}

impl<'a> ProductSearcher<'a> {
    pub fn new(products: &'a [Product]) -> Self {
        Self { products }
    }
}

impl<'a> SemanticSearch for ProductSearcher<'a> {
    type Item = Product;

    fn find_matches(&self, search_term: &str, threshold: f64) -> Vec<Product> {
        find_matching_products_semantic(search_term, self.products, threshold)
    }
}

/// Find products that semantically match the search term using embeddings.
///
/// # Arguments
/// * `search_term` - The term to search for
/// * `products` - List of products to search through
/// * `threshold` - Minimum similarity score (0.0 to 1.0, recommended: 0.3-0.5)
///
/// # Returns
/// Vector of matching products sorted by similarity score (descending)
pub fn find_matching_products_semantic(
    search_term: &str,
    products: &[Product],
    threshold: f64,
) -> Vec<Product> {
    if products.is_empty() {
        return Vec::new();
    }

    // Build batch: search term + all product texts
    let mut texts = vec![search_term.to_string()];
    texts.extend(products.iter().map(|p| p.to_embedding_text()));

    // Generate all embeddings in one batch
    let all_embeddings = match EmbeddingService::generate_batch(&texts) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let search_embedding = &all_embeddings[0];

    // Calculate similarity for each product
    let mut matches: Vec<Product> = products
        .iter()
        .enumerate()
        .filter_map(|(i, product)| {
            let product_embedding = &all_embeddings[i + 1];
            let similarity = cosine_similarity(search_embedding, product_embedding);

            if similarity >= threshold {
                let mut matched = product.clone();
                matched.similarity_score = similarity;
                Some(matched)
            } else {
                None
            }
        })
        .collect();

    // Sort by similarity score descending
    matches.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

    matches
}

/// Find the best semantic matches, sorted by price.
///
/// # Arguments
/// * `search_term` - The term to search for
/// * `products` - List of products to search through
/// * `threshold` - Minimum similarity score (0.0 to 1.0)
/// * `top_n` - Maximum number of results to return
///
/// # Returns
/// Vector of top N cheapest semantically matching products
pub fn find_best_matches_semantic(
    search_term: &str,
    products: &[Product],
    threshold: f64,
    top_n: usize,
) -> Vec<Product> {
    let mut matches = find_matching_products_semantic(search_term, products, threshold);

    // Sort by price ascending (cheapest first)
    matches.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

    matches.into_iter().take(top_n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_products() -> Vec<Product> {
        vec![
            Product {
                product_id: 1,
                product_name: "Anchor Butter 500g".to_string(),
                brand: "Anchor".to_string(),
                size_value: 0.5,
                size_unit: "Kilogram".to_string(),
                price: 6.99,
                supermarket: "PakNSave".to_string(),
                store_name: "Pak'n Save Albany".to_string(),
                store_id: "store1".to_string(),
                store_latitude: -36.7276,
                store_longitude: 174.7021,
                similarity_score: 0.0,
                image_url: "http://example.com".to_string(),
            },
            Product {
                product_id: 2,
                product_name: "Buttercup Pumpkin".to_string(),
                brand: "".to_string(),
                size_value: 1.0,
                size_unit: "Unit".to_string(),
                price: 3.99,
                supermarket: "NewWorld".to_string(),
                store_name: "New World Mt Eden".to_string(),
                store_id: "store2".to_string(),
                store_latitude: -36.8762,
                store_longitude: 174.7567,
                similarity_score: 0.0,
                image_url: "http://example.com".to_string(),
            },
            Product {
                product_id: 3,
                product_name: "Bread Wholemeal 700g".to_string(),
                brand: "Vogel's".to_string(),
                size_value: 0.7,
                size_unit: "Kilogram".to_string(),
                price: 4.50,
                supermarket: "PakNSave".to_string(),
                store_name: "Pak'n Save Albany".to_string(),
                store_id: "store1".to_string(),
                store_latitude: -36.7276,
                store_longitude: 174.7021,
                similarity_score: 0.0,
                image_url: "http://example.com".to_string(),
            },
        ]
    }

    #[test]
    fn test_semantic_butter_matching() {
        let products = sample_products();
        let matches = find_matching_products_semantic("butter", &products, 0.3);

        let butter_match = matches.iter().find(|m| m.product_name.contains("Anchor Butter"));
        assert!(butter_match.is_some(), "Should find Anchor Butter");
    }

    #[test]
    fn test_semantic_bread_matching() {
        let products = sample_products();
        let matches = find_matching_products_semantic("bread", &products, 0.3);

        let bread_match = matches.iter().find(|m| m.product_name.contains("Bread Wholemeal"));
        assert!(bread_match.is_some(), "Should find Bread Wholemeal");
    }

    #[test]
    fn test_product_searcher_trait() {
        let products = sample_products();
        let searcher = ProductSearcher::new(&products);
        let matches = searcher.find_matches("butter", 0.3);
        assert!(!matches.is_empty());
    }
}
