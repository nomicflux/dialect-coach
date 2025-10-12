use anyhow::Result;

#[path = "../src/qdrant_service.rs"]
mod qdrant_service;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    println!("Creating Qdrant field index for 'dialect'...\n");

    let qdrant = qdrant_service::QdrantService::from_env().await?;

    // Create index on the dialect field
    qdrant.create_field_index("dialect").await?;

    println!("\n✅ Index created successfully!");
    println!("You can now search/filter by dialect field.");

    Ok(())
}
