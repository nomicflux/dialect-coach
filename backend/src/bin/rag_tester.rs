use anyhow::Result;
use dialect_coach_backend::{
    agent_service::AgentService, embedding_service::EmbeddingService,
    qdrant_service::QdrantService, rag_config::RAGConfig, test_utils::TestStats,
    test_utils::run_self_chat_test,
};
use dialect_coach_shared::{Dialect, Formality};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Initializing services...");
    let qdrant = Arc::new(QdrantService::from_env().await?);
    let embeddings = Arc::new(EmbeddingService::new()?);
    let agent = AgentService::from_env(qdrant.clone(), embeddings.clone())?;

    println!("Starting RAG configuration tests...\n");

    let configs = get_test_configs();
    let dialects = get_test_dialects();

    let mut results = Vec::new();

    for config in &configs {
        for (dialect, formality, seed) in &dialects {
            print_test_header(config, dialect);

            let stats = run_self_chat_test(
                &agent,
                &qdrant,
                &embeddings,
                *config,
                *dialect,
                *formality,
                seed,
            )
            .await?;

            print_test_result(&stats);
            results.push(stats);
        }
    }

    print_summary(&results);

    Ok(())
}

fn get_test_configs() -> Vec<RAGConfig> {
    vec![
        RAGConfig::new(0, 0),
        RAGConfig::new(5, 0),
        RAGConfig::new(25, 0),
        RAGConfig::new(0, 5),
        RAGConfig::new(0, 25),
        RAGConfig::new(5, 5),
        RAGConfig::new(5, 25),
        RAGConfig::new(25, 5),
        RAGConfig::new(25, 25),
    ]
}

fn get_test_dialects() -> Vec<(Dialect, Formality, &'static str)> {
    vec![
        (
            Dialect::SpanishArgentinian,
            Formality::DialectRich,
            "¿Che, qué hacés?",
        ),
        (
            Dialect::SpanishCuban,
            Formality::DialectRich,
            "¿Qué bola asere?",
        ),
        (Dialect::ArabicLevantine, Formality::DialectRich, "كيفك؟"),
        (Dialect::ArabicGulf, Formality::DialectRich, "شلونك؟"),
        (
            Dialect::FrenchQuebecois,
            Formality::DialectRich,
            "Comment ça va?",
        ),
    ]
}

fn print_test_header(config: &RAGConfig, dialect: &Dialect) {
    println!(
        "[TEST] Testing: {}/{} conversation/random on {}",
        config.num_conversation_documents,
        config.num_random_documents,
        dialect.name()
    );
}

fn print_test_result(stats: &TestStats) {
    println!(
        "[TEST] Cosine MSE - Mean: {:.6}, Median: {:.6}, Variance: {:.8}",
        stats.cosine_mse_mean, stats.cosine_mse_median, stats.cosine_mse_variance
    );
    println!(
        "[TEST] L2 MSE - Mean: {:.6}, Median: {:.6}, Variance: {:.8}",
        stats.l2_mse_mean, stats.l2_mse_median, stats.l2_mse_variance
    );
}

fn print_summary(results: &[TestStats]) {
    println!("\n[TEST][SUMMARY] === SUMMARY ===\n");
    println!("[TEST][SUMMARY] Configurations by cosine MSE (lower is better):\n");

    let mut sorted = results.to_vec();
    sorted.sort_by(|a, b| a.cosine_mse_mean.partial_cmp(&b.cosine_mse_mean).unwrap());

    for (i, stats) in sorted.iter().enumerate() {
        println!(
            "[TEST][SUMMARY] {}. {}/{} on {} - Cosine: {:.6}, L2: {:.6}",
            i + 1,
            stats.config.num_conversation_documents,
            stats.config.num_random_documents,
            stats.dialect,
            stats.cosine_mse_mean,
            stats.l2_mse_mean
        );
    }
}
