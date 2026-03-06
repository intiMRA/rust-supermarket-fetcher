use crate::protocols::logger_protocol::LoggerProtocol;

pub struct Logger {
    prefix: String,
}

impl Logger {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
}

impl LoggerProtocol for Logger {
    fn fetching(&self, entity: &str) {
        println!("[{}] Fetching {}...", self.prefix, entity);
    }

    fn fetched(&self, count: usize, entity: &str) {
        println!("[{}] Fetched {} {}", self.prefix, count, entity);
    }

    fn found(&self, count: usize, entity: &str) {
        println!("[{}] Found {} {}", self.prefix, count, entity);
    }

    fn fetching_category(&self, category: &str) {
        println!("[{}] Fetching items for category: {}...", self.prefix, category);
    }

    fn fetched_category(&self, count: usize, category: &str) {
        println!("[{}] Fetched {} items for category: {}", self.prefix, count, category);
    }

    fn error(&self, message: &str) {
        eprintln!("[{}] Error: {}", self.prefix, message);
    }
}
