use serde::Serialize;
use strsim::jaro_winkler;

/// A product match result with similarity score and store information.
#[derive(Debug, Clone, Serialize)]
pub struct Product {
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
/// * `products` - List of products to search through
/// * `threshold` - Minimum similarity score (0.0 to 1.0, e.g., 0.6 = 60%)
///
/// # Returns
/// Vector of matching products sorted by similarity score (descending)
pub fn find_matching_products(
    search_term: &str,
    products: &[Product],
    threshold: f64,
) -> Vec<Product> {
    let mut matches: Vec<Product> = products
        .iter()
        .filter_map(|product| {
            let similarity = calculate_similarity(search_term, product.product_name.as_str());

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
    products: &[Product],
    threshold: f64,
    top_n: usize,
) -> Vec<Product> {
    let mut matches = find_matching_products(search_term, products, threshold);

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
                product_name: "Anchor Blue Milk 2L".to_string(),
                brand: "Anchor".to_string(),
                price: 4.99,
                supermarket: "PakNSave".to_string(),
                store_name: "Pak'n Save Albany".to_string(),
                store_id: "store1".to_string(),
                store_latitude: -36.7276,
                store_longitude: 174.7021,
                similarity_score: 0.0,
            },
            Product {
                product_name: "Meadow Fresh Milk 2L".to_string(),
                brand: "Meadow Fresh".to_string(),
                price: 5.49,
                supermarket: "NewWorld".to_string(),
                store_name: "New World Mt Eden".to_string(),
                store_id: "store2".to_string(),
                store_latitude: -36.8762,
                store_longitude: 174.7567,
                similarity_score: 0.0,
            },
            Product {
                product_name: "Anchor Lite Milk 1L".to_string(),
                brand: "Anchor".to_string(),
                price: 3.29,
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
