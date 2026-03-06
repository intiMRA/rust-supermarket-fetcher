pub struct Logger {
    prefix: String,
}

impl Logger {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }

    pub fn fetching(&self, entity: &str) {
        println!("[{}] Fetching {}...", self.prefix, entity);
    }

    pub fn fetched(&self, count: usize, entity: &str) {
        println!("[{}] Fetched {} {}", self.prefix, count, entity);
    }

    pub fn found(&self, count: usize, entity: &str) {
        println!("[{}] Found {} {}", self.prefix, count, entity);
    }

    pub fn fetching_category(&self, category: &str) {
        println!("[{}] Fetching items for category: {}...", self.prefix, category);
    }

    pub fn fetched_category(&self, count: usize, category: &str) {
        println!("[{}] Fetched {} items for category: {}", self.prefix, count, category);
    }

    pub fn error(&self, message: &str) {
        eprintln!("[{}] Error: {}", self.prefix, message);
    }
}
