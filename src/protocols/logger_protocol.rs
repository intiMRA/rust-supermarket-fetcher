pub trait LoggerProtocol: Send + Sync {
    fn fetching(&self, entity: &str);
    fn fetched(&self, count: usize, entity: &str);
    fn found(&self, count: usize, entity: &str);
    fn fetching_category(&self, category: &str);
    fn fetched_category(&self, count: usize, category: &str);
    fn error(&self, message: &str);
}
