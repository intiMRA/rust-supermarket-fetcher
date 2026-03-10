# Matching Module

Product matching algorithms for search relevance ranking.

## Files

| File | Purpose |
|------|---------|
| `fuzzy_matcher.rs` | Jaro-Winkler string similarity with word boundary detection |
| `semantic_matcher.rs` | Sentence transformer embeddings (AllMiniLML6V2) |

## Matching Strategies

### BM25 (in database module)
Keyword-based ranking from SQLite FTS5:
- Fast and accurate for exact matches
- "milk" strongly prefers "Fresh Milk" over "Chocolate Milk"
- Weight: 40% in hybrid scoring

### Semantic Matching
Neural embedding similarity using `fastembed`:
- Understands meaning beyond keywords
- Handles synonyms and related concepts
- Weight: 20% in hybrid scoring

### Fuzzy Matching
Character-level similarity (Jaro-Winkler):
- Handles typos and spelling variations
- Word boundary detection prevents false positives
- Used as fallback when other methods fail

## Hybrid Scoring

The shopping list service combines all three:

```
final_score = (bm25 * 0.4) + (semantic * 0.2) + (price * 0.4)
```

## Usage

```rust
// Semantic matching
let matches = find_matching_products_semantic("butter", &products, 0.3);

// Fuzzy matching
let matches = find_matching_products("bread", &products, 0.6);
```

## Model

Uses `AllMiniLML6V2` sentence transformer:
- ~90MB download on first use
- Generates 384-dimensional embeddings
- Cached globally for reuse across requests
