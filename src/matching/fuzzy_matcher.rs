use serde::Serialize;
use strsim::jaro_winkler;

/// A product match result with similarity score and store information.
#[derive(Debug, Clone, Serialize)]
pub struct ProductMatch {
    pub product_name: String,
    pub brand: String,
    pub price: f64,
    pub supermarket: String,
    pub store_name: String,
    pub store_id: String,
    pub store_latitude: f64,
    pub store_longitude: f64,
    pub similarity_score: f64,
}

/// Calculate a combined similarity score using both substring matching and fuzzy matching.
///
/// Returns a score between 0.0 and 1.0 where:
/// - 1.0 means exact match or search term is a substring of product name
/// - Lower values indicate partial fuzzy matches
fn calculate_similarity(search_term: &str, product_name: &str) -> f64 {
    let search_lower = search_term.to_lowercase();
    let name_lower = product_name.to_lowercase();

    // If search term is contained in product name, give high score
    if name_lower.contains(&search_lower) {
        // Score based on how much of the product name is the search term
        let ratio = search_lower.len() as f64 / name_lower.len() as f64;
        // Minimum 0.8 for substring matches, up to 1.0 for exact matches
        return 0.8 + (ratio * 0.2);
    }

    // Check each word in the product name for fuzzy matching
    let words: Vec<&str> = name_lower.split_whitespace().collect();
    let mut best_word_score = 0.0;

    for word in &words {
        let score = jaro_winkler(&search_lower, word);
        if score > best_word_score {
            best_word_score = score;
        }
    }

    // Also compare against the full name
    let full_name_score = jaro_winkler(&search_lower, &name_lower);

    // Return the best score
    best_word_score.max(full_name_score)
}

/// Find products that match the search term using fuzzy matching.
///
/// # Arguments
/// * `search_term` - The term to search for
/// * `products` - List of products to search through (name, brand, price, supermarket, store_name, store_id, lat, lon)
/// * `threshold` - Minimum similarity score (0.0 to 1.0, e.g., 0.6 = 60%)
///
/// # Returns
/// Vector of matching products sorted by similarity score (descending)
pub fn find_matching_products(
    search_term: &str,
    products: &[(String, String, f64, String, String, String, f64, f64)],
    threshold: f64,
) -> Vec<ProductMatch> {
    let mut matches: Vec<ProductMatch> = products
        .iter()
        .filter_map(|(name, brand, price, supermarket, store_name, store_id, lat, lon)| {
            let similarity = calculate_similarity(search_term, name);

            if similarity >= threshold {
                Some(ProductMatch {
                    product_name: name.clone(),
                    brand: brand.clone(),
                    price: *price,
                    supermarket: supermarket.clone(),
                    store_name: store_name.clone(),
                    store_id: store_id.clone(),
                    store_latitude: *lat,
                    store_longitude: *lon,
                    similarity_score: similarity,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by similarity score descending
    matches.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

    matches
}

/// Find the best matches for a search term, filtered and sorted by price.
///
/// # Arguments
/// * `search_term` - The term to search for
/// * `products` - List of products to search through
/// * `threshold` - Minimum similarity score (0.0 to 1.0)
/// * `top_n` - Maximum number of results to return
///
/// # Returns
/// Vector of top N cheapest matching products
pub fn find_best_matches(
    search_term: &str,
    products: &[(String, String, f64, String, String, String, f64, f64)],
    threshold: f64,
    top_n: usize,
) -> Vec<ProductMatch> {
    let mut matches = find_matching_products(search_term, products, threshold);

    // Sort by price ascending (cheapest first)
    matches.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

    matches.into_iter().take(top_n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_products() -> Vec<(String, String, f64, String, String, String, f64, f64)> {
        vec![
            ("Anchor Blue Milk 2L".to_string(), "Anchor".to_string(), 4.99, "PakNSave".to_string(), "Pak'n Save Albany".to_string(), "store1".to_string(), -36.7276, 174.7021),
            ("Meadow Fresh Milk 2L".to_string(), "Meadow Fresh".to_string(), 5.49, "NewWorld".to_string(), "New World Mt Eden".to_string(), "store2".to_string(), -36.8762, 174.7567),
            ("Anchor Lite Milk 1L".to_string(), "Anchor".to_string(), 3.29, "Woolworth".to_string(), "Countdown Auckland".to_string(), "store3".to_string(), -36.8485, 174.7633),
            ("Bread Wholemeal 700g".to_string(), "Vogel's".to_string(), 4.50, "PakNSave".to_string(), "Pak'n Save Albany".to_string(), "store1".to_string(), -36.7276, 174.7021),
        ]
    }

    #[test]
    fn test_find_matching_products_milk() {
        let products = sample_products();
        let matches = find_matching_products("milk", &products, 0.5);
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_find_matching_products_no_match() {
        let products = sample_products();
        let matches = find_matching_products("xyz123", &products, 0.8);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_best_matches_top_2() {
        let products = sample_products();
        let matches = find_best_matches("milk", &products, 0.5, 2);
        assert_eq!(matches.len(), 2);
        // Should be sorted by price
        assert!(matches[0].price <= matches[1].price);
    }
}
