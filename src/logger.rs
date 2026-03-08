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

    fn fetching_category(&self, _category: &str) {
        // Silent - too verbose to log each category
    }

    fn fetched_category(&self, _count: usize, _category: &str) {
        // Silent - too verbose to log each category
    }

    fn error(&self, message: &str) {
        eprintln!("[{}] Error: {}", self.prefix, message);
    }

    fn rate_limit_warning(&self, status: u16, message: &str) {
        eprintln!("\n⚠️  [{}] RATE LIMITED (HTTP {}): {}", self.prefix, status, message);
        eprintln!("⚠️  [{}] The API may be blocking requests. Consider increasing delays.\n", self.prefix);
    }

    fn retrying(&self, attempt: u32, max_attempts: u32) {
        println!("[{}] Retrying... (attempt {}/{})", self.prefix, attempt, max_attempts);
    }
}
