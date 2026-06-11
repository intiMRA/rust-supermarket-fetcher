use std::sync::Mutex;
use crate::loggers::logger_trait::LoggerTrait;

pub struct Logger {
    prefix: String,
    store_context: Mutex<Option<String>>,
}

impl Logger {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            store_context: Mutex::new(None),
        }
    }

    fn format_prefix(&self) -> String {
        let ctx = self.store_context.lock().unwrap();
        match ctx.as_ref() {
            Some(store) => format!("[{}: {}]", self.prefix, store),
            None => format!("[{}]", self.prefix),
        }
    }
}

impl LoggerTrait for Logger {
    fn set_store_context(&self, store_name: &str) {
        *self.store_context.lock().unwrap() = Some(store_name.to_string());
    }

    fn clear_store_context(&self) {
        *self.store_context.lock().unwrap() = None;
    }

    fn fetching(&self, entity: &str) {
        println!("{} Fetching {}...", self.format_prefix(), entity);
    }

    fn fetched(&self, count: usize, entity: &str) {
        println!("{} Fetched {} {}", self.format_prefix(), count, entity);
    }

    fn found(&self, count: usize, entity: &str) {
        println!("{} Found {} {}", self.format_prefix(), count, entity);
    }

    fn fetching_category(&self, _category: &str) {
        // Silent - too verbose to log each category
    }

    fn fetched_category(&self, _count: usize, _category: &str) {
        // Silent - too verbose to log each category
    }

    fn error(&self, message: &str) {
        eprintln!("{} Error: {}", self.format_prefix(), message);
    }

    fn rate_limit_warning(&self, status: u16, message: &str) {
        eprintln!("\n⚠️  {} RATE LIMITED (HTTP {}): {}", self.format_prefix(), status, message);
        eprintln!("⚠️  {} The API may be blocking requests. Consider increasing delays.\n", self.format_prefix());
    }

    fn retrying(&self, attempt: u32, max_attempts: u32) {
        println!("{} Retrying... (attempt {}/{})", self.format_prefix(), attempt, max_attempts);
    }
}
