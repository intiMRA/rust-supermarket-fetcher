use std::f64::consts::PI;

/// Mean radius of Earth in kilometers (WGS84 approximation).
const EARTH_RADIUS_KM: f64 = 6371.0;

/// Calculate the great-circle distance between two points on Earth using the Haversine formula.
///
/// The Haversine formula determines the shortest distance over the Earth's surface
/// (as the crow flies), giving an "as-the-crow-flies" distance between two points
/// on a sphere from their latitudes and longitudes.
///
/// # Formula
///
/// ```text
/// a = sin²(Δlat/2) + cos(lat1) × cos(lat2) × sin²(Δlon/2)
/// c = 2 × atan2(√a, √(1−a))
/// d = R × c
/// ```
///
/// Where:
/// - `Δlat` = lat2 − lat1 (difference in latitude)
/// - `Δlon` = lon2 − lon1 (difference in longitude)
/// - `R` = Earth's radius (6,371 km mean radius)
/// - `d` = distance between the two points
///
/// # Arguments
///
/// * `lat1`, `lon1` - Latitude and longitude of the first point in degrees
/// * `lat2`, `lon2` - Latitude and longitude of the second point in degrees
///
/// # Returns
///
/// Distance in kilometers (great-circle distance).
///
/// # Accuracy
///
/// The Haversine formula assumes a spherical Earth, which introduces an error
/// of up to 0.3% compared to the WGS84 ellipsoid. For distances under 100km
/// (typical for finding nearby stores), this error is negligible.
///
/// # Example
///
/// ```text
/// Auckland CBD (-36.8485, 174.7633) to Wellington (-41.2865, 174.7762)
/// haversine_distance_km ≈ 493 km
/// ```
///
/// # Use Case
///
/// Used to find stores within a certain radius of the user's location.
/// Stores beyond `MAX_DISTANCE_KM` (20km) are filtered out for NewWorld/PakNSave.
pub fn haversine_distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1 * PI / 180.0;
    let lat2_rad = lat2 * PI / 180.0;
    let delta_lat = (lat2 - lat1) * PI / 180.0;
    let delta_lon = (lon2 - lon1) * PI / 180.0;

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_KM * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_same_point() {
        let distance = haversine_distance_km(-36.8485, 174.7633, -36.8485, 174.7633);
        assert!((distance - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_haversine_auckland_to_wellington() {
        // Auckland: -36.8485, 174.7633
        // Wellington: -41.2865, 174.7762
        let distance = haversine_distance_km(-36.8485, 174.7633, -41.2865, 174.7762);
        // Approximately 493 km
        assert!(distance > 490.0 && distance < 500.0);
    }

    #[test]
    fn test_haversine_short_distance() {
        // Auckland CBD to Albany (~15km north)
        // Auckland CBD: -36.8485, 174.7633
        // Albany: -36.7276, 174.7021
        let distance = haversine_distance_km(-36.8485, 174.7633, -36.7276, 174.7021);
        assert!(distance > 10.0 && distance < 20.0);
    }
}
