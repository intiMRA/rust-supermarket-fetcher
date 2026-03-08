use rusqlite::Connection;

/// SQL statements to create all tables.
///
/// Using `IF NOT EXISTS` makes this idempotent - safe to run multiple times.
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

    -- Brands table (normalized to avoid duplicate strings)
    CREATE TABLE IF NOT EXISTS brands (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE
    );

    -- Categories table (e.g., "Pantry > Chocolate > Bags")
    CREATE TABLE IF NOT EXISTS categories (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        display_name TEXT NOT NULL,                             -- Full path: "Pantry > Chocolate"
        slug TEXT NOT NULL,                                     -- Leaf name: "Chocolate"
        supermarket_id INTEGER NOT NULL REFERENCES supermarkets(id),
        UNIQUE(display_name, supermarket_id)                    -- Same category can exist per supermarket
    );

    -- Products table (the actual items)
    CREATE TABLE IF NOT EXISTS products (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        external_id TEXT NOT NULL,                              -- ID from the API (e.g., "5329009-EA-000")
        name TEXT NOT NULL,
        brand_id INTEGER REFERENCES brands(id),
        image_url TEXT,
        size_value REAL,                                        -- e.g., 500.0
        size_unit TEXT,                                         -- e.g., "Gram", "Milliliter"
        category_id INTEGER REFERENCES categories(id),
        supermarket_id INTEGER NOT NULL REFERENCES supermarkets(id),
        UNIQUE(external_id, supermarket_id)                     -- Same product ID unique per supermarket
    );

    -- Prices table (tracks price per store, enables history)
    -- Note: We use DATE() for daily price tracking. For more granular tracking,
    -- you could use DATETIME() or a custom timestamp column.
    CREATE TABLE IF NOT EXISTS prices (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        product_id INTEGER NOT NULL REFERENCES products(id),
        store_id TEXT NOT NULL REFERENCES stores(id),
        price REAL NOT NULL,
        fetched_at TEXT DEFAULT (DATE('now')),                  -- Date we fetched this price
        UNIQUE(product_id, store_id, fetched_at)                -- One price per product/store/day
    );

    -- Indexes for fast queries
    CREATE INDEX IF NOT EXISTS idx_prices_product ON prices(product_id);
    CREATE INDEX IF NOT EXISTS idx_prices_store ON prices(store_id);
    CREATE INDEX IF NOT EXISTS idx_prices_fetched ON prices(fetched_at);
    CREATE INDEX IF NOT EXISTS idx_products_name ON products(name);
    CREATE INDEX IF NOT EXISTS idx_products_category ON products(category_id);
    CREATE INDEX IF NOT EXISTS idx_products_external ON products(external_id);

    -- Metadata table for tracking various timestamps and settings
    CREATE TABLE IF NOT EXISTS metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
"#;

/// Initialize the database schema.
///
/// This creates all tables and indexes if they don't exist.
/// Safe to call multiple times.
pub fn initialize(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(CREATE_TABLES)
}
