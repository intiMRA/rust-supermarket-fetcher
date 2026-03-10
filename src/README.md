# Source Code Structure

## Module Overview

```
src/
├── main.rs              # CLI entry point (fetch, query, serve)
├── api/                 # REST API (Actix-web)
├── database/            # SQLite storage with deduplication
├── matching/            # Embedding utilities & search algorithms
├── services/            # Business logic
├── supermarkets/        # Data fetchers for each supermarket
├── loggers/             # Logging utilities
├── utils/               # Helper functions
└── custom_types/        # Shared types and errors
```

## Data Flow

```
1. FETCH (with batch deduplication)
   Supermarket APIs
        │
        ▼
   Collect all items from all stores
        │
        ▼
   Repository.insert_all_items()
        │
        ├─► Phase 1: Insert stores
        ├─► Phase 2: In-memory dedup (200k → 30k unique)
        ├─► Phase 3: Exact match lookup
        ├─► Phase 4: Batch embeddings (1000 at a time)
        └─► Phase 5: Insert variants + prices

2. SERVE
   API Request → services/shopping_list_service.rs
                     ├── database/queries.rs (BM25 search)
                     ├── matching/semantic_matcher.rs (embeddings)
                     └── Hybrid scoring → Response
```

## Key Components

| Module | Responsibility |
|--------|----------------|
| `supermarkets/` | Scrape products from NewWorld, PAK'nSAVE, Woolworths |
| `database/` | Store deduplicated products, handle batch inserts |
| `matching/` | Embedding generation, cosine similarity, fuzzy search |
| `services/` | Combine search + location + price for final results |
| `api/` | HTTP endpoints for shopping list queries |

## Product Deduplication

Products are deduplicated across supermarkets during fetch:

```
"Anchor Butter 500g" (NewWorld)  ─┐
"Anchor Butter 500g" (PakNSave)  ─┼─► ONE canonical product
"Anchor Butter 0.5kg" (Woolworths)─┘   with 3 variants & prices
```

### Optimizations

| Optimization | Impact |
|--------------|--------|
| In-memory grouping | 7x fewer DB calls |
| Batch embeddings | 30x faster generation |
| Category caching | 400x fewer lookups |

## CLI Commands

```bash
cargo run -- fetch   # Scrape all supermarkets (with deduplication)
cargo run -- query   # Run sample database queries
cargo run -- serve   # Start REST API on :8080
```

## Search Algorithm

Hybrid ranking with three signals:

| Signal | Weight | Source |
|--------|--------|--------|
| BM25 (keywords) | 40% | SQLite FTS5 |
| Semantic | 20% | AllMiniLML6V2 embeddings |
| Price | 40% | Lower = better |

## Database Rebuild

```bash
rm data/supermarket.db
cargo run -- fetch
```

See individual module READMEs for details.
