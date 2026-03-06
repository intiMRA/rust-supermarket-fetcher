use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Failed to fetch category '{category}': HTTP {status}")]
    CategoryFetch { category: String, status: u16 },

    #[error("Unexpected API response: {0}")]
    UnexpectedResponse(String),

    #[error("Missing authentication token")]
    MissingToken,
}
