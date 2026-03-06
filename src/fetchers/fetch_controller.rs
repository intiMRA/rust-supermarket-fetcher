use crate::fetchers::food_stuff::food_stuff_commons::FoodStuff;
use crate::fetchers::new_world_fetcher::NewWorldFetcher;
use crate::fetchers::pack_n_save_fetcher::PackNSaveFetcher;
use crate::fetchers::woolworth_fetcher::WoolworthFetcher;
use crate::logger::Logger;
use crate::protocols::super_market_fetcher_protocol::SuperMarketFetcherProtocol;

pub struct FetchController {
    woolworth_fetcher: Box<dyn SuperMarketFetcherProtocol>,
    new_world_fetcher: Box<dyn SuperMarketFetcherProtocol>,
    pack_n_save_fetcher: Box<dyn SuperMarketFetcherProtocol>,
}

impl FetchController {
    pub fn new() -> Self {
        Self {
            woolworth_fetcher: Box::new(WoolworthFetcher::new(Box::new(Logger::new("Woolworths")))),
            new_world_fetcher: Box::new(NewWorldFetcher::new(
                Box::new(Logger::new("NewWorld")),
                Box::new(FoodStuff::new_world()),
            )),
            pack_n_save_fetcher: Box::new(PackNSaveFetcher::new(
                Box::new(Logger::new("PackNSave")),
                Box::new(FoodStuff::pack_n_save()),
            )),
        }
    }

    pub async fn run(&mut self) {
        // Fetch from NewWorld
        let wines = self.new_world_fetcher.get_items_for_category(None, "Red Cask Wine").await.unwrap();
        println!("[NewWorld] Fetched: {} items", wines.len());

        // Fetch from NewWorld
        let wines = self.pack_n_save_fetcher.get_items_for_category(None, "Red Cask Wine").await.unwrap();
        println!("[PackNSave] Fetched: {} items", wines.len());

        // Fetch from Woolworths
        let woolworth_wines = self.woolworth_fetcher.get_items_for_category(None, "cask-wine-2l").await.unwrap();
        println!("[Woolworths] Fetched: {} items", woolworth_wines.len());
    }
}

impl Default for FetchController {
    fn default() -> Self {
        Self::new()
    }
}
