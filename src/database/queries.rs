use rusqlite::params;

use super::Database;

/// Result type for product search with price and store information.
#[derive(Debug, Clone)]
pub struct ProductWithPriceAndStore {
    pub product_name: String,
    pub brand: String,
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
        let mut stmt = self.db.conn.prepare(
            "SELECT p.name, s.name, b.name, p.size_value, p.size_unit, c.display_name
             FROM products p
             JOIN supermarkets s ON p.supermarket_id = s.id
             LEFT JOIN brands b ON p.brand_id = b.id
             LEFT JOIN categories c ON p.category_id = c.id
             WHERE p.name LIKE ?1
             ORDER BY p.name
             LIMIT 50"
        ).unwrap();

        let pattern = format!("%{}%", search_term);
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(ProductResult {
                name: row.get(0)?,
                supermarket: row.get(1)?,
                brand: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size_value: row.get(3)?,
                size_unit: row.get(4)?,
                category: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            })
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Price Comparison
    // -------------------------------------------------------------------------

    /// Get price range for a product across all stores.
    pub fn get_price_range(&self, product_name: &str) -> Vec<PriceRangeResult> {
        let mut stmt = self.db.conn.prepare(
            "SELECT p.name, s.name, MIN(pr.price), MAX(pr.price), COUNT(DISTINCT pr.store_id)
             FROM products p
             JOIN supermarkets s ON p.supermarket_id = s.id
             JOIN prices pr ON p.id = pr.product_id
             WHERE p.name LIKE ?1
             GROUP BY p.name, s.name
             ORDER BY MIN(pr.price)"
        ).unwrap();

        let pattern = format!("%{}%", product_name);
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(PriceRangeResult {
                product_name: row.get(0)?,
                supermarket: row.get(1)?,
                min_price: row.get(2)?,
                max_price: row.get(3)?,
                store_count: row.get(4)?,
            })
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Find the cheapest stores for a specific product.
    pub fn find_cheapest_stores(&self, product_name: &str, limit: u32) -> Vec<StorePrice> {
        let mut stmt = self.db.conn.prepare(
            "SELECT p.name, s.name, st.name, st.address, pr.price
             FROM products p
             JOIN supermarkets s ON p.supermarket_id = s.id
             JOIN prices pr ON p.id = pr.product_id
             JOIN stores st ON pr.store_id = st.id
             WHERE p.name LIKE ?1
             ORDER BY pr.price ASC
             LIMIT ?2"
        ).unwrap();

        let pattern = format!("%{}%", product_name);
        let rows = stmt.query_map(params![pattern, limit], |row| {
            Ok(StorePrice {
                product_name: row.get(0)?,
                supermarket: row.get(1)?,
                store_name: row.get(2)?,
                store_address: row.get(3)?,
                price: row.get(4)?,
            })
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Statistics
    // -------------------------------------------------------------------------

    /// Get database statistics.
    pub fn get_stats(&self) -> DatabaseStats {
        let product_count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM products", [], |row| row.get(0)
        ).unwrap_or(0);

        let price_count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM prices", [], |row| row.get(0)
        ).unwrap_or(0);

        let store_count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM stores", [], |row| row.get(0)
        ).unwrap_or(0);

        let brand_count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM brands", [], |row| row.get(0)
        ).unwrap_or(0);

        let category_count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM categories", [], |row| row.get(0)
        ).unwrap_or(0);

        DatabaseStats {
            products: product_count,
            prices: price_count,
            stores: store_count,
            brands: brand_count,
            categories: category_count,
        }
    }

    /// Get product count per supermarket.
    pub fn get_products_per_supermarket(&self) -> Vec<(String, i64)> {
        let mut stmt = self.db.conn.prepare(
            "SELECT s.name, COUNT(*)
             FROM products p
             JOIN supermarkets s ON p.supermarket_id = s.id
             GROUP BY s.name
             ORDER BY COUNT(*) DESC"
        ).unwrap();

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Category Queries
    // -------------------------------------------------------------------------

    /// Get products by category.
    pub fn get_products_by_category(&self, category: &str, limit: u32) -> Vec<ProductResult> {
        let mut stmt = self.db.conn.prepare(
            "SELECT p.name, s.name, b.name, p.size_value, p.size_unit, c.display_name
             FROM products p
             JOIN supermarkets s ON p.supermarket_id = s.id
             LEFT JOIN brands b ON p.brand_id = b.id
             LEFT JOIN categories c ON p.category_id = c.id
             WHERE c.display_name LIKE ?1
             ORDER BY p.name
             LIMIT ?2"
        ).unwrap();

        let pattern = format!("%{}%", category);
        let rows = stmt.query_map(params![pattern, limit], |row| {
            Ok(ProductResult {
                name: row.get(0)?,
                supermarket: row.get(1)?,
                brand: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size_value: row.get(3)?,
                size_unit: row.get(4)?,
                category: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            })
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Brand Queries
    // -------------------------------------------------------------------------

    /// Get all products from a specific brand.
    pub fn get_products_by_brand(&self, brand: &str) -> Vec<ProductWithPrice> {
        let mut stmt = self.db.conn.prepare(
            "SELECT p.name, s.name, b.name, MIN(pr.price), MAX(pr.price)
             FROM products p
             JOIN supermarkets s ON p.supermarket_id = s.id
             JOIN brands b ON p.brand_id = b.id
             JOIN prices pr ON p.id = pr.product_id
             WHERE b.name LIKE ?1
             GROUP BY p.id
             ORDER BY p.name
             LIMIT 50"
        ).unwrap();

        let pattern = format!("%{}%", brand);
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(ProductWithPrice {
                name: row.get(0)?,
                supermarket: row.get(1)?,
                brand: row.get(2)?,
                min_price: row.get(3)?,
                max_price: row.get(4)?,
            })
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Store Queries
    // -------------------------------------------------------------------------

    /// Get all stores with their location information.
    pub fn get_all_stores(&self) -> Vec<StoreInfo> {
        let mut stmt = self.db.conn.prepare(
            "SELECT st.id, st.name, st.supermarket_id, s.name,
                    COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM stores st
             JOIN supermarkets s ON st.supermarket_id = s.id"
        ).unwrap();

        let rows = stmt.query_map([], |row| {
            Ok(StoreInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                supermarket_id: row.get(2)?,
                supermarket_name: row.get(3)?,
                latitude: row.get(4)?,
                longitude: row.get(5)?,
            })
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get stores for a specific supermarket.
    pub fn get_stores_by_supermarket(&self, supermarket_id: i32) -> Vec<StoreInfo> {
        let mut stmt = self.db.conn.prepare(
            "SELECT st.id, st.name, st.supermarket_id, s.name,
                    COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM stores st
             JOIN supermarkets s ON st.supermarket_id = s.id
             WHERE st.supermarket_id = ?1"
        ).unwrap();

        let rows = stmt.query_map(params![supermarket_id], |row| {
            Ok(StoreInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                supermarket_id: row.get(2)?,
                supermarket_name: row.get(3)?,
                latitude: row.get(4)?,
                longitude: row.get(5)?,
            })
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    // -------------------------------------------------------------------------
    // Shopping List Query
    // -------------------------------------------------------------------------

    /// Search for products with their prices, filtered by store IDs.
    ///
    /// This is more efficient than fetching all products and filtering afterwards.
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
            .map(|w| format!("(p.name LIKE '%{}%' OR b.name LIKE '%{}%')", w, w))
            .collect();
        let search_clause = word_conditions.join(" AND ");

        // Build IN clause for store IDs
        let store_placeholders: Vec<String> = store_ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();
        let store_in_clause = store_placeholders.join(", ");

        let query = format!(
            "SELECT p.name, COALESCE(b.name, ''), pr.price, s.name, st.id, p.supermarket_id,
                    st.name, COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM products p
             JOIN supermarkets s ON p.supermarket_id = s.id
             LEFT JOIN brands b ON p.brand_id = b.id
             JOIN prices pr ON p.id = pr.product_id
             JOIN stores st ON pr.store_id = st.id
             WHERE ({})
             AND st.id IN ({})
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.product_id = pr.product_id AND pr2.store_id = pr.store_id
             )
             ORDER BY pr.price ASC
             LIMIT 500",
            search_clause, store_in_clause
        );

        let mut stmt = self.db.conn.prepare(&query).unwrap();

        let rows = stmt.query_map([], |row| {
            Ok(ProductWithPriceAndStore {
                product_name: row.get(0)?,
                brand: row.get(1)?,
                price: row.get(2)?,
                supermarket: row.get(3)?,
                store_id: row.get(4)?,
                supermarket_id: row.get(5)?,
                store_name: row.get(6)?,
                store_latitude: row.get(7)?,
                store_longitude: row.get(8)?,
            })
        }).unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Search for products with their prices and store information.
    ///
    /// Returns products matching the search term with their latest price and store details.
    /// For multi-word searches, finds products that contain ALL words (in any order).
    /// Also searches the brand name for better matching.
    pub fn search_products_with_prices_and_stores(&self, search_term: &str) -> Vec<ProductWithPriceAndStore> {
        // Split search term into words and create LIKE conditions for each
        let words: Vec<&str> = search_term.split_whitespace().collect();

        if words.is_empty() {
            return Vec::new();
        }

        // Build WHERE clause: each word must appear in product name OR brand name
        let word_conditions: Vec<String> = words
            .iter()
            .map(|w| format!("(p.name LIKE '%{}%' OR b.name LIKE '%{}%')", w, w))
            .collect();
        let where_clause = word_conditions.join(" AND ");

        let query = format!(
            "SELECT p.name, COALESCE(b.name, ''), pr.price, s.name, st.id, p.supermarket_id,
                    st.name, COALESCE(st.latitude, 0.0), COALESCE(st.longitude, 0.0)
             FROM products p
             JOIN supermarkets s ON p.supermarket_id = s.id
             LEFT JOIN brands b ON p.brand_id = b.id
             JOIN prices pr ON p.id = pr.product_id
             JOIN stores st ON pr.store_id = st.id
             WHERE ({})
             AND pr.fetched_at = (
                 SELECT MAX(pr2.fetched_at)
                 FROM prices pr2
                 WHERE pr2.product_id = pr.product_id AND pr2.store_id = pr.store_id
             )
             ORDER BY pr.price ASC
             LIMIT 500",
            where_clause
        );

        let mut stmt = self.db.conn.prepare(&query).unwrap();

        let rows = stmt.query_map([], |row| {
            Ok(ProductWithPriceAndStore {
                product_name: row.get(0)?,
                brand: row.get(1)?,
                price: row.get(2)?,
                supermarket: row.get(3)?,
                store_id: row.get(4)?,
                supermarket_id: row.get(5)?,
                store_name: row.get(6)?,
                store_latitude: row.get(7)?,
                store_longitude: row.get(8)?,
            })
        }).unwrap();

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
    pub prices: i64,
    pub stores: i64,
    pub brands: i64,
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
    println!("  Products:   {:>10}", stats.products);
    println!("  Prices:     {:>10}", stats.prices);
    println!("  Stores:     {:>10}", stats.stores);
    println!("  Brands:     {:>10}", stats.brands);
    println!("  Categories: {:>10}", stats.categories);
    println!();

    // 2. Products per Supermarket
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│ 2. PRODUCTS PER SUPERMARKET                                      │");
    println!("└──────────────────────────────────────────────────────────────────┘");
    for (name, count) in queries.get_products_per_supermarket() {
        println!("  {:<15} {:>10} products", name, count);
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
