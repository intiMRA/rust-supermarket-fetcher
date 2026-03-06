use chrono::DateTime;
use chrono::offset::Utc;
#[derive(Clone)]
pub struct Token {
    pub token: String,
    pub expiry_time: DateTime<Utc>,
}