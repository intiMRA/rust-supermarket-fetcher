use rusqlite::params;
use super::Database;
use crate::supermarkets::supermarket_types::Supermarket;
use crate::supermarkets::models::store::Store;
use crate::supermarkets::models::super_market_item::SuperMarketItem;

/// Repository for database operations.
///
/// Provides methods to insert and query data.
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
    // Brands
    // -------------------------------------------------------------------------

    /// Insert a brand and return its ID.
    ///
    /// If the brand already exists, returns the existing ID.
    pub fn insert_brand(&self, name: &str) -> rusqlite::Result<i64> {
        // Try to insert (will be ignored if exists due to UNIQUE constraint)
        self.db.conn.execute(
            "INSERT OR IGNORE INTO brands (name) VALUES (?1)",
            params![name],
        )?;

        // Get the ID (whether we just inserted or it already existed)
        let id: i64 = self.db.conn.query_row(
            "SELECT id FROM brands WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    // -------------------------------------------------------------------------
    // Categories
    // -------------------------------------------------------------------------

    /// Insert a category and return its ID.
    ///
    /// If the category already exists for this supermarket, returns existing ID.
    pub fn insert_category(
        &self,
        display_name: &str,
        slug: &str,
        supermarket: Supermarket,
    ) -> rusqlite::Result<i64> {
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

        Ok(id)
    }

    // -------------------------------------------------------------------------
    // Products
    // -------------------------------------------------------------------------

    /// Insert a product and return its ID.
    ///
    /// If the product already exists, returns the existing ID.
    pub fn insert_product(&self, item: &SuperMarketItem) -> rusqlite::Result<i64> {
        let supermarket_id = Self::supermarket_id(item.supermarket);

        // Insert brand first (if present)
        let brand_id = if !item.brand_name.is_empty() {
            Some(self.insert_brand(&item.brand_name)?)
        } else {
            None
        };

        // Insert category
        let category_id = self.insert_category(
            &item.category.display_name,
            &item.category.slug,
            item.supermarket,
        )?;

        // Extract size info
        let (size_value, size_unit) = &item.size.to_value_and_unit();

        // Insert product
        self.db.conn.execute(
            "INSERT OR IGNORE INTO products
             (external_id, name, brand_id, image_url, size_value, size_unit, category_id, supermarket_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                item.id,
                item.name,
                brand_id,
                item.image_url,
                size_value,
                size_unit,
                category_id,
                supermarket_id,
            ],
        )?;

        // Get product ID
        let id: i64 = self.db.conn.query_row(
            "SELECT id FROM products WHERE external_id = ?1 AND supermarket_id = ?2",
            params![item.id, supermarket_id],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    // -------------------------------------------------------------------------
    // Prices
    // -------------------------------------------------------------------------

    /// Insert a price record for a product at a specific store.
    ///
    /// Uses INSERT OR IGNORE to skip duplicate prices (same product/store/timestamp).
    pub fn insert_price(&self, product_id: i64, store_id: &str, price: f64) -> rusqlite::Result<()> {
        self.db.conn.execute(
            "INSERT OR IGNORE INTO prices (product_id, store_id, price) VALUES (?1, ?2, ?3)",
            params![product_id, store_id, price],
        )?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // High-level: Insert item with price
    // -------------------------------------------------------------------------

    /// Insert a supermarket item and its price at a specific store.
    ///
    /// This is the main method you'll use to import data.
    pub fn insert_item_with_price(&self, item: &SuperMarketItem, store_id: &str) -> rusqlite::Result<()> {
        let product_id = self.insert_product(item)?;
        self.insert_price(product_id, store_id, item.price)?;
        Ok(())
    }

}
