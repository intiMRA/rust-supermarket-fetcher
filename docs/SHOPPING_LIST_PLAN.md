# Shopping List Price Comparison Feature

## Overview
Build a REST API that accepts a shopping list and user location, then returns the top 3 cheapest options for each item across supermarkets within a 20km radius (New World, Pack n Save) plus Woolworths (always included).

## Requirements
- **Input**: HTTP POST request with shopping list items + user lat/long
- **Matching**: BM25 full-text search via Tantivy with typo correction pre-processing
  - Supports queries like "anchor trim milk" (brand + product)
  - Handles typos like "ankor milk" → corrected to "anchor milk" before search
- **Location**: New World & Pack n Save filtered to 20km radius; Woolworths always included
- **Output**: Top 3 cheapest items per shopping list entry

---

## Implementation Plan

### Phase 1: Add Dependencies

**File**: `Cargo.toml`

Add:
```toml
actix-web = "4"           # REST API framework
tantivy = "0.22"          # Full-text search with BM25 ranking
strsim = "0.11"           # Edit distance for typo correction
```

---

### Phase 2: Haversine Distance Calculation

**New file**: `src/utils/geo.rs`

Calculate distance between two lat/long points:
```rust
pub fn haversine_distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64
```

Used to filter stores within 20km of user location.

---

### Phase 3: Typo Correction Layer

**New file**: `src/search/typo_corrector.rs`

Build a vocabulary from product names/brands and correct user typos before search:

```rust
pub struct TypoCorrector {
    vocabulary: HashSet<String>,  // All known words from product names + brands
}

impl TypoCorrector {
    /// Build vocabulary from product database
    pub fn build_from_products(products: &[(String, String)]) -> Self  // (name, brand)

    /// Correct a search query by finding closest vocabulary matches
    /// Uses edit distance (Levenshtein) with threshold
    pub fn correct(&self, query: &str, max_edit_distance: usize) -> String
}
```

**How it works**:
1. Extract all unique words from product names and brands → vocabulary
2. When user searches "ankor milk", split into tokens ["ankor", "milk"]
3. For each token, check if it's in vocabulary
4. If not, find closest match with edit distance ≤ 2 → "ankor" → "anchor"
5. Return corrected query "anchor milk" for BM25 search

**Rust crate**: `strsim::levenshtein` for edit distance

---

### Phase 4: BM25 Product Search with Tantivy

**New file**: `src/search/product_index.rs`

Build an in-memory search index for products:

```rust
pub struct ProductIndex {
    index: Index,
    reader: IndexReader,
    name_field: Field,
    brand_field: Field,
    product_id_field: Field,
}

impl ProductIndex {
    /// Build index from database products (call at startup)
    pub fn build_from_products(products: &[(i64, String, String)]) -> Result<Self, SearchError>

    /// Search for products matching query (e.g., "anchor trim milk")
    /// Returns product IDs ranked by BM25 score
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<(i64, f32)>, SearchError>
}
```

**Index schema**:
- `name` field: TEXT with tokenizer (searchable)
- `brand` field: TEXT with tokenizer (searchable, boosted 1.5x)
- `product_id` field: I64 stored (for joining back to DB)

**Query handling**:
- User input "anchor trim milk" → searches both name AND brand fields
- BM25 naturally ranks products matching more terms higher
- Brand matches get 1.5x boost (configurable)

---

### Phase 5: Shopping List Query Logic

**New file**: `src/services/shopping_list_service.rs`

```rust
pub struct ShoppingListRequest {
    pub items: Vec<String>,       // ["milk", "bread", "eggs"]
    pub latitude: f64,
    pub longitude: f64,
}

pub struct ShoppingListItem {
    pub search_term: String,
    pub top_matches: Vec<ProductMatch>,  // Top 3 cheapest
}

pub struct ShoppingListResponse {
    pub items: Vec<ShoppingListItem>,
}

pub async fn process_shopping_list(
    request: ShoppingListRequest,
    db: &Repository,
) -> Result<ShoppingListResponse, Error>
```

Logic:
1. For each search term in shopping list:
   a. Query all products matching the term (fuzzy)
   b. Get prices for each product from stores
   c. Filter stores: Woolworths always, NW/PnS within 20km
   d. Sort by price ascending
   e. Return top 3

---

### Phase 6: Database Query Updates

**File**: `src/database/queries.rs`

Add new query:
```rust
pub fn search_products_with_prices_and_stores(
    conn: &Connection,
    search_term: &str,
) -> Result<Vec<(Product, Price, Store)>, DatabaseError>
```

Returns product + latest price + store info for location filtering.

---

### Phase 7: REST API Endpoints

**New file**: `src/api/mod.rs`
**New file**: `src/api/routes.rs`
**New file**: `src/api/handlers.rs`

#### Endpoint: `POST /api/shopping-list`

**Request Body**:
```json
{
  "items": ["milk", "bread", "butter"],
  "latitude": -36.8485,
  "longitude": 174.7633
}
```

**Response**:
```json
{
  "items": [
    {
      "search_term": "milk",
      "top_matches": [
        {
          "product_name": "Anchor Blue Milk 2L",
          "brand": "Anchor",
          "price": 4.99,
          "supermarket": "PakNSave",
          "store_name": "Pak'n Save Albany",
          "distance_km": 5.2,
          "similarity_score": 0.95
        },
        ...
      ]
    },
    ...
  ]
}
```

---

### Phase 8: Main Entry Point Updates

**File**: `src/main.rs`

Add HTTP server startup:
```rust
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/api/shopping-list", web::post().to(handlers::shopping_list))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

---

## New File Structure

```
src/
├── api/
│   ├── mod.rs
│   ├── routes.rs
│   └── handlers.rs
├── services/
│   ├── mod.rs
│   └── shopping_list_service.rs
├── search/
│   ├── mod.rs
│   ├── product_index.rs      # Tantivy BM25 search index
│   └── typo_corrector.rs     # Edit-distance based typo correction
├── utils/
│   ├── mod.rs
│   └── geo.rs
└── ... (existing files)
```

---

## Key Files to Modify

| File | Changes |
|------|---------|
| `Cargo.toml` | Add actix-web, tantivy, strsim |
| `src/main.rs` | Add HTTP server startup, initialize search index |
| `src/database/queries.rs` | Add joined product+price+store query |
| `src/lib.rs` or `src/main.rs` | Register new modules (api, services, search, utils) |

---

## Verification Plan

1. **Unit tests**: Fuzzy matcher with known product names
2. **Distance test**: Verify Haversine formula with known coordinates
3. **Integration test**: Mock shopping list request, verify response structure
4. **Manual test**:
   - Start server: `cargo run`
   - POST to `http://localhost:8080/api/shopping-list` with sample data
   - Verify top 3 results per item, proper distance filtering

---

## Assumptions

- Database already populated with products, prices, and stores
- Fuzzy matching threshold: ~60% similarity (configurable)
- Woolworths has a single "virtual store" (already in current implementation)
- 20km radius is a hard cutoff (not configurable initially)

---

## ML Alternatives to Fuzzy Matching

For better product matching accuracy, consider these ML approaches:

### Option 1: TF-IDF + Cosine Similarity (Recommended for Rust)
**Complexity**: Low | **Accuracy**: Good | **Speed**: Fast

- Vectorize product names using TF-IDF
- Compare search term vector with product vectors using cosine similarity
- Rust crate: `rust-stemmers` + custom TF-IDF implementation or `tfidf`
- Handles multi-word queries better than edit distance
- Can be precomputed at startup for fast runtime queries

### Option 2: Word Embeddings (FastText/Word2Vec)
**Complexity**: Medium | **Accuracy**: Very Good | **Speed**: Medium

- Pre-train or use pre-trained word embeddings
- Average word vectors for product names → single vector per product
- Compare with cosine similarity
- Rust: Load ONNX model via `ort` (ONNX Runtime) or use `fastembed-rs`
- Captures semantic similarity ("2% milk" ≈ "light milk")

### Option 3: Sentence Transformers (Small BERT)
**Complexity**: High | **Accuracy**: Excellent | **Speed**: Slower

- Use a small model like `all-MiniLM-L6-v2` (~22MB)
- Generates semantic embeddings for entire product names
- Rust: `candle` or `ort` for inference
- Best for understanding intent ("cheap milk" → budget milk options)

### Option 4: BM25 (Probabilistic Retrieval)
**Complexity**: Low | **Accuracy**: Good | **Speed**: Very Fast

- Standard information retrieval algorithm
- Better than TF-IDF for short queries
- Rust crate: `tantivy` (full-text search engine with BM25)
- Good balance of speed and accuracy

### Comparison Table

| Approach | Setup Effort | Runtime Speed | Semantic Understanding | Memory |
|----------|--------------|---------------|------------------------|--------|
| Fuzzy (Levenshtein) | Very Low | Fast | None | Tiny |
| TF-IDF | Low | Fast | Basic | Low |
| BM25 (Tantivy) | Low | Very Fast | Basic | Medium |
| Word Embeddings | Medium | Medium | Good | Medium |
| Sentence Transformer | High | Slow | Excellent | High |

### Chosen Approach: BM25 via Tantivy ✓

**BM25 via Tantivy** was selected for this implementation because:

1. **Best accuracy-to-runtime ratio** for product search
2. **Multi-field search** - handles queries like "anchor trim milk" (brand + product)
3. **Sub-millisecond queries** with inverted index
4. **Battle-tested** - Tantivy is used in production by many Rust projects
5. **Field boosting** - can prioritize brand matches over generic terms

For a supermarket product search with brand-aware queries, BM25 provides excellent accuracy without the complexity or memory overhead of neural models.
