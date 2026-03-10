use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::sync::{Mutex, OnceLock};

use super::fuzzy_matcher::Product;

/// Global embedding model - initialized once, reused across calls.
/// Using OnceLock for lazy initialization and Mutex for mutable access.
static EMBEDDING_MODEL: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

/// Get the embedding model, initializing it if needed.
/// Downloads the model on first use (~90MB for AllMiniLML6V2).
fn get_model() -> &'static Mutex<TextEmbedding> {
    EMBEDDING_MODEL.get_or_init(|| {
        println!("Initializing embedding model (first use only)...");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true),
        )
        .expect("Failed to initialize embedding model");
        Mutex::new(model)
    })
}

/// Calculate cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot_product / (norm_a * norm_b)) as f64
}

/// Find products that semantically match the search term using embeddings.
///
/// This uses sentence transformers to understand semantic meaning.
/// Category-based filtering should be done before calling this function
/// to ensure products are from the correct category.
///
/// # Arguments
/// * `search_term` - The term to search for
/// * `products` - List of products to search through (should be pre-filtered by category)
/// * `threshold` - Minimum similarity score (0.0 to 1.0, recommended: 0.3-0.5 for embeddings)
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

    let model_mutex = get_model();
    let mut model = model_mutex.lock().expect("Failed to lock model");

    // Prepare texts for embedding
    let search_text = search_term.to_string();
    let product_texts: Vec<String> = products
        .iter()
        .map(|p| {
            if p.brand.is_empty() {
                p.product_name.clone()
            } else {
                format!("{} {}", p.brand, p.product_name)
            }
        })
        .collect();

    // Create batch with search term first, then all products
    let mut all_texts = vec![search_text];
    all_texts.extend(product_texts);

    // Generate all embeddings in one batch for efficiency
    let texts_for_embedding: Vec<&str> = all_texts.iter().map(|s| s.as_str()).collect();
    let all_embeddings = model
        .embed(texts_for_embedding, None)
        .expect("Failed to generate embeddings");

    // Release the lock before processing results
    drop(model);

    // First embedding is the search term
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

/// Find the best semantic matches for a search term, sorted by price.
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
                product_name: "Anchor Butter 500g".to_string(),
                brand: "Anchor".to_string(),
                price: 6.99,
                supermarket: "PakNSave".to_string(),
                store_name: "Pak'n Save Albany".to_string(),
                store_id: "store1".to_string(),
                store_latitude: -36.7276,
                store_longitude: 174.7021,
                similarity_score: 0.0,
            },
            Product {
                product_name: "Buttercup Pumpkin".to_string(),
                brand: "".to_string(),
                price: 3.99,
                supermarket: "NewWorld".to_string(),
                store_name: "New World Mt Eden".to_string(),
                store_id: "store2".to_string(),
                store_latitude: -36.8762,
                store_longitude: 174.7567,
                similarity_score: 0.0,
            },
            Product {
                product_name: "Butter Chicken Spring Rolls".to_string(),
                brand: "Machi".to_string(),
                price: 5.99,
                supermarket: "Woolworth".to_string(),
                store_name: "Countdown Auckland".to_string(),
                store_id: "store3".to_string(),
                store_latitude: -36.8485,
                store_longitude: 174.7633,
                similarity_score: 0.0,
            },
            Product {
                product_name: "Bread Wholemeal 700g".to_string(),
                brand: "Vogel's".to_string(),
                price: 4.50,
                supermarket: "PakNSave".to_string(),
                store_name: "Pak'n Save Albany".to_string(),
                store_id: "store1".to_string(),
                store_latitude: -36.7276,
                store_longitude: 174.7021,
                similarity_score: 0.0,
            },
            Product {
                product_name: "Shortbread Cookies".to_string(),
                brand: "Arnotts".to_string(),
                price: 4.99,
                supermarket: "Woolworth".to_string(),
                store_name: "Countdown Auckland".to_string(),
                store_id: "store3".to_string(),
                store_latitude: -36.8485,
                store_longitude: 174.7633,
                similarity_score: 0.0,
            },
        ]
    }

    #[test]
    fn test_semantic_butter_matching() {
        let products = sample_products();
        let matches = find_matching_products_semantic("butter", &products, 0.3);

        // Should match "Anchor Butter 500g" with high score
        let butter_match = matches.iter().find(|m| m.product_name.contains("Anchor Butter"));
        assert!(butter_match.is_some(), "Should find Anchor Butter");
    }

    #[test]
    fn test_semantic_bread_matching() {
        let products = sample_products();
        let matches = find_matching_products_semantic("bread", &products, 0.3);

        // Should match "Bread Wholemeal 700g" with high score
        let bread_match = matches.iter().find(|m| m.product_name.contains("Bread Wholemeal"));
        assert!(bread_match.is_some(), "Should find Bread Wholemeal");
    }
}
