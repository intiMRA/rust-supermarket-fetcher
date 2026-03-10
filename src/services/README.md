# Services Module

Business logic layer between API and database.

## Files

| File | Purpose |
|------|---------|
| `shopping_list_service.rs` | Shopping list processing with hybrid search |

## Shopping List Service

Processes shopping list requests using a hybrid BM25 + semantic search approach.

### Algorithm

1. **Find nearby stores** - Filter by 20km radius (NewWorld/PakNSave) or include all (Woolworths)
2. **BM25 search** - Get top 100 candidates using keyword matching
3. **Semantic scoring** - Apply embedding similarity to candidates
4. **Hybrid ranking** - Combine scores with weights:
   - BM25: 40%
   - Semantic: 20%
   - Price: 40%
5. **Deduplicate** - Keep cheapest option per unique product
6. **Return top 3** - Per search term

### Fallback

If BM25 returns no results (FTS index miss), falls back to:
1. Category-based search (matches category slugs)
2. LIKE-based text search

### Configuration

```rust
const BM25_WEIGHT: f64 = 0.4;
const SEMANTIC_WEIGHT: f64 = 0.2;
const PRICE_WEIGHT: f64 = 0.4;
const MAX_DISTANCE_KM: f64 = 20.0;
const TOP_N_MATCHES: usize = 3;
```

## Usage

```rust
let response = process_shopping_list(&request, &db);
```
