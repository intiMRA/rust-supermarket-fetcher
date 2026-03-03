use crate::custom_types::size_unit_types::SizeUnit;
use crate::custom_types::supermarket_types::Supermarket;

#[derive(Debug)]
pub struct SuperMarketItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) supermarket: Supermarket,
    pub(crate) image_url: String,
    pub(crate) price: f64,
    pub(crate) brand_name: String,
    pub(crate) size: Option<SizeUnit>,
    pub(crate) category: String,
}
