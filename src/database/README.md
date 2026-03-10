# Database Module

SQLite database layer for storing and querying supermarket data.

## Files

| File | Purpose |
|------|---------|
| `connection.rs` | Database connection wrapper and initialization |
| `schema.rs` | Table definitions and FTS5 full-text search index |
| `repository.rs` | Insert operations (products, prices, stores) |
| `queries.rs` | Read operations and search functionality |

## Schema

```
supermarkets (id, name)
    └── stores (id, supermarket_id, name, address, lat, lon)
    └── categories (id, display_name, slug, supermarket_id)
    └── products (id, external_id, name, brand_id, size, category_id, supermarket_id)
            └── prices (product_id, store_id, price, fetched_at)

brands (id, name)

products_fts (FTS5 virtual table for BM25 search)
```

## Key Features

### Full-Text Search (FTS5)
Products are indexed in `products_fts` for BM25 keyword search:
- Automatically synced when products are inserted
- Supports prefix matching (`milk*` matches `milks`)
- Returns relevance scores for ranking

### Transaction Support
`insert_items_for_store()` wraps all inserts in a transaction for performance (~100x faster than individual inserts).

## Usage

```rust
let db = Database::open("data/supermarket.db")?;
let repo = Repository::new(&db);
let queries = Queries::new(&db);

// Insert
repo.insert_items_for_store(&store, Supermarket::NewWorld, &items)?;

// Query
let results = queries.search_products_bm25("milk", &store_ids, 100);
```
