# API Module

REST API endpoints using Actix-web.

## Files

| File | Purpose |
|------|---------|
| `routes.rs` | Route configuration |
| `handlers.rs` | Request handlers and app state |

## Endpoints

### POST /api/shopping-list

Find cheapest products matching a shopping list.

**Request:**
```json
{
  "items": ["milk", "bread", "butter"],
  "latitude": -36.8485,
  "longitude": 174.7633
}
```

**Response:**
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
          "store_name": "PAK'nSAVE Albany",
          "distance_km": 5.2,
          "similarity_score": 0.85
        }
      ]
    }
  ]
}
```

### GET /api/health

Health check endpoint.

**Response:**
```json
{
  "status": "ok"
}
```

## Running

```bash
cargo run -- serve
# Listening on http://127.0.0.1:8080
```
