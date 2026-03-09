use crate::custom_types::supermarket_types::Supermarket;
use crate::database::{Database, Repository};
use crate::fetchers::food_stuff::food_stuff_commons::FoodStuff;
use crate::fetchers::new_world_fetcher::NewWorldFetcher;
use crate::fetchers::pack_n_save_fetcher::PackNSaveFetcher;
use crate::fetchers::woolworth_fetcher::WoolworthFetcher;
use crate::logger::Logger;
use crate::traits::super_market_fetcher_trait::SuperMarketFetcherTrait;

pub struct FetchController {
    woolworth_fetcher: Box<dyn SuperMarketFetcherTrait>,
    new_world_fetcher: Box<dyn SuperMarketFetcherTrait>,
    pack_n_save_fetcher: Box<dyn SuperMarketFetcherTrait>,
}

impl FetchController {
    pub fn new() -> Self {
        Self {
            woolworth_fetcher: Box::new(WoolworthFetcher::new(Logger::new("Woolworths"))),
            new_world_fetcher: Box::new(NewWorldFetcher::new(
                Logger::new("NewWorld"),
                FoodStuff::new_world(),
            )),
            pack_n_save_fetcher: Box::new(PackNSaveFetcher::new(
                Logger::new("PackNSave"),
                FoodStuff::pack_n_save(),
            )),
        }
    }

    pub async fn run(&mut self) {
        // Create data directory if it doesn't exist
        tokio::fs::create_dir_all("data").await.unwrap();

        // Open database
        let db = Database::open("data/supermarket.db").expect("Failed to open database");
        let repo = Repository::new(&db);

        println!("Database opened: data/supermarket.db");

        // Fetch from NewWorld
        let new_world_stores = self.new_world_fetcher.get_stores().await.unwrap();
        let num_stores = new_world_stores.len();
        for (i, store) in new_world_stores.iter().enumerate() {
            println!("[NewWorld] Fetching store {} of {}: {}", i + 1, num_stores, store.name);

            // Insert store into database
            repo.insert_store(store, Supermarket::NewWorld)
                .expect("Failed to insert store");

            // Fetch and insert items
            let items = self.new_world_fetcher
                .get_items(Some(store.id.as_str()))
                .await
                .unwrap();

            for item in &items {
                repo.insert_item_with_price(item, &store.id)
                    .expect("Failed to insert item");
            }

            println!("[NewWorld] Inserted {} items for {}", items.len(), store.name);
        }

        // Fetch from PackNSave
        let pack_n_save_stores = self.pack_n_save_fetcher.get_stores().await.unwrap();
        let num_stores = pack_n_save_stores.len();
        for (i, store) in pack_n_save_stores.iter().enumerate() {
            println!("[PakNSave] Fetching store {} of {}: {}", i + 1, num_stores, store.name);

            // Insert store into database
            repo.insert_store(store, Supermarket::PakNSave)
                .expect("Failed to insert store");

            // Fetch and insert items
            let items = self.pack_n_save_fetcher
                .get_items(Some(store.id.as_str()))
                .await
                .unwrap();

            for item in &items {
                repo.insert_item_with_price(item, &store.id)
                    .expect("Failed to insert item");
            }

            println!("[PakNSave] Inserted {} items for {}", items.len(), store.name);
        }

        // Fetch from Woolworths (single store, uniform pricing)
        println!("[Woolworths] Fetching items...");

        // Woolworths uses a default store since prices are uniform
        let woolworth_store = crate::models::store::Store {
            id: "default".to_string(),
            name: "Woolworths (All Stores)".to_string(),
            address: String::new(),
            latitude: 0.0,
            longitude: 0.0,
        };
        repo.insert_store(&woolworth_store, Supermarket::Woolworth)
            .expect("Failed to insert Woolworths store");

        let items = self.woolworth_fetcher
            .get_items(None)
            .await
            .unwrap();

        for item in &items {
            repo.insert_item_with_price(item, "default")
                .expect("Failed to insert item");
        }

        println!("[Woolworths] Inserted {} items", items.len());
        println!("\nFetch complete! Data saved to data/supermarket.db");
    }
}

impl Default for FetchController {
    fn default() -> Self {
        Self::new()
    }
}
