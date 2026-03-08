use std::f64::consts::PI;

const EARTH_RADIUS_KM: f64 = 6371.0;

/// Calculate the great-circle distance between two points on Earth using the Haversine formula.
///
/// # Arguments
/// * `lat1`, `lon1` - Latitude and longitude of the first point in degrees
/// * `lat2`, `lon2` - Latitude and longitude of the second point in degrees
///
/// # Returns
/// Distance in kilometers
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
