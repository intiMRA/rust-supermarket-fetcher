use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use crate::custom_types::size_unit_types::SizeUnit;
use crate::custom_types::supermarket_types::Supermarket;
use crate::models::category::Category;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct SuperMarketItem {
    pub id: String,
    pub name: String,
    pub supermarket: Supermarket,
    pub image_url: String,
    pub price: f64,
    pub brand_name: String,
    pub size: SizeUnit,
    pub category: Category,
}

impl Eq for SuperMarketItem {}

impl PartialEq<Self> for SuperMarketItem {
    fn eq(&self, _other: &Self) -> bool {
        self.name.eq_ignore_ascii_case(&_other.name)
            && self.brand_name.eq(&_other.brand_name)
    }
}

impl PartialOrd<Self> for SuperMarketItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Ord for SuperMarketItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}