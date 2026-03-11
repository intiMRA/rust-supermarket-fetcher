use crate::custom_types::error::FetchError;
use crate::supermarkets::supermarket_types::Supermarket;
use crate::database::{Database, Repository};
use crate::database::repository::ItemWithStore;
use crate::supermarkets::food_stuff::food_stuff_commons::FoodStuff;
use crate::supermarkets::new_world_fetcher::NewWorldFetcher;
use crate::supermarkets::pack_n_save_fetcher::PackNSaveFetcher;
use crate::supermarkets::woolworth_fetcher::WoolworthFetcher;
use crate::supermarkets::super_market_fetcher_trait::SuperMarketFetcherTrait;
use crate::loggers::logger::Logger;
use crate::loggers::parse_logger::clear_parse_log;
use crate::loggers::empty_brand_logger::clear_empty_brand_log;
use crate::supermarkets::models::super_market_item::SuperMarketItem;
use crate::supermarkets::models::store::Store;

/// Result of fetching from a supermarket
pub struct FetchResult {
    pub supermarket: Supermarket,
    pub store: Store,
    pub items: Vec<SuperMarketItem>,
}

pub struct FetchController;

impl FetchController {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&self) {
        // Create data directory if it doesn't exist
        tokio::fs::create_dir_all("data").await.unwrap();

        // Clear logs for this run
        clear_parse_log();
        clear_empty_brand_log();

        // Open database
        let db = Database::open("data/supermarket.db").expect("Failed to open database");
        let repo = Repository::new(&db);

        println!("Database opened: data/supermarket.db");
        println!("Parse warnings will be logged to: data/parse_warnings.log");
        println!("Empty brand items will be logged to: data/empty_brand.log");

        // PHASE 1: Fetch all supermarkets in parallel (network I/O)
        println!("\n=== PHASE 1: Fetching from APIs ===");
        let (woolworth_result, new_world_result, pack_n_save_result) = tokio::join!(
            Self::fetch_woolworth(),
            Self::fetch_new_world(),
            Self::fetch_pack_n_save(),
        );

        // PHASE 2: Collect all results
        println!("\n=== PHASE 2: Collecting results ===");
        let mut all_results: Vec<FetchResult> = Vec::new();

        for fetch_result in [woolworth_result, new_world_result, pack_n_save_result] {
            if let Ok(results) = fetch_result {
                for result in results {
                    println!(
                        "[{:?}] {} items from {}",
                        result.supermarket,
                        result.items.len(),
                        result.store.name
                    );
                    all_results.push(result);
                }
            }
        }

        let total_items: usize = all_results.iter().map(|r| r.items.len()).sum();
        println!("Total: {} items from {} stores", total_items, all_results.len());

        // PHASE 3: In-memory deduplication + batch database insert
        println!("\n=== PHASE 3: Deduplication & Database Insert ===");

        // Build ItemWithStore references
        let items_with_stores: Vec<ItemWithStore<'_>> = all_results
            .iter()
            .flat_map(|result| {
                result.items.iter().map(move |item| ItemWithStore {
                    item,
                    store: &result.store,
                    supermarket: result.supermarket,
                })
            })
            .collect();

        // Single batch insert with in-memory deduplication
        repo.insert_all_items(&items_with_stores)
            .expect("Failed to insert items");

        // Print final stats
        if let Ok(stats) = repo.get_deduplication_stats() {
            println!("\n=== Final Statistics ===");
            println!("{}", stats);
        }

        println!("\nFetch complete! Data saved to data/supermarket.db");
    }

    async fn fetch_woolworth() -> Result<Vec<FetchResult>, FetchError> {
        println!("[Woolworths] Fetching items...");

        // Each task creates its own fetcher - no sharing, no mutex
        let mut fetcher = WoolworthFetcher::new(Logger::new("Woolworths"));

        let woolworth_store = Store {
            id: "default".to_string(),
            name: "Woolworths (All Stores)".to_string(),
            address: String::new(),
            latitude: 0.0,
            longitude: 0.0,
        };

        let items = fetcher.get_items(None).await?;

        println!("[Woolworths] Fetched {} items", items.len());

        Ok(vec![FetchResult {
            supermarket: Supermarket::Woolworth,
            store: woolworth_store,
            items,
        }])
    }

    async fn fetch_new_world() -> Result<Vec<FetchResult>, FetchError> {
        // Each task creates its own fetcher - no sharing, no mutex
        let mut fetcher = NewWorldFetcher::new(
            Logger::new("NewWorld"),
            FoodStuff::new_world(),
        );

        let stores = fetcher.get_stores().await?;
        let num_stores = stores.len();
        let mut results = Vec::new();

        for (i, store) in stores.into_iter().enumerate() {
            println!("[NewWorld] Fetching store {} of {}: {}", i + 1, num_stores, store.name);

            let items = fetcher.get_items(Some(store.id.as_str())).await?;

            println!("[NewWorld] Fetched {} items for {}", items.len(), store.name);

            results.push(FetchResult {
                supermarket: Supermarket::NewWorld,
                store,
                items,
            });
        }

        Ok(results)
    }

    async fn fetch_pack_n_save() -> Result<Vec<FetchResult>, FetchError> {
        // Each task creates its own fetcher - no sharing, no mutex
        let mut fetcher = PackNSaveFetcher::new(
            Logger::new("PackNSave"),
            FoodStuff::pack_n_save(),
        );

        let stores = fetcher.get_stores().await?;
        let num_stores = stores.len();
        let mut results = Vec::new();

        for (i, store) in stores.into_iter().enumerate() {
            println!("[PakNSave] Fetching store {} of {}: {}", i + 1, num_stores, store.name);

            let items = fetcher.get_items(Some(store.id.as_str())).await?;

            println!("[PakNSave] Fetched {} items for {}", items.len(), store.name);

            results.push(FetchResult {
                supermarket: Supermarket::PakNSave,
                store,
                items,
            });
        }

        Ok(results)
    }
}

impl Default for FetchController {
    fn default() -> Self {
        Self::new()
    }
}
