# Utils Module

Utility functions used across the application.

## Files

| File | Purpose |
|------|---------|
| `geo.rs` | Geographic calculations |

## Geo Utilities

### Haversine Distance

Calculates distance between two GPS coordinates in kilometers.

```rust
use crate::utils::geo::haversine_distance_km;

let distance = haversine_distance_km(
    -36.8485, 174.7633,  // Auckland
    -41.2866, 174.7756,  // Wellington
);
// ~493 km
```

Used for:
- Filtering stores within 20km radius
- Displaying distance to stores in API responses
