# Supermarkets Module

Fetchers for scraping product data from NZ supermarket APIs.

## Files

| File | Purpose |
|------|---------|
| `fetch_controller.rs` | Orchestrates parallel fetching and batch insertion |
| `new_world_fetcher.rs` | New World API client |
| `pack_n_save_fetcher.rs` | PAK'nSAVE API client |
| `woolworth_fetcher.rs` | Woolworths (Countdown) API client |
| `super_market_fetcher_trait.rs` | Common trait for fetchers |
| `supermarket_types.rs` | Enum for supermarket identification |
| `size_unit_types.rs` | Parser for product sizes (kg, ml, pack, etc.) |

## Submodules

### models/
Data structures for products, stores, categories, tokens.

### food_stuff/
Shared code for New World and PAK'nSAVE (same parent company, similar APIs).

## Architecture

```
FetchController.run()
    │
    ├── Phase 1: Parallel fetch (tokio::join!)
    │       ├── WoolworthFetcher
    │       ├── NewWorldFetcher (per-store)
    │       └── PackNSaveFetcher (per-store)
    │
    ├── Phase 2: Collect all results
    │       └── Vec<FetchResult> (~200k items)
    │
    └── Phase 3: Batch insert
            └── Repository.insert_all_items()
                    └── Deduplication + embedding generation
```

## Size Unit Parsing

`SizeUnit::parse()` handles various formats:

| Input | Parsed As |
|-------|-----------|
| `500g`, `0.5kg` | `Gram(500)`, `Kilogram(0.5)` |
| `6 x 250ml` | `MultiPack { count: 6, unit: Milliliter(250) }` |
| `0.5-0.7kg` | `Range { min: 0.5, max: 0.7, unit: Kilogram }` |
| `100 sheets` | `Sheet(100)` |
| `each`, `single` | `Each(1)` |

### Size Normalization

`to_normalized_value_and_unit()` normalizes to base units for deduplication:

| Category | Base Unit | Conversions |
|----------|-----------|-------------|
| Weight | Gram | kg × 1000, mg ÷ 1000 |
| Volume | Milliliter | L × 1000 |
| Length | Centimeter | m × 100, mm ÷ 10 |

This ensures "500g" and "0.5kg" are recognized as the same size.

## Usage

```rust
let controller = FetchController::new();
controller.run().await;  // Fetches all supermarkets with deduplication
```
