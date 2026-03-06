use serde::Deserialize;

#[derive(Deserialize)]
pub struct StoresResponse {
    pub stores: Vec<Store>,
}

#[derive(Deserialize)]
pub struct Store {
    pub id: String,
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
}