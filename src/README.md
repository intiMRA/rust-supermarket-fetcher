# Source Code Structure

## Module Overview

```
src/
├── main.rs              # CLI entry point (fetch, query, serve)
├── api/                 # REST API (Actix-web)
├── database/            # SQLite storage and queries
├── matching/            # Product search algorithms
├── services/            # Business logic
├── supermarkets/        # Data fetchers for each supermarket
├── loggers/             # Logging utilities
├── utils/               # Helper functions
└── custom_types/        # Shared types and errors
```

## Data Flow

```
1. FETCH
   supermarkets/* → database/repository.rs → SQLite

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
| `database/` | Store products, prices, and FTS5 search index |
| `matching/` | Rank products using BM25 + semantic similarity |
| `services/` | Combine search + location + price for final results |
| `api/` | HTTP endpoints for shopping list queries |

## CLI Commands

```bash
cargo run -- fetch   # Scrape all supermarkets
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

See individual module READMEs for details.
