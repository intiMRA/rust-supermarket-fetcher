use rusqlite::params;

use super::Database;

/// Result type for product search with price and store information.
#[derive(Debug, Clone)]
pub struct ProductWithPriceAndStore {
    pub product_id: i32,
    pub product_name: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub price: f64,
    pub supermarket: String,
    pub supermarket_id: i32,
    pub store_name: String,
    pub store_id: String,
    pub store_latitude: f64,
    pub store_longitude: f64,
}

/// Store information from the database.
#[derive(Debug, Clone)]
pub struct StoreInfo {
    pub id: String,
    pub name: String,
    pub supermarket_id: i32,
    pub supermarket_name: String,
    pub latitude: f64,
    pub longitude: f64,
}

/// Query helper for running common database queries.
pub struct Queries<'a> {
    db: &'a Database,
}

impl<'a> Queries<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    // -------------------------------------------------------------------------
    // Product Search
    // -------------------------------------------------------------------------

    /// Search for products by name (case-insensitive partial match).
    pub fn search_products(&self, search_term: &str) -> Vec<ProductResult> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT p.name, v.supermarket, p.brand, p.size_value, p.size_unit, c.display_name
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             LEFT JOIN categories c ON v.category_id = c.id
             WHERE p.name LIKE ?1
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             GROUP BY p.id
             ORDER BY p.name
             LIMIT 50",
            )
            .unwrap();

        let pattern = format!("%{}%", search_term);
        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok(ProductResult {
                    name: row.get(0)?,
                    supermarket: row.get(1)?,
                    brand: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    size_value: row.get(3)?,
                    size_unit: row.get(4)?,
                    category: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Price Comparison
    // -------------------------------------------------------------------------

    /// Get price range for a product across all stores.
    pub fn get_price_range(&self, product_name: &str) -> Vec<PriceRangeResult> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT p.name, v.supermarket, MIN(pr.price), MAX(pr.price), COUNT(DISTINCT pr.store_id)
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             JOIN prices pr ON v.id = pr.variant_id
             WHERE p.name LIKE ?1
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             GROUP BY p.name, v.supermarket
             ORDER BY MIN(pr.price)",
            )
            .unwrap();

        let pattern = format!("%{}%", product_name);
        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok(PriceRangeResult {
                    product_name: row.get(0)?,
                    supermarket: row.get(1)?,
                    min_price: row.get(2)?,
                    max_price: row.get(3)?,
                    store_count: row.get(4)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Find the cheapest stores for a specific product.
    pub fn find_cheapest_stores(&self, product_name: &str, limit: u32) -> Vec<StorePrice> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT p.name, v.supermarket, st.name, st.address, pr.price
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             JOIN prices pr ON v.id = pr.variant_id
             JOIN stores st ON pr.store_id = st.id
             WHERE p.name LIKE ?1
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             ORDER BY pr.price ASC
             LIMIT ?2",
            )
            .unwrap();

        let pattern = format!("%{}%", product_name);
        let rows = stmt
            .query_map(params![pattern, limit], |row| {
                Ok(StorePrice {
                    product_name: row.get(0)?,
                    supermarket: row.get(1)?,
                    store_name: row.get(2)?,
                    store_address: row.get(3)?,
                    price: row.get(4)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get all prices for a single deduplicated product across all stores.
    pub fn get_prices_for_product(&self, product_id: i64) -> Vec<ProductPriceInfo> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT v.supermarket, st.name, st.id, pr.price, pr.fetched_at,
                    COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM product_variants v
             JOIN prices pr ON v.id = pr.variant_id
             JOIN stores st ON pr.store_id = st.id
             WHERE v.product_id = ?1
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.variant_id = pr.variant_id AND pr2.store_id = pr.store_id
             )
             ORDER BY pr.price ASC",
            )
            .unwrap();

        let rows = stmt
            .query_map(params![product_id], |row| {
                Ok(ProductPriceInfo {
                    supermarket: row.get(0)?,
                    store_name: row.get(1)?,
                    store_id: row.get(2)?,
                    price: row.get(3)?,
                    fetched_at: row.get(4)?,
                    store_latitude: row.get(5)?,
                    store_longitude: row.get(6)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Statistics
    // -------------------------------------------------------------------------

    /// Get database statistics.
    pub fn get_stats(&self) -> DatabaseStats {
        let product_count: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .unwrap_or(0);

        let variant_count: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM product_variants", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let price_count: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM prices", [], |row| row.get(0))
            .unwrap_or(0);

        let store_count: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM stores", [], |row| row.get(0))
            .unwrap_or(0);

        let category_count: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
            .unwrap_or(0);

        DatabaseStats {
            products: product_count,
            variants: variant_count,
            prices: price_count,
            stores: store_count,
            categories: category_count,
        }
    }

    /// Get product count per supermarket.
    pub fn get_products_per_supermarket(&self) -> Vec<(String, i64)> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT v.supermarket, COUNT(DISTINCT v.product_id)
             FROM product_variants v
             WHERE v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             GROUP BY v.supermarket
             ORDER BY COUNT(*) DESC",
            )
            .unwrap();

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Category Queries
    // -------------------------------------------------------------------------

    /// Find category IDs where the search term matches the category slug.
    ///
    /// Matching strategy (in priority order):
    /// 1. Exact slug match: "butter" → "Butter"
    /// 2. "Fresh X" pattern: "milk" → "Fresh Milk" (base product)
    /// 3. Slug ends with search term: "milk" → "UHT Milk"
    ///
    /// Returns only the highest priority matches found.
    pub fn find_matching_category_ids(&self, search_term: &str) -> Vec<i64> {
        let search_lower = search_term.to_lowercase();

        // Priority 1: Exact slug match
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT DISTINCT id FROM categories WHERE LOWER(slug) = ?1")
            .unwrap();

        let rows = stmt
            .query_map(params![search_lower], |row| row.get(0))
            .unwrap();
        let exact_matches: Vec<i64> = rows.filter_map(|r| r.ok()).collect();

        if !exact_matches.is_empty() {
            return exact_matches;
        }

        // Priority 2: "Fresh X" pattern
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT DISTINCT id FROM categories WHERE LOWER(slug) = ?1")
            .unwrap();

        let fresh_pattern = format!("fresh {}", search_lower);
        let rows = stmt
            .query_map(params![fresh_pattern], |row| row.get(0))
            .unwrap();
        let fresh_matches: Vec<i64> = rows.filter_map(|r| r.ok()).collect();

        if !fresh_matches.is_empty() {
            return fresh_matches;
        }

        // Priority 3: Slug ends with search term
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT DISTINCT id FROM categories WHERE LOWER(slug) LIKE ?1")
            .unwrap();

        let pattern_ends = format!("% {}", search_lower);
        let rows = stmt
            .query_map(params![pattern_ends], |row| row.get(0))
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Search for products in specific categories, filtered by store IDs.
    pub fn search_products_in_categories_and_stores(
        &self,
        category_ids: &[i64],
        store_ids: &[String],
    ) -> Vec<ProductWithPriceAndStore> {
        if category_ids.is_empty() || store_ids.is_empty() {
            return Vec::new();
        }

        // Build IN clause for category IDs
        let category_placeholders: Vec<String> =
            category_ids.iter().map(|id| id.to_string()).collect();
        let category_in_clause = category_placeholders.join(", ");

        // Build IN clause for store IDs
        let store_placeholders: Vec<String> = store_ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();
        let store_in_clause = store_placeholders.join(", ");

        let query = format!(
            "SELECT p.id, p.name, COALESCE(p.brand, ''), COALESCE(p.size_value, 0.0), COALESCE(p.size_unit, ''),
                    pr.price, v.supermarket, st.id, s.id,
                    st.name, COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             JOIN supermarkets s ON v.supermarket = s.name
             JOIN prices pr ON v.id = pr.variant_id
             JOIN stores st ON pr.store_id = st.id
             WHERE v.category_id IN ({})
             AND st.id IN ({})
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.variant_id = pr.variant_id AND pr2.store_id = pr.store_id
             )
             ORDER BY pr.price ASC
             LIMIT 500",
            category_in_clause, store_in_clause
        );

        let mut stmt = self.db.conn.prepare(&query).unwrap();

        let rows = stmt
            .query_map([], |row| {
                Ok(ProductWithPriceAndStore {
                    product_id: row.get(0)?,
                    product_name: row.get(1)?,
                    brand: row.get(2)?,
                    size_value: row.get(3)?,
                    size_unit: row.get(4)?,
                    price: row.get(5)?,
                    supermarket: row.get(6)?,
                    store_id: row.get(7)?,
                    supermarket_id: row.get(8)?,
                    store_name: row.get(9)?,
                    store_latitude: row.get(10)?,
                    store_longitude: row.get(11)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Search for products in specific categories, filtered by store IDs.
    pub fn get_paginated_products(
        &self,
        store_ids: &[String],
        page_number: i32,
        items_per_page: i32,
    ) -> Vec<ProductWithPriceAndStore> {
        if store_ids.is_empty() {
            return Vec::new();
        }
        let placeholders: Vec<String> = store_ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT p.id, p.name, COALESCE(p.brand, ''), COALESCE(p.size_value, 0.0), COALESCE(p.size_unit, ''),
                    pr.price, v.supermarket, st.id, s.id,
                    st.name, COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             JOIN supermarkets s ON v.supermarket = s.name
             JOIN prices pr ON v.id = pr.variant_id
             JOIN stores st ON pr.store_id = st.id
             WHERE st.id IN ({})
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.variant_id = pr.variant_id AND pr2.store_id = pr.store_id
             )
             ORDER BY p.name ASC
             LIMIT {} OFFSET {}",
            placeholders.join(", "),
            items_per_page,
            page_number * items_per_page
        );
        let mut stmt = self.db.conn.prepare(&query).unwrap();
        let params: Vec<&dyn rusqlite::ToSql> = store_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok(ProductWithPriceAndStore {
                    product_id: row.get(0)?,
                    product_name: row.get(1)?,
                    brand: row.get(2)?,
                    size_value: row.get(3)?,
                    size_unit: row.get(4)?,
                    price: row.get(5)?,
                    supermarket: row.get(6)?,
                    store_id: row.get(7)?,
                    supermarket_id: row.get(8)?,
                    store_name: row.get(9)?,
                    store_latitude: row.get(10)?,
                    store_longitude: row.get(11)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get products by category.
    pub fn get_products_by_category(&self, category: &str, limit: u32) -> Vec<ProductResult> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT p.name, v.supermarket, p.brand, p.size_value, p.size_unit, c.display_name
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             LEFT JOIN categories c ON v.category_id = c.id
             WHERE c.display_name LIKE ?1
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             GROUP BY p.id
             ORDER BY p.name
             LIMIT ?2",
            )
            .unwrap();

        let pattern = format!("%{}%", category);
        let rows = stmt
            .query_map(params![pattern, limit], |row| {
                Ok(ProductResult {
                    name: row.get(0)?,
                    supermarket: row.get(1)?,
                    brand: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    size_value: row.get(3)?,
                    size_unit: row.get(4)?,
                    category: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Brand Queries
    // -------------------------------------------------------------------------

    /// Get all products from a specific brand.
    pub fn get_products_by_brand(&self, brand: &str) -> Vec<ProductWithPrice> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT p.name, v.supermarket, p.brand, MIN(pr.price), MAX(pr.price)
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             JOIN prices pr ON v.id = pr.variant_id
             WHERE p.brand LIKE ?1
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             GROUP BY p.id
             ORDER BY p.name
             LIMIT 50",
            )
            .unwrap();

        let pattern = format!("%{}%", brand);
        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok(ProductWithPrice {
                    name: row.get(0)?,
                    supermarket: row.get(1)?,
                    brand: row.get(2)?,
                    min_price: row.get(3)?,
                    max_price: row.get(4)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Store Queries
    // -------------------------------------------------------------------------

    /// Get all stores with their location information.
    pub fn get_all_stores(&self) -> Vec<StoreInfo> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT st.id, st.name, st.supermarket_id, s.name,
                    COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM stores st
             JOIN supermarkets s ON st.supermarket_id = s.id",
            )
            .unwrap();

        let rows = stmt
            .query_map([], |row| {
                Ok(StoreInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    supermarket_id: row.get(2)?,
                    supermarket_name: row.get(3)?,
                    latitude: row.get(4)?,
                    longitude: row.get(5)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get stores for a specific supermarket.
    pub fn get_stores_by_supermarket(&self, supermarket_id: i32) -> Vec<StoreInfo> {
        let mut stmt = self
            .db
            .conn
            .prepare(
                "SELECT st.id, st.name, st.supermarket_id, s.name,
                    COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM stores st
             JOIN supermarkets s ON st.supermarket_id = s.id
             WHERE st.supermarket_id = ?1",
            )
            .unwrap();

        let rows = stmt
            .query_map(params![supermarket_id], |row| {
                Ok(StoreInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    supermarket_id: row.get(2)?,
                    supermarket_name: row.get(3)?,
                    latitude: row.get(4)?,
                    longitude: row.get(5)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Shopping List Query
    // -------------------------------------------------------------------------

    /// Search for products with their prices, filtered by store IDs.
    pub fn search_products_in_stores(
        &self,
        search_term: &str,
        store_ids: &[String],
    ) -> Vec<ProductWithPriceAndStore> {
        if store_ids.is_empty() {
            return Vec::new();
        }

        // Split search term into words
        let words: Vec<&str> = search_term.split_whitespace().collect();
        if words.is_empty() {
            return Vec::new();
        }

        // Build WHERE clause for search terms
        let word_conditions: Vec<String> = words
            .iter()
            .map(|w| format!("(p.name LIKE '%{}%' OR p.brand LIKE '%{}%')", w, w))
            .collect();
        let search_clause = word_conditions.join(" AND ");

        // Build IN clause for store IDs
        let store_placeholders: Vec<String> = store_ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();
        let store_in_clause = store_placeholders.join(", ");

        let query = format!(
            "SELECT p.id, p.name, COALESCE(p.brand, ''), COALESCE(p.size_value, 0.0), COALESCE(p.size_unit, ''),
                    pr.price, v.supermarket, st.id, s.id,
                    st.name, COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             JOIN supermarkets s ON v.supermarket = s.name
             JOIN prices pr ON v.id = pr.variant_id
             JOIN stores st ON pr.store_id = st.id
             WHERE ({})
             AND st.id IN ({})
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.variant_id = pr.variant_id AND pr2.store_id = pr.store_id
             )
             ORDER BY pr.price ASC
             LIMIT 500",
            search_clause, store_in_clause
        );

        let mut stmt = self.db.conn.prepare(&query).unwrap();

        let rows = stmt
            .query_map([], |row| {
                Ok(ProductWithPriceAndStore {
                    product_id: row.get(0)?,
                    product_name: row.get(1)?,
                    brand: row.get(2)?,
                    size_value: row.get(3)?,
                    size_unit: row.get(4)?,
                    price: row.get(5)?,
                    supermarket: row.get(6)?,
                    store_id: row.get(7)?,
                    supermarket_id: row.get(8)?,
                    store_name: row.get(9)?,
                    store_latitude: row.get(10)?,
                    store_longitude: row.get(11)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Search for products with their prices and store information.
    pub fn search_products_with_prices_and_stores(
        &self,
        search_term: &str,
    ) -> Vec<ProductWithPriceAndStore> {
        // Split search term into words
        let words: Vec<&str> = search_term.split_whitespace().collect();

        if words.is_empty() {
            return Vec::new();
        }

        // Build WHERE clause
        let word_conditions: Vec<String> = words
            .iter()
            .map(|w| format!("(p.name LIKE '%{}%' OR p.brand LIKE '%{}%')", w, w))
            .collect();
        let where_clause = word_conditions.join(" AND ");

        let query = format!(
            "SELECT p.id, p.name, COALESCE(p.brand, ''), COALESCE(p.size_value, 0.0), COALESCE(p.size_unit, ''),
                    pr.price, v.supermarket, st.id, s.id,
                    st.name, COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             JOIN supermarkets s ON v.supermarket = s.name
             JOIN prices pr ON v.id = pr.variant_id
             JOIN stores st ON pr.store_id = st.id
             WHERE ({})
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.variant_id = pr.variant_id AND pr2.store_id = pr.store_id
             )
             ORDER BY pr.price ASC
             LIMIT 500",
            where_clause
        );

        let mut stmt = self.db.conn.prepare(&query).unwrap();

        let rows = stmt
            .query_map([], |row| {
                Ok(ProductWithPriceAndStore {
                    product_id: row.get(0)?,
                    product_name: row.get(1)?,
                    brand: row.get(2)?,
                    size_value: row.get(3)?,
                    size_unit: row.get(4)?,
                    price: row.get(5)?,
                    supermarket: row.get(6)?,
                    store_id: row.get(7)?,
                    supermarket_id: row.get(8)?,
                    store_name: row.get(9)?,
                    store_latitude: row.get(10)?,
                    store_longitude: row.get(11)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // BM25 Full-Text Search
    // -------------------------------------------------------------------------

    /// Search for products using FTS5 BM25 ranking.
    ///
    /// BM25 is a keyword-based ranking algorithm that:
    /// - Strongly prefers exact word matches
    /// - Weights term frequency and document length
    /// - Better at "milk" → "Fresh Milk" over "Chocolate Milk"
    ///
    /// Returns products with their BM25 score (lower = better match).
    pub fn search_products_bm25(
        &self,
        search_term: &str,
        store_ids: &[String],
        limit: usize,
    ) -> Vec<ProductWithBm25Score> {
        if store_ids.is_empty() || search_term.trim().is_empty() {
            return Vec::new();
        }

        // Build IN clause for store IDs
        let store_placeholders: Vec<String> = store_ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();
        let store_in_clause = store_placeholders.join(", ");

        // FTS5 query: exact word matching
        let fts_query = search_term.split_whitespace().collect::<Vec<_>>().join(" ");

        let query = format!(
            "SELECT p.id, p.name, COALESCE(p.brand, ''), COALESCE(p.size_value, 0.0), COALESCE(p.size_unit, ''),
                    pr.price, v.supermarket, st.id,
                    st.name, COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0),
                    bm25(products_fts) as bm25_score
             FROM products_fts fts
             JOIN products p ON fts.rowid = p.id
             JOIN product_variants v ON p.id = v.product_id
             JOIN prices pr ON v.id = pr.variant_id
             JOIN stores st ON pr.store_id = st.id
             WHERE products_fts MATCH ?1
             AND st.id IN ({})
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.variant_id = pr.variant_id AND pr2.store_id = pr.store_id
             )
             ORDER BY bm25_score
             LIMIT ?2",
            store_in_clause
        );

        let mut stmt = match self.db.conn.prepare(&query) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = stmt.query_map(params![fts_query, limit as i64], |row| {
            Ok(ProductWithBm25Score {
                product_id: row.get(0)?,
                product_name: row.get(1)?,
                brand: row.get(2)?,
                size_value: row.get(3)?,
                size_unit: row.get(4)?,
                price: row.get(5)?,
                supermarket: row.get(6)?,
                store_id: row.get(7)?,
                store_name: row.get(8)?,
                store_latitude: row.get(9)?,
                store_longitude: row.get(10)?,
                bm25_score: row.get(11)?,
            })
        });

        match rows {
            Ok(r) => r.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }
    // -------------------------------------------------------------------------
    // Products by id
    // -------------------------------------------------------------------------
    pub fn get_products_by_ids(
        &self,
        store_ids: &[String],
        product_ids: &[String]
    ) -> Vec<ProductWithPriceAndStore> {
        if store_ids.is_empty() {
            return Vec::new();
        }
        let placeholders: Vec<String> = store_ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT p.id, p.name, COALESCE(p.brand, ''), COALESCE(p.size_value, 0.0), COALESCE(p.size_unit, ''),
                    pr.price, v.supermarket, st.id, s.id,
                    st.name, COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM products p
             JOIN product_variants v ON p.id = v.product_id
             JOIN supermarkets s ON v.supermarket = s.name
             JOIN prices pr ON v.id = pr.variant_id
             JOIN stores st ON pr.store_id = st.id
             WHERE st.id IN ({})
             AND v.fetch_stamp = (SELECT value FROM metadata WHERE key = 'valid_fetch_stamp')
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.variant_id = pr.variant_id AND pr2.store_id = pr.store_id
             )
             AND p.id in ({})
             ORDER BY p.name ASC",
            placeholders.join(", "),
            product_ids.join(", "),
        );
        let mut stmt = self.db.conn.prepare(&query).unwrap();
        let params: Vec<&dyn rusqlite::ToSql> = store_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok(ProductWithPriceAndStore {
                    product_id: row.get(0)?,
                    product_name: row.get(1)?,
                    brand: row.get(2)?,
                    size_value: row.get(3)?,
                    size_unit: row.get(4)?,
                    price: row.get(5)?,
                    supermarket: row.get(6)?,
                    store_id: row.get(7)?,
                    supermarket_id: row.get(8)?,
                    store_name: row.get(9)?,
                    store_latitude: row.get(10)?,
                    store_longitude: row.get(11)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }
}
// -----------------------------------------------------------------------------
// Result Types
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct ProductResult {
    pub name: String,
    pub supermarket: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub category: String,
}

#[derive(Debug)]
pub struct PriceRangeResult {
    pub product_name: String,
    pub supermarket: String,
    pub min_price: f64,
    pub max_price: f64,
    pub store_count: i64,
}

#[derive(Debug)]
pub struct StorePrice {
    pub product_name: String,
    pub supermarket: String,
    pub store_name: String,
    pub store_address: String,
    pub price: f64,
}

#[derive(Debug)]
pub struct DatabaseStats {
    pub products: i64,
    pub variants: i64,
    pub prices: i64,
    pub stores: i64,
    pub categories: i64,
}

#[derive(Debug)]
pub struct ProductWithPrice {
    pub name: String,
    pub supermarket: String,
    pub brand: String,
    pub min_price: f64,
    pub max_price: f64,
}

/// Price information for a single product across stores.
#[derive(Debug, Clone)]
pub struct ProductPriceInfo {
    pub supermarket: String,
    pub store_name: String,
    pub store_id: String,
    pub price: f64,
    pub fetched_at: String,
    pub store_latitude: f64,
    pub store_longitude: f64,
}

/// Product with BM25 relevance score from full-text search.
#[derive(Debug, Clone)]
pub struct ProductWithBm25Score {
    pub product_id: i32,
    pub product_name: String,
    pub brand: String,
    pub size_value: f64,
    pub size_unit: String,
    pub price: f64,
    pub supermarket: String,
    pub store_id: String,
    pub store_name: String,
    pub store_latitude: f64,
    pub store_longitude: f64,
    /// BM25 score (negative, more negative = better match)
    pub bm25_score: f64,
}

// -----------------------------------------------------------------------------
// Demo function to run sample queries
// -----------------------------------------------------------------------------

pub fn run_sample_queries(db: &Database) {
    let queries = Queries::new(db);

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              SUPERMARKET DATABASE - SAMPLE QUERIES               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // 1. Database Statistics
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ 1. DATABASE STATISTICS                                           │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    let stats = queries.get_stats();
    println!("  Products (deduplicated): {:>10}", stats.products);
    println!("  Variants:                {:>10}", stats.variants);
    println!("  Prices:                  {:>10}", stats.prices);
    println!("  Stores:                  {:>10}", stats.stores);
    println!("  Categories:              {:>10}", stats.categories);
    if stats.products > 0 {
        println!(
            "  Dedup ratio:             {:>10.2}x",
            stats.variants as f64 / stats.products as f64
        );
    }
    println!();

    // 2. Products per Supermarket
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ 2. PRODUCTS PER SUPERMARKET                                      │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    for (name, count) in queries.get_products_per_supermarket() {
        println!("  {:<15} {:>10} variants", name, count);
    }
    println!();

    // 3. Search Example
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ 3. SEARCH: \"Chicken Breast\" (first 10)                           │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    let products = queries.search_products("Chicken Breast");
    for p in products.into_iter().take(10) {
        println!("  {} | {} | {}", p.name, p.supermarket, p.brand);
    }
    println!();

    // 4. Price Comparison
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ 4. PRICE RANGE: \"Anchor Milk\" across stores                      │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    let prices = queries.get_price_range("Anchor Milk");
    for p in prices.into_iter().take(10) {
        let savings = p.max_price - p.min_price;
        println!(
            "  {} | {} | ${:.2} - ${:.2} ({} stores) [Save ${:.2}]",
            p.product_name, p.supermarket, p.min_price, p.max_price, p.store_count, savings
        );
    }
    println!();

    // 5. Cheapest Stores
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ 5. CHEAPEST STORES: \"Salted Butter\" (top 10)                     │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    let cheapest = queries.find_cheapest_stores("Salted Butter", 10);
    for s in &cheapest {
        println!(
            "  ${:.2} | {} | {} | {}",
            s.price, s.product_name, s.supermarket, s.store_name
        );
    }
    println!();

    // 6. Products by Brand
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ 6. BRAND SEARCH: \"Whittaker\" products                            │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    let brand_products = queries.get_products_by_brand("Whittaker");
    for p in brand_products.into_iter().take(10) {
        println!(
            "  {} | {} | ${:.2} - ${:.2}",
            p.name, p.supermarket, p.min_price, p.max_price
        );
    }
    println!();

    // 7. Products by Category
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ 7. CATEGORY: \"Chocolate\" (first 10)                              │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    let cat_products = queries.get_products_by_category("Chocolate", 10);
    for p in &cat_products {
        println!("  {} | {} | {}", p.name, p.supermarket, p.category);
    }
    println!();

    println!("══════════════════════════════════════════════════════════════════");
    println!("  Queries completed! Use Queries struct for custom queries.");
    println!("══════════════════════════════════════════════════════════════════");
}
