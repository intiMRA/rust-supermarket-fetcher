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

/// Calculate a combined similarity score using whole-word matching and fuzzy matching.
///
/// Returns a score between 0.0 and 1.0 where:
/// - 1.0 means exact whole-word match
/// - 0.95 means a word starts with the search term (e.g., "milk" matches "milks")
/// - Lower values indicate fuzzy matches
///
/// Important: This uses whole-word matching to avoid false positives like
/// "bread" matching "Shortbread" or "butter" matching "Buttercup".
fn calculate_similarity(search_term: &str, product_name: &str) -> f64 {
    let search_lower = search_term.to_lowercase();
    let name_lower = product_name.to_lowercase();

    // Split product name into words
    let words: Vec<&str> = name_lower.split_whitespace().collect();

    // Check for exact whole-word match
    for word in &words {
        if *word == search_lower {
            return 1.0; // Exact word match
        }
    }

    // Check if any word starts with the search term
    // Only allow if the extra suffix is just "s" or "es" (plurals)
    for word in &words {
        if word.starts_with(&search_lower) && word.len() > search_lower.len() {
            let suffix = &word[search_lower.len()..];
            if suffix == "s" || suffix == "es" {
                // Plural form is acceptable (e.g., "milk" -> "milks")
                return 0.95;
            } else {
                // Other suffixes like "y", "cup", "bread" are likely different words
                // e.g., "butter" -> "buttery", "buttercup"
                return 0.4;
            }
        }
    }

    // Check if search term appears as a suffix of any word (e.g., "bread" in "shortbread")
    for word in &words {
        if word.ends_with(&search_lower) && word.len() > search_lower.len() {
            return 0.4;
        }
    }

    // Fall back to fuzzy matching on individual words
    let mut best_word_score = 0.0;
    for word in &words {
        let base_score = jaro_winkler(&search_lower, word);

        // Penalize if word is much longer than search term
        let length_ratio = search_lower.len() as f64 / word.len() as f64;
        let length_penalty = if length_ratio < 0.7 {
            length_ratio
        } else {
            1.0
        };

        let score = base_score * length_penalty;
        if score > best_word_score {
            best_word_score = score;
        }
    }

    // For full name comparison, only use if search term has multiple words
    let search_word_count = search_lower.split_whitespace().count();
    if search_word_count > 1 {
        let full_name_score = jaro_winkler(&search_lower, &name_lower);
        return best_word_score.max(full_name_score);
    }

    best_word_score
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
        .into_iter()
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

    #[test]
    fn test_whole_word_matching_bread() {
        // "bread" should NOT match "Shortbread" (substring within word)
        let score = calculate_similarity("bread", "Shortbread Cookies");
        assert!(score < 0.8, "bread should not match Shortbread, got score: {}", score);

        // "bread" SHOULD match "Bread Wholemeal"
        let score = calculate_similarity("bread", "Bread Wholemeal 700g");
        assert!(score >= 0.95, "bread should match Bread, got score: {}", score);
    }

    #[test]
    fn test_whole_word_matching_butter() {
        // "butter" should NOT match "Buttercup Pumpkin"
        let score = calculate_similarity("butter", "Buttercup Pumpkin");
        assert!(score < 0.8, "butter should not match Buttercup, got score: {}", score);

        // "butter" should NOT match "Buttery Popcorn"
        let score = calculate_similarity("butter", "Buttery Popcorn");
        assert!(score < 0.8, "butter should not match Buttery, got score: {}", score);

        // "butter" SHOULD match "Anchor Butter 500g"
        let score = calculate_similarity("butter", "Anchor Butter 500g");
        assert!(score >= 0.95, "butter should match Butter, got score: {}", score);
    }

    #[test]
    fn test_whole_word_matching_milk() {
        // "milk" should match "milks" (plural, close in length)
        let score = calculate_similarity("milk", "Fresh Milks 2L");
        assert!(score >= 0.9, "milk should match Milks, got score: {}", score);

        // "milk" should match exact word
        let score = calculate_similarity("milk", "Anchor Milk 2L");
        assert_eq!(score, 1.0, "milk should exactly match Milk");
    }
}
