# Matching Module

Product matching algorithms for search relevance ranking and cross-supermarket deduplication.

## Files

| File | Purpose |
|------|---------|
| `product_matcher.rs` | Embedding utilities (batch generation, cosine similarity, byte conversion) |
| `fuzzy_matcher.rs` | Jaro-Winkler string similarity with word boundary detection |
| `semantic_matcher.rs` | Sentence transformer embeddings for search ranking |

## Product Deduplication

Deduplication is handled by the `Repository` in the database module, using utilities from this module:

1. **In-memory grouping** - Group items by (name, brand, normalized_size)
2. **Batch embeddings** - Generate embeddings for unique products in batches of 1000
3. **Cosine similarity** - Compare embeddings to find semantic matches (85% threshold)

## Search Matching Strategies

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
// Batch embedding generation (used by Repository)
let texts = vec!["Anchor Butter 500g".to_string(), "Fresh Milk 2L".to_string()];
let embeddings = generate_embeddings_batch(&texts)?;

// Compare embeddings
let similarity = cosine_similarity(&embeddings[0], &embeddings[1]);

// Semantic search ranking
let matches = find_matching_products_semantic("butter", &products, 0.3);

// Fuzzy matching
let matches = find_matching_products("bread", &products, 0.6);
```

## Model

Uses `AllMiniLML6V2` sentence transformer:
- ~90MB download on first use
- Generates 384-dimensional embeddings
- Cached globally for reuse across requests
