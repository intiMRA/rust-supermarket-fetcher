# Custom Types Module

Shared type definitions and error handling.

## Files

| File | Purpose |
|------|---------|
| `error.rs` | Error types for fetch operations |

## Error Types

### FetchError

Error type for supermarket API operations:

```rust
pub enum FetchError {
    NetworkError(String),
    ParseError(String),
    ApiError(String),
}
```

Used by all fetchers to report failures without panicking.

```rust
async fn get_items(&mut self, store_id: Option<&str>) -> Result<Vec<SuperMarketItem>, FetchError> {
    // ...
}
```
