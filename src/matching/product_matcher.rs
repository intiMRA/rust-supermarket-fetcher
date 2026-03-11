//! Product matching for deduplication during data ingestion.
//!
//! Re-exports embedding utilities used by the repository.

pub use super::embedding::{
    cosine_similarity, f32_vec_to_bytes, bytes_to_f32_vec,
    EmbeddingService, EmbeddingError, Embeddable,
};

/// Error type for backward compatibility.
pub type MatchError = EmbeddingError;

/// Generate embeddings for multiple texts (backward compatibility wrapper).
pub fn generate_embeddings_batch(texts: &[String]) -> Result<Vec<Vec<f32>>, MatchError> {
    EmbeddingService::generate_batch(texts)
}
