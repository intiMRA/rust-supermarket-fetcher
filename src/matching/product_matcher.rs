//! Product matching utilities for deduplication across supermarkets.
//!
//! This module provides embedding generation and similarity functions
//! used by the repository for cross-supermarket product deduplication.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::{Mutex, OnceLock};

/// Global embedding model - initialized once, reused across calls.
static EMBEDDING_MODEL: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

/// Error types for product matching operations.
#[derive(Debug)]
pub enum MatchError {
    Embedding(String),
}

impl std::fmt::Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchError::Embedding(e) => write!(f, "Embedding error: {}", e),
        }
    }
}

impl std::error::Error for MatchError {}

/// Get the embedding model, initializing it if needed.
fn get_model() -> &'static Mutex<TextEmbedding> {
    EMBEDDING_MODEL.get_or_init(|| {
        println!("Initializing embedding model (first use only)...");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )
        .expect("Failed to initialize embedding model");
        Mutex::new(model)
    })
}

/// Calculate cosine similarity between two vectors.
///
/// Cosine similarity measures the cosine of the angle between two vectors,
/// indicating how similar their directions are regardless of magnitude.
///
/// # Formula
///
/// ```text
///                    A · B           Σ(Aᵢ × Bᵢ)
/// cos(θ) = ─────────────────── = ─────────────────────
///           ||A|| × ||B||       √Σ(Aᵢ²) × √Σ(Bᵢ²)
/// ```
///
/// # Returns
///
/// A value between -1.0 and 1.0:
/// - `1.0` = vectors point in the same direction (identical)
/// - `0.0` = vectors are orthogonal (unrelated)
/// - `-1.0` = vectors point in opposite directions
///
/// For normalized embeddings (like those from sentence transformers),
/// values typically range from 0.0 to 1.0.
///
/// # Use Case
///
/// Used to compare semantic embeddings from neural networks. Two product names
/// with similar meanings will have embeddings that point in similar directions,
/// yielding a high cosine similarity score.
///
/// # Example
///
/// ```text
/// "Anchor Butter 500g" embedding ≈ "Anchor NZ Butter 500g" embedding
/// cosine_similarity ≈ 0.92 (high similarity)
///
/// "Anchor Butter 500g" embedding ≠ "Fresh Milk 2L" embedding
/// cosine_similarity ≈ 0.15 (low similarity)
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot_product / (norm_a * norm_b)) as f64
}

/// Convert f32 vector to bytes for database storage.
pub fn f32_vec_to_bytes(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert bytes from database to f32 vector.
pub fn bytes_to_f32_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Generate embeddings for multiple texts efficiently (batch processing).
///
/// This is much faster than generating embeddings one at a time because:
/// - The model can process multiple inputs in parallel
/// - GPU/CPU batch operations are more efficient
/// - Reduces model initialization overhead
///
/// # Arguments
///
/// * `texts` - Slice of text strings to generate embeddings for
///
/// # Returns
///
/// A vector of 384-dimensional f32 embeddings, one per input text.
pub fn generate_embeddings_batch(texts: &[String]) -> Result<Vec<Vec<f32>>, MatchError> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let model_mutex = get_model();
    let mut model = model_mutex
        .lock()
        .map_err(|e| MatchError::Embedding(format!("Failed to lock model: {}", e)))?;

    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let embeddings = model
        .embed(text_refs, None)
        .map_err(|e| MatchError::Embedding(format!("Failed to generate embeddings: {}", e)))?;

    Ok(embeddings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f32_vec_roundtrip() {
        let original = vec![1.0f32, 2.5, -3.14, 0.0];
        let bytes = f32_vec_to_bytes(&original);
        let restored = bytes_to_f32_vec(&bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![1.0f32, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![-1.0f32, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 0.001);
    }
}
