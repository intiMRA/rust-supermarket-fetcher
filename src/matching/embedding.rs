//! Core embedding functionality for semantic matching.
//!
//! This module provides the shared embedding infrastructure used by both
//! product deduplication (ingestion) and product search (query time).

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::{Mutex, OnceLock};

/// Global embedding model - initialized once, reused across all calls.
static EMBEDDING_MODEL: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

/// Trait for types that can be converted to embedding text.
///
/// Implement this for any type that should be semantically matchable.
pub trait Embeddable {
    /// Convert to text representation for embedding generation.
    fn to_embedding_text(&self) -> String;
}

/// Trait for semantic similarity operations.
pub trait SimilarityScorer {
    /// Calculate similarity between two embeddings.
    fn score(&self, a: &[f32], b: &[f32]) -> f64;

    /// Get the similarity threshold for matching.
    fn threshold(&self) -> f64;

    /// Check if two embeddings are similar enough to match.
    fn is_match(&self, a: &[f32], b: &[f32]) -> bool {
        self.score(a, b) >= self.threshold()
    }
}

/// Cosine similarity scorer.
///
/// Uses cosine similarity to compare embeddings, which measures
/// the cosine of the angle between two vectors.
pub struct CosineSimilarity {
    threshold: f64,
}

impl CosineSimilarity {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }
}

impl SimilarityScorer for CosineSimilarity {
    fn score(&self, a: &[f32], b: &[f32]) -> f64 {
        cosine_similarity(a, b)
    }

    fn threshold(&self) -> f64 {
        self.threshold
    }
}

/// Embedding service for generating and comparing embeddings.
///
/// This is the main entry point for embedding operations.
/// Uses a lazily-initialized singleton model.
pub struct EmbeddingService;

impl EmbeddingService {
    /// Get the singleton embedding model.
    fn model() -> &'static Mutex<TextEmbedding> {
        EMBEDDING_MODEL.get_or_init(|| {
            println!("Initializing embedding model (first use only)...");
            let model = TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                    .with_show_download_progress(true),
            )
            .expect("Failed to initialize embedding model");
            Mutex::new(model)
        })
    }

    /// Generate embedding for a single text.
    pub fn generate(text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let embeddings = Self::generate_batch(&[text.to_string()])?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }

    /// Generate embeddings for multiple texts efficiently.
    ///
    /// Batch processing is much faster than generating one at a time.
    pub fn generate_batch(texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let model_mutex = Self::model();
        let mut model = model_mutex
            .lock()
            .map_err(|e| EmbeddingError::Lock(e.to_string()))?;

        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = model
            .embed(text_refs, None)
            .map_err(|e| EmbeddingError::Generation(e.to_string()))?;

        Ok(embeddings)
    }

    /// Generate embeddings for a slice of Embeddable items.
    pub fn generate_for<T: Embeddable>(items: &[T]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let texts: Vec<String> = items.iter().map(|item| item.to_embedding_text()).collect();
        Self::generate_batch(&texts)
    }
}

/// Errors that can occur during embedding operations.
#[derive(Debug)]
pub enum EmbeddingError {
    Lock(String),
    Generation(String),
}

impl std::fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingError::Lock(e) => write!(f, "Failed to lock embedding model: {}", e),
            EmbeddingError::Generation(e) => write!(f, "Failed to generate embedding: {}", e),
        }
    }
}

impl std::error::Error for EmbeddingError {}

/// Calculate cosine similarity between two vectors.
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
/// - `1.0` = identical direction
/// - `0.0` = orthogonal (unrelated)
/// - `-1.0` = opposite directions
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
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![-1.0f32, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_scorer_is_match() {
        let scorer = CosineSimilarity::new(0.8);
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![1.0f32, 0.0, 0.0];
        assert!(scorer.is_match(&a, &b));
    }

    struct TestProduct {
        name: String,
        brand: String,
    }

    impl Embeddable for TestProduct {
        fn to_embedding_text(&self) -> String {
            if self.brand.is_empty() {
                self.name.clone()
            } else {
                format!("{} {}", self.brand, self.name)
            }
        }
    }

    #[test]
    fn test_embeddable_trait() {
        let product = TestProduct {
            name: "Butter 500g".to_string(),
            brand: "Anchor".to_string(),
        };
        assert_eq!(product.to_embedding_text(), "Anchor Butter 500g");
    }
}
