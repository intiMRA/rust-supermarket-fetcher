# Database Module

SQLite database layer for storing and querying supermarket data with cross-supermarket product deduplication.

## Files

| File | Purpose |
|------|---------|
| `connection.rs` | Database connection wrapper and initialization |
| `schema.rs` | Table definitions and FTS5 full-text search index |
| `repository.rs` | Insert operations with batch deduplication |
| `queries.rs` | Read operations and search functionality |

## Schema

The schema supports **product deduplication** across supermarkets. "Anchor Butter 500g" from NewWorld, PAK'nSAVE, and Woolworths maps to ONE canonical product.

```
supermarkets (id, name)
    └── stores (id, supermarket_id, name, address, lat, lon)
    └── categories (id, display_name, slug, supermarket_id)

products (id, name, brand, size_value, size_unit, embedding)
    │   └── Deduplicated across supermarkets
    │   └── 384-dim embedding for semantic matching
    │
    └── product_variants (id, product_id, external_id, original_name, supermarket)
            │   └── One per supermarket source
            │
            └── prices (variant_id, store_id, price, fetched_at)

products_fts (FTS5 virtual table for BM25 search)
```

## Deduplication Pipeline

The `Repository.insert_all_items()` method handles deduplication in 5 phases:

```
Phase 1: Insert stores
Phase 2: In-memory deduplication (200k items → 30k unique products)
Phase 3: Find exact matches (skip embeddings for existing products)
Phase 4: Batch generate embeddings (1000 at a time for new products)
Phase 5: Insert variants and prices (with category caching)
```

### Optimizations

| Optimization | Impact |
|--------------|--------|
| In-memory grouping | 200k → 30k DB calls |
| Batch embeddings | 30x faster than one-at-a-time |
| Category caching | 200k → 500 category lookups |
| Exact match first | Skip embeddings for re-fetches |

## Key Features

### Full-Text Search (FTS5)
Products are indexed in `products_fts` for BM25 keyword search:
- Automatically synced when products are inserted
- Supports prefix matching (`milk*` matches `milks`)
- Returns relevance scores for ranking

### Transaction Support
All inserts wrapped in a single transaction for performance.

## Usage

```rust
let db = Database::open("data/supermarket.db")?;
let repo = Repository::new(&db);
let queries = Queries::new(&db);

// Insert with deduplication
let items_with_stores = /* ... */;
repo.insert_all_items(&items_with_stores)?;

// Query deduplicated products
let results = queries.search_products_bm25("milk", &store_ids, 100);

// Get stats
let stats = repo.get_deduplication_stats()?;
println!("{}", stats);  // Unique: 25000, Variants: 75000, Ratio: 3.00x
```

## Rebuilding the Database

```bash
rm data/supermarket.db
cargo run -- fetch
```
