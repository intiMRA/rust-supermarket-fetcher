pub trait LoggerProtocol: Send + Sync {
    fn fetching(&self, entity: &str);
    fn fetched(&self, count: usize, entity: &str);
    fn found(&self, count: usize, entity: &str);
    fn fetching_category(&self, category: &str);
    fn fetched_category(&self, count: usize, category: &str);
    fn error(&self, message: &str);
    fn rate_limit_warning(&self, status: u16, message: &str);
    fn retrying(&self, attempt: u32, max_attempts: u32);
}
