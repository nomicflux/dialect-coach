use anyhow::Result;
use dialect_coach_shared::Dialect;
use std::sync::Arc;

#[path = "../src/agent_service.rs"]
mod agent_service;

#[path = "../src/qdrant_service.rs"]
mod qdrant_service;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Initialize services
    let qdrant = qdrant_service::QdrantService::from_env().await?;
    let agent = agent_service::AgentService::from_env(Arc::new(qdrant))?;

    println!("Testing Rig Agent with different dialects...\n");
    println!("{}", "=".repeat(80));

    // Test 1: Argentinian Spanish
    println!("\n🇦🇷 ARGENTINIAN SPANISH");
    println!("{}", "-".repeat(80));
    let response = agent
        .generate_response(
            "Hola, ¿cómo estás? Quiero aprender argentino.",
            Dialect::SpanishArgentinian,
            &[],
        )
        .await?;
    println!("User: Hola, ¿cómo estás? Quiero aprender argentino.");
    println!("Agent: {}\n", response);

    // Test 2: Colombian Spanish
    println!("\n🇨🇴 COLOMBIAN SPANISH");
    println!("{}", "-".repeat(80));
    let response = agent
        .generate_response(
            "¿Qué tal? Me gustaría practicar colombiano.",
            Dialect::SpanishColombian,
            &[],
        )
        .await?;
    println!("User: ¿Qué tal? Me gustaría practicar colombiano.");
    println!("Agent: {}\n", response);

    // Test 3: Caribbean Spanish (Puerto Rican)
    println!("\n🇵🇷 CARIBBEAN SPANISH (Puerto Rican)");
    println!("{}", "-".repeat(80));
    let response = agent
        .generate_response(
            "Wepa! Quiero hablar como boricua.",
            Dialect::SpanishCaribbean,
            &[],
        )
        .await?;
    println!("User: Wepa! Quiero hablar como boricua.");
    println!("Agent: {}\n", response);

    // Test 4: With conversation history
    println!("\n💬 WITH CONVERSATION HISTORY");
    println!("{}", "-".repeat(80));
    let history = vec![
        "User: Hola, ¿cómo estás?".to_string(),
        "Agent: ¡Todo bien, che! ¿Y vos?".to_string(),
    ];
    let response = agent
        .generate_response(
            "Estoy bien también. ¿Podés enseñarme más frases argentinas?",
            Dialect::SpanishArgentinian,
            &history,
        )
        .await?;
    println!("Previous:");
    for msg in &history {
        println!("  {}", msg);
    }
    println!("User: Estoy bien también. ¿Podés enseñarme más frases argentinas?");
    println!("Agent: {}\n", response);

    println!("{}", "=".repeat(80));
    println!("\n✅ All tests completed successfully!");

    Ok(())
}
