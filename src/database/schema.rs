use rusqlite::Connection;

/// SQL statements to create all tables.
///
/// Using `IF NOT EXISTS` makes this idempotent - safe to run multiple times.
///
/// Schema Design (Deduplication):
/// - `products`: One row per unique product (e.g., "Anchor Butter 500g")
///   - Deduplicated across supermarkets using semantic matching
///   - `embedding` stores 384-dim float32 vector for matching
/// - `product_variants`: One row per supermarket source
///   - Links to canonical product, stores original name
/// - `prices`: One row per variant/store/day
const CREATE_TABLES: &str = r#"
    -- Supermarkets table (NewWorld, PakNSave, Woolworth)
    CREATE TABLE IF NOT EXISTS supermarkets (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL UNIQUE
    );

    -- Insert the three supermarkets (ignore if already exist)
    INSERT OR IGNORE INTO supermarkets (id, name) VALUES (1, 'NewWorld');
    INSERT OR IGNORE INTO supermarkets (id, name) VALUES (2, 'PakNSave');
    INSERT OR IGNORE INTO supermarkets (id, name) VALUES (3, 'Woolworth');

    -- Stores table (physical store locations)
    CREATE TABLE IF NOT EXISTS stores (
        id TEXT PRIMARY KEY,                                    -- UUID from API
        supermarket_id INTEGER NOT NULL REFERENCES supermarkets(id),
        name TEXT NOT NULL,
        address TEXT,
        latitude REAL,
        longitude REAL
    );

    -- Categories table (e.g., "Pantry > Chocolate > Bags")
    -- Categories are supermarket-specific
    CREATE TABLE IF NOT EXISTS categories (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        display_name TEXT NOT NULL,                             -- Full path: "Pantry > Chocolate"
        slug TEXT NOT NULL,                                     -- Leaf name: "Chocolate"
        supermarket_id INTEGER NOT NULL REFERENCES supermarkets(id),
        UNIQUE(display_name, supermarket_id)                    -- Same category can exist per supermarket
    );

    -- Products table (deduplicated across supermarkets)
    -- One row per unique product (e.g., "Anchor Butter 500g")
    -- Size is part of identity: "Butter 1kg" != "Butter 500g"
    CREATE TABLE IF NOT EXISTS products (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        brand TEXT,                                             -- TEXT, no brands table needed
        size_value REAL,                                        -- Part of product identity (normalized)
        size_unit TEXT,                                         -- Part of product identity (normalized)
        embedding BLOB NOT NULL                                 -- 384-dim float32 (1536 bytes)
    );

    -- Product variants (original product info from each supermarket)
    -- One row per supermarket source (tracks original names)
    CREATE TABLE IF NOT EXISTS product_variants (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        product_id INTEGER NOT NULL REFERENCES products(id),
        external_id TEXT NOT NULL,                              -- ID from the API
        original_name TEXT NOT NULL,                            -- As supermarket calls it
        image_url TEXT,
        category_id INTEGER REFERENCES categories(id),          -- Supermarket-specific
        supermarket TEXT NOT NULL,                              -- "NewWorld", "PakNSave", "Woolworth"
        UNIQUE(external_id, supermarket)
    );

    -- Prices per store
    CREATE TABLE IF NOT EXISTS prices (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        variant_id INTEGER NOT NULL REFERENCES product_variants(id),
        store_id TEXT NOT NULL REFERENCES stores(id),
        price REAL NOT NULL,
        fetched_at TEXT DEFAULT (DATE('now')),
        UNIQUE(variant_id, store_id, fetched_at)
    );

    -- Indexes for fast queries
    CREATE INDEX IF NOT EXISTS idx_products_exact ON products(name, brand, size_value, size_unit);
    CREATE INDEX IF NOT EXISTS idx_products_brand ON products(brand);
    CREATE INDEX IF NOT EXISTS idx_variants_product ON product_variants(product_id);
    CREATE INDEX IF NOT EXISTS idx_variants_external ON product_variants(external_id, supermarket);
    CREATE INDEX IF NOT EXISTS idx_prices_variant ON prices(variant_id);
    CREATE INDEX IF NOT EXISTS idx_prices_store ON prices(store_id);
    CREATE INDEX IF NOT EXISTS idx_prices_fetched ON prices(fetched_at);

    -- Metadata table for tracking various timestamps and settings
    CREATE TABLE IF NOT EXISTS metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    -- FTS5 full-text search index for products
    -- Combines product name and brand for better matching
    CREATE VIRTUAL TABLE IF NOT EXISTS products_fts USING fts5(
        name,
        brand,
        content='',           -- External content (we manage sync manually)
        contentless_delete=1  -- Allow deletions
    );
"#;

/// Initialize the database schema.
///
/// This creates all tables and indexes if they don't exist.
/// Safe to call multiple times.
pub fn initialize(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(CREATE_TABLES)
}
