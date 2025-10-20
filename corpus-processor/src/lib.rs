pub mod chunking;
pub mod embeddings;
pub mod loaders;
pub mod processor;
pub mod qdrant;
pub mod test_seams;

// Make test utilities available for integration tests
#[cfg(test)]
pub use test_seams::test_support;
