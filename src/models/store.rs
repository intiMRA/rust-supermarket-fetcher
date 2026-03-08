use serde::Deserialize;

#[derive(Deserialize)]
pub struct StoresResponse {
    pub stores: Vec<Store>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Store {
    pub id: String,
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
}

// -----------------------------------------------------------------------------
// Woolworths Store API Response Types
// -----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WoolworthsStoresResponse {
    pub site_detail: Vec<WoolworthsSiteDetail>,
}

#[derive(Debug, Deserialize)]
pub struct WoolworthsSiteDetail {
    pub site: WoolworthsSite,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WoolworthsSite {
    pub id: i64,
    pub name: String,
    pub address_line1: String,
    #[serde(default)]
    pub address_line2: Option<String>,
    pub suburb: String,
    pub postcode: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl From<WoolworthsSite> for Store {
    fn from(site: WoolworthsSite) -> Self {
        let address = if let Some(line2) = site.address_line2 {
            format!("{}, {}, {} {}", site.address_line1, line2, site.suburb, site.postcode)
        } else {
            format!("{}, {} {}", site.address_line1, site.suburb, site.postcode)
        };

        Store {
            id: site.id.to_string(),
            name: site.name,
            address,
            latitude: site.latitude,
            longitude: site.longitude,
        }
    }
}