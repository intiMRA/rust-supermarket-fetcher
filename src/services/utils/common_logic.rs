use crate::database::Queries;
use crate::services::common_models::nearby_store::NearbyStore;
use crate::supermarkets::supermarket_types::Supermarket;
use crate::utils::geo::haversine_distance_km;

/// Maximum distance in km for filtering nearby stores.
const MAX_DISTANCE_KM: f64 = 20.0;

/// Find stores to query for products.
///
/// For supermarkets with per-store pricing (NewWorld, PakNSave), returns
/// physical stores within 20km.
///
/// For uniform-pricing supermarkets (Woolworths), checks if any physical
/// store is within 20km. If so, includes the "default" pricing store
/// (with the nearest physical store's name and distance).
pub(crate) fn find_stores_to_query(
    queries: &Queries<'_>,
    user_lat: f64,
    user_lon: f64,
) -> Vec<NearbyStore> {
    let mut stores_to_query = Vec::new();
    let db_stores = queries.get_all_stores();

    // Track the closest Woolworths physical store for distance check
    let mut closest_woolworths: Option<(String, f64, f64, f64)> = None;

    for store in &db_stores {
        let supermarket = Supermarket::from_id(store.supermarket_id);
        let has_uniform_pricing = supermarket.map(|s| s.has_single_store()).unwrap_or(false);

        if has_uniform_pricing {
            // Skip "default" virtual store and physical stores for now.
            // Physical stores are only used to check proximity.
            if store.id != "default" {
                let distance = haversine_distance_km(
                    user_lat, user_lon, store.latitude, store.longitude,
                );
                if distance <= MAX_DISTANCE_KM {
                    let is_closer = match &closest_woolworths {
                        Some((_, d, _, _)) => distance < *d,
                        None => true,
                    };
                    if is_closer {
                        closest_woolworths = Some((
                            store.name.clone(),
                            distance,
                            store.latitude,
                            store.longitude,
                        ));
                    }
                }
            }
        } else {
            // Supermarkets with per-store pricing - filter by distance
            let distance = haversine_distance_km(
                user_lat, user_lon, store.latitude, store.longitude,
            );

            if distance <= MAX_DISTANCE_KM {
                stores_to_query.push(NearbyStore {
                    id: store.id.clone(),
                    name: store.name.clone(),
                    distance_km: distance,
                    latitude: store.latitude,
                    longitude: store.longitude,
                });
            }
        }
    }

    // If a Woolworths physical store is nearby, include the "default" store
    // so that price queries find the uniform Woolworths prices.
    // Use the closest physical store's name and coordinates.
    if let Some((name, distance, lat, lon)) = closest_woolworths {
        stores_to_query.push(NearbyStore {
            id: "default".to_string(),
            name,
            distance_km: distance,
            latitude: lat,
            longitude: lon,
        });
    }

    stores_to_query
}