use crate::fetchers::woolworth_fetcher::WoolworthFetcher;
use crate::protocols::super_market_fetcher_protocol::SuperMarketFetcherProtocol;

pub struct FetchController {
    woolworth_fetcher: Box<dyn SuperMarketFetcherProtocol>,
}

impl FetchController {
    pub fn new() -> Self {
        Self {
            woolworth_fetcher: Box::new(WoolworthFetcher::new()),
        }
    }

    pub async fn run(&self) {
        match self.woolworth_fetcher.get_items().await {
            Ok(items) => println!("Fetched: {} items", items.len()),
            Err(e) => eprintln!("Failed to fetch: {}", e),
        }
    }
}

impl Default for FetchController {
    fn default() -> Self {
        Self::new()
    }
}
