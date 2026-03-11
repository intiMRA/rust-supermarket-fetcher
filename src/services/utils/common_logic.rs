use crate::database::Queries;
use crate::services::common_models::nearby_store::NearbyStore;
use crate::supermarkets::supermarket_types::Supermarket;
use crate::utils::geo::haversine_distance_km;

/// Maximum distance in km for NewWorld and PakNSave stores.
const MAX_DISTANCE_KM: f64 = 20.0;

/// Find stores to query for products.
pub(crate) fn find_stores_to_query(
    queries: &Queries<'_>,
    user_lat: f64,
    user_lon: f64,
) -> Vec<NearbyStore> {
    let mut stores_to_query = Vec::new();
    let db_stores = queries.get_all_stores();

    for store in db_stores {
        let supermarket = Supermarket::from_id(store.supermarket_id);
        let has_single_store = supermarket.map(|s| s.has_single_store()).unwrap_or(false);

        if has_single_store {
            // Supermarkets with a single virtual store (e.g., Woolworths) - no distance filtering
            stores_to_query.push(NearbyStore {
                id: store.id,
                name: store.name,
                distance_km: 0.0,
            });
        } else {
            // Supermarkets with physical stores - filter by distance
            let distance = haversine_distance_km(
                user_lat,
                user_lon,
                store.latitude,
                store.longitude,
            );

            if distance <= MAX_DISTANCE_KM {
                stores_to_query.push(NearbyStore {
                    id: store.id,
                    name: store.name,
                    distance_km: distance,
                });
            }
        }
    }

    stores_to_query
}