use crate::custom_types::size_unit_types::SizeUnit;
use crate::custom_types::supermarket_types::Supermarket;

#[allow(dead_code)]
#[derive(Debug)]
pub struct SuperMarketItem {
    pub id: String,
    pub name: String,
    pub supermarket: Supermarket,
    pub image_url: String,
    pub price: f64,
    pub brand_name: String,
    pub size: Option<SizeUnit>,
    pub category: String,
}
