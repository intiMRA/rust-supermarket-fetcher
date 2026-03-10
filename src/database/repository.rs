use std::collections::HashMap;

use rusqlite::params;

use super::Database;
use crate::matching::product_matcher::{
    bytes_to_f32_vec, cosine_similarity, f32_vec_to_bytes, generate_embeddings_batch,
};
use crate::supermarkets::models::store::Store;
use crate::supermarkets::models::super_market_item::SuperMarketItem;
use crate::supermarkets::supermarket_types::Supermarket;

/// Similarity threshold for semantic matching.
const SIMILARITY_THRESHOLD: f64 = 0.85;

/// Batch size for embedding generation (balance memory vs speed).
const EMBEDDING_BATCH_SIZE: usize = 1000;

/// Normalize brand name for consistent matching.
///
/// Removes punctuation, spaces, and converts to lowercase.
/// This ensures "Whittaker's" and "Whittakers" are treated as the same brand.
///
/// # Examples
/// - "Whittaker's" → "whittakers"
/// - "M&M's" → "mms"
/// - "Dr. Oetker" → "droetker"
fn normalize_brand(brand: &str) -> String {
    brand
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Key for in-memory product deduplication.
/// Products with the same key are considered the same product.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct ProductKey {
    pub name: String,
    pub brand: String,
    pub size_value_cents: i64, // Store as integer cents to avoid float hashing issues
    pub size_unit: String,
}

impl ProductKey {
    pub fn from_item(item: &SuperMarketItem) -> Self {
        let (size_value, size_unit) = item.size.to_normalized_value_and_unit();
        Self {
            name: item.name.to_lowercase(),
            brand: normalize_brand(&item.brand_name),
            size_value_cents: (size_value * 100.0).round() as i64,
            size_unit: size_unit.to_string(),
        }
    }

    /// Build text representation for embedding.
    pub fn to_embedding_text(&self) -> String {
        if self.brand.is_empty() {
            self.name.clone()
        } else {
            format!("{} {}", self.brand, self.name)
        }
    }
}

/// Item with its associated store for batch processing.
pub struct ItemWithStore<'a> {
    pub item: &'a SuperMarketItem,
    pub store: &'a Store,
    pub supermarket: Supermarket,
}

/// Repository for database operations.
///
/// Provides methods to insert and query data using the deduplicated schema.
pub struct Repository<'a> {
    db: &'a Database,
}

impl<'a> Repository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    // -------------------------------------------------------------------------
    // Helper: Get supermarket ID from enum
    // -------------------------------------------------------------------------

    fn supermarket_id(supermarket: Supermarket) -> i32 {
        match supermarket {
            Supermarket::NewWorld => 1,
            Supermarket::PakNSave => 2,
            Supermarket::Woolworth => 3,
        }
    }

    /// Get supermarket name string for storing in product_variants.
    fn supermarket_name(supermarket: Supermarket) -> &'static str {
        match supermarket {
            Supermarket::NewWorld => "NewWorld",
            Supermarket::PakNSave => "PakNSave",
            Supermarket::Woolworth => "Woolworth",
        }
    }

    // -------------------------------------------------------------------------
    // Stores
    // -------------------------------------------------------------------------

    /// Insert a store into the database.
    ///
    /// Uses `INSERT OR IGNORE` to skip if the store already exists.
    pub fn insert_store(&self, store: &Store, supermarket: Supermarket) -> rusqlite::Result<()> {
        self.db.conn.execute(
            "INSERT OR IGNORE INTO stores (id, supermarket_id, name, address, latitude, longitude)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                store.id,
                Self::supermarket_id(supermarket),
                store.name,
                store.address,
                store.latitude,
                store.longitude,
            ],
        )?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Categories (with caching)
    // -------------------------------------------------------------------------

    /// Insert a category and return its ID, using a cache to avoid repeated lookups.
    fn insert_category_cached(
        &self,
        display_name: &str,
        slug: &str,
        supermarket: Supermarket,
        cache: &mut HashMap<(String, Supermarket), i64>,
    ) -> rusqlite::Result<i64> {
        let cache_key = (display_name.to_string(), supermarket);

        if let Some(&id) = cache.get(&cache_key) {
            return Ok(id);
        }

        let supermarket_id = Self::supermarket_id(supermarket);

        self.db.conn.execute(
            "INSERT OR IGNORE INTO categories (display_name, slug, supermarket_id)
             VALUES (?1, ?2, ?3)",
            params![display_name, slug, supermarket_id],
        )?;

        let id: i64 = self.db.conn.query_row(
            "SELECT id FROM categories WHERE display_name = ?1 AND supermarket_id = ?2",
            params![display_name, supermarket_id],
            |row| row.get(0),
        )?;

        cache.insert(cache_key, id);
        Ok(id)
    }

    // -------------------------------------------------------------------------
    // Product Variants
    // -------------------------------------------------------------------------

    /// Insert a product variant and return its ID.
    pub fn insert_variant(
        &self,
        item: &SuperMarketItem,
        product_id: i64,
        category_id: i64,
    ) -> rusqlite::Result<i64> {
        let supermarket_name = Self::supermarket_name(item.supermarket);

        self.db.conn.execute(
            "INSERT OR REPLACE INTO product_variants
             (product_id, external_id, original_name, image_url, category_id, supermarket)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                product_id,
                item.id,
                item.name,
                item.image_url,
                category_id,
                supermarket_name,
            ],
        )?;

        let variant_id: i64 = self.db.conn.query_row(
            "SELECT id FROM product_variants WHERE external_id = ?1 AND supermarket = ?2",
            params![item.id, supermarket_name],
            |row| row.get(0),
        )?;

        Ok(variant_id)
    }

    // -------------------------------------------------------------------------
    // Prices
    // -------------------------------------------------------------------------

    /// Upsert a price record for a variant at a specific store.
    pub fn insert_price(&self, variant_id: i64, store_id: &str, price: f64) -> rusqlite::Result<()> {
        self.db.conn.execute(
            "DELETE FROM prices
             WHERE variant_id = ?1
             AND store_id = ?2
             AND fetched_at >= DATE('now', '-5 days')",
            params![variant_id, store_id],
        )?;

        self.db.conn.execute(
            "INSERT INTO prices (variant_id, store_id, price, fetched_at)
             VALUES (?1, ?2, ?3, DATE('now'))",
            params![variant_id, store_id, price],
        )?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Product matching helpers
    // -------------------------------------------------------------------------

    /// Find exact match on normalized fields.
    fn find_exact_match(
        &self,
        name: &str,
        brand: &str,
        size_value: f64,
        size_unit: &str,
    ) -> rusqlite::Result<Option<i64>> {
        let mut stmt = self.db.conn.prepare_cached(
            "SELECT id FROM products
             WHERE name = ?1 AND brand = ?2
             AND size_value = ?3 AND size_unit = ?4",
        )?;

        match stmt.query_row(params![name, brand, size_value, size_unit], |row| row.get(0)) {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get candidate products with the same brand for semantic matching.
    fn get_candidates_by_brand(&self, brand: &str) -> rusqlite::Result<Vec<(i64, f64, String, Vec<f32>)>> {
        let mut stmt = self.db.conn.prepare_cached(
            "SELECT id, size_value, size_unit, embedding FROM products WHERE brand = ?1",
        )?;

        let rows = stmt.query_map(params![brand], |row| {
            let embedding_bytes: Vec<u8> = row.get(3)?;
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, String>(2)?,
                bytes_to_f32_vec(&embedding_bytes),
            ))
        })?;

        rows.collect()
    }

    /// Create a new product in the database.
    fn create_product(
        &self,
        item: &SuperMarketItem,
        embedding: &[f32],
    ) -> rusqlite::Result<i64> {
        let (size_value, size_unit) = item.size.to_normalized_value_and_unit();
        let embedding_bytes = f32_vec_to_bytes(embedding);
        let normalized_brand = normalize_brand(&item.brand_name);

        self.db.conn.execute(
            "INSERT INTO products (name, brand, size_value, size_unit, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![item.name, normalized_brand, size_value, size_unit, embedding_bytes],
        )?;

        let product_id = self.db.conn.last_insert_rowid();

        // Sync to FTS index
        self.db.conn.execute(
            "INSERT OR REPLACE INTO products_fts (rowid, name, brand) VALUES (?1, ?2, ?3)",
            params![product_id, item.name, item.brand_name],
        )?;

        Ok(product_id)
    }

    /// Check if sizes are compatible (same unit, within 1% tolerance).
    fn sizes_compatible(a_value: f64, a_unit: &str, b_value: f64, b_unit: &str) -> bool {
        if a_unit != b_unit {
            return false;
        }
        if b_value == 0.0 && a_value == 0.0 {
            return true;
        }
        if b_value == 0.0 || a_value == 0.0 {
            return false;
        }
        let ratio = a_value / b_value;
        (0.99..=1.01).contains(&ratio)
    }

    // -------------------------------------------------------------------------
    // High-level: Batch insert with optimized deduplication
    // -------------------------------------------------------------------------

    /// Insert all items with optimized batch processing.
    ///
    /// Optimizations:
    /// 1. In-memory deduplication: Group by (name, brand, size)
    /// 2. Batch embeddings: Generate all embeddings in one call
    /// 3. Category caching: Avoid repeated category lookups
    pub fn insert_all_items(&self, items_with_stores: &[ItemWithStore<'_>]) -> rusqlite::Result<()> {
        if items_with_stores.is_empty() {
            return Ok(());
        }

        // Start transaction
        self.db.conn.execute("BEGIN TRANSACTION", [])?;

        // PHASE 1: Insert all stores
        println!("Phase 1: Inserting stores...");
        let mut seen_stores = std::collections::HashSet::new();
        for iws in items_with_stores {
            let store_key = (&iws.store.id, iws.supermarket);
            if !seen_stores.contains(&store_key) {
                if let Err(e) = self.insert_store(iws.store, iws.supermarket) {
                    eprintln!("Warning: Failed to insert store '{}': {}", iws.store.name, e);
                }
                seen_stores.insert(store_key);
            }
        }
        println!("  Inserted {} stores", seen_stores.len());

        // PHASE 2: Group items by product key (in-memory deduplication)
        println!("Phase 2: In-memory deduplication...");
        let mut product_groups: HashMap<ProductKey, Vec<&ItemWithStore<'_>>> = HashMap::new();
        for iws in items_with_stores {
            let key = ProductKey::from_item(iws.item);
            product_groups.entry(key).or_default().push(iws);
        }
        println!(
            "  {} items -> {} unique products",
            items_with_stores.len(),
            product_groups.len()
        );

        // PHASE 3: Separate products needing embeddings from exact matches
        println!("Phase 3: Finding exact matches...");
        let mut product_id_cache: HashMap<ProductKey, i64> = HashMap::new();
        let mut needs_embedding: Vec<(ProductKey, &SuperMarketItem)> = Vec::new();
        let mut exact_matches = 0;

        for (key, group) in &product_groups {
            let item = group[0].item;
            let (size_value, size_unit) = item.size.to_normalized_value_and_unit();

            // Try exact match first (no embedding needed)
            let normalized_brand = normalize_brand(&item.brand_name);
            match self.find_exact_match(&item.name, &normalized_brand, size_value, size_unit) {
                Ok(Some(id)) => {
                    product_id_cache.insert(key.clone(), id);
                    exact_matches += 1;
                }
                Ok(None) => {
                    needs_embedding.push((key.clone(), item));
                }
                Err(e) => {
                    eprintln!("Warning: Exact match query failed: {}", e);
                    needs_embedding.push((key.clone(), item));
                }
            }
        }
        println!("  {} exact matches, {} need embeddings", exact_matches, needs_embedding.len());

        // PHASE 4: Batch generate embeddings for products that need them
        if !needs_embedding.is_empty() {
            println!("Phase 4: Generating embeddings in batches...");
            let mut semantic_matches = 0;
            let mut new_products = 0;

            // Process in batches to manage memory
            for (batch_idx, batch) in needs_embedding.chunks(EMBEDDING_BATCH_SIZE).enumerate() {
                let texts: Vec<String> = batch.iter().map(|(k, _)| k.to_embedding_text()).collect();

                print!("  Batch {}: generating {} embeddings... ", batch_idx + 1, texts.len());

                let embeddings = match generate_embeddings_batch(&texts) {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("Failed: {}", e);
                        continue;
                    }
                };

                println!("done, matching...");

                // Try semantic matching for each, or create new product
                for ((key, item), embedding) in batch.iter().zip(embeddings.iter()) {
                    let (size_value, size_unit) = item.size.to_normalized_value_and_unit();
                    let normalized_brand = normalize_brand(&item.brand_name);

                    // Try semantic match against existing products with same brand
                    let mut matched = false;
                    if let Ok(candidates) = self.get_candidates_by_brand(&normalized_brand) {
                        for (cand_id, cand_size_val, cand_size_unit, cand_embedding) in candidates {
                            if !Self::sizes_compatible(size_value, size_unit, cand_size_val, &cand_size_unit) {
                                continue;
                            }
                            let similarity = cosine_similarity(embedding, &cand_embedding);
                            if similarity >= SIMILARITY_THRESHOLD {
                                product_id_cache.insert(key.clone(), cand_id);
                                semantic_matches += 1;
                                matched = true;
                                break;
                            }
                        }
                    }

                    // Create new product if no match
                    if !matched {
                        match self.create_product(item, embedding) {
                            Ok(id) => {
                                product_id_cache.insert(key.clone(), id);
                                new_products += 1;
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to create product '{}': {}", item.name, e);
                            }
                        }
                    }
                }
            }

            println!(
                "  {} semantic matches, {} new products",
                semantic_matches, new_products
            );
        }

        // PHASE 5: Insert variants and prices with category caching
        println!("Phase 5: Inserting variants and prices...");
        let mut category_cache: HashMap<(String, Supermarket), i64> = HashMap::new();
        let mut variants_inserted = 0;
        let mut prices_inserted = 0;

        for iws in items_with_stores {
            let key = ProductKey::from_item(iws.item);

            let product_id = match product_id_cache.get(&key) {
                Some(id) => *id,
                None => continue,
            };

            // Insert category (cached)
            let category_id = match self.insert_category_cached(
                &iws.item.category.display_name,
                &iws.item.category.slug,
                iws.supermarket,
                &mut category_cache,
            ) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Warning: Failed to insert category: {}", e);
                    continue;
                }
            };

            // Insert variant
            let variant_id = match self.insert_variant(iws.item, product_id, category_id) {
                Ok(id) => {
                    variants_inserted += 1;
                    id
                }
                Err(e) => {
                    eprintln!("Warning: Failed to insert variant '{}': {}", iws.item.name, e);
                    continue;
                }
            };

            // Insert price
            if let Err(e) = self.insert_price(variant_id, &iws.store.id, iws.item.price) {
                eprintln!("Warning: Failed to insert price: {}", e);
            } else {
                prices_inserted += 1;
            }
        }

        println!(
            "  {} variants, {} prices (cached {} categories)",
            variants_inserted, prices_inserted, category_cache.len()
        );

        // Commit transaction
        self.db.conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Insert all items for a store (legacy method, wraps insert_all_items).
    pub fn insert_items_for_store(
        &self,
        store: &Store,
        supermarket: Supermarket,
        items: &[SuperMarketItem],
    ) -> rusqlite::Result<()> {
        let items_with_stores: Vec<ItemWithStore<'_>> = items
            .iter()
            .map(|item| ItemWithStore {
                item,
                store,
                supermarket,
            })
            .collect();

        self.insert_all_items(&items_with_stores)
    }

    // -------------------------------------------------------------------------
    // Statistics
    // -------------------------------------------------------------------------

    /// Get deduplication statistics.
    pub fn get_deduplication_stats(&self) -> rusqlite::Result<DeduplicationStats> {
        let unique_products: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))?;

        let variants: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM product_variants", [], |row| row.get(0))?;

        let prices: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM prices", [], |row| row.get(0))?;

        Ok(DeduplicationStats {
            unique_products,
            variants,
            prices,
            deduplication_ratio: if variants > 0 {
                variants as f64 / unique_products as f64
            } else {
                1.0
            },
        })
    }
}

/// Statistics about product deduplication.
#[derive(Debug)]
pub struct DeduplicationStats {
    pub unique_products: i64,
    pub variants: i64,
    pub prices: i64,
    pub deduplication_ratio: f64,
}

impl std::fmt::Display for DeduplicationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unique products: {}, Variants: {}, Prices: {}, Ratio: {:.2}x",
            self.unique_products, self.variants, self.prices, self.deduplication_ratio
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_brand_removes_apostrophes() {
        assert_eq!(normalize_brand("Whittaker's"), "whittakers");
        assert_eq!(normalize_brand("M&M's"), "mms");
    }

    #[test]
    fn test_normalize_brand_removes_spaces_and_punctuation() {
        assert_eq!(normalize_brand("Dr. Oetker"), "droetker");
        assert_eq!(normalize_brand("Tip-Top"), "tiptop");
        assert_eq!(normalize_brand("Ben & Jerry's"), "benjerrys");
    }

    #[test]
    fn test_normalize_brand_lowercases() {
        assert_eq!(normalize_brand("ANCHOR"), "anchor");
        assert_eq!(normalize_brand("Anchor"), "anchor");
        assert_eq!(normalize_brand("anchor"), "anchor");
    }

    #[test]
    fn test_normalize_brand_empty_string() {
        assert_eq!(normalize_brand(""), "");
    }
}
