/// Information about a store to query for products.
#[derive(Clone)]
pub struct NearbyStore {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) distance_km: f64,
}