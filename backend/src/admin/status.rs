use crate::admin;
use crate::state::AppState;
use axum::{Json, extract::State};
use chrono::Utc;

/// Get admin status from all monitors
pub async fn get_admin_status(
    State(state): State<AppState>,
) -> Json<admin::types::AdminStatusResponse> {
    let timestamp = Utc::now().to_rfc3339();
    let anthropic = fetch_anthropic_stats().await;

    let elevenlabs = fetch_elevenlabs_stats().await;
    let qdrant = error_text(admin::qdrant_monitor::get_qdrant_stats(&state.qdrant).await);

    Json(admin::types::AdminStatusResponse {
        timestamp,
        anthropic,
        elevenlabs,
        qdrant,
    })
}

/// The stats, or the full error chain for the dashboard to show.
fn error_text<T>(stats: anyhow::Result<T>) -> Result<T, String> {
    stats.map_err(|e| format!("{:#}", e))
}

async fn fetch_elevenlabs_stats() -> Result<admin::types::ElevenLabsStats, String> {
    match std::env::var("ELEVEN_LABS_API_KEY") {
        Ok(api_key) => error_text(admin::elevenlabs_monitor::get_elevenlabs_usage(&api_key).await),
        Err(_) => Err("ELEVEN_LABS_API_KEY not set".to_string()),
    }
}

async fn fetch_anthropic_stats() -> admin::types::AnthropicStats {
    match std::env::var("ANTHROPIC_ADMIN_API_KEY") {
        Ok(api_key) => fetch_with_key(&api_key).await,
        Err(_) => anthropic_stats_error("ANTHROPIC_ADMIN_API_KEY not set"),
    }
}

async fn fetch_with_key(api_key: &str) -> admin::types::AnthropicStats {
    match admin::anthropic_monitor::get_anthropic_stats(api_key).await {
        Ok(stats) => stats,
        Err(e) => anthropic_stats_error(&format!("API error: {}", e)),
    }
}

fn anthropic_stats_error(error: &str) -> admin::types::AnthropicStats {
    admin::types::AnthropicStats {
        uncached_input_tokens: None,
        cached_input_tokens: None,
        cache_creation_tokens: None,
        output_tokens: None,
        total_cost_usd: None,
        error: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_text_keeps_the_whole_error() {
        let failed = anyhow::anyhow!("Collection `x` doesn't exist").context("count failed");
        assert_eq!(
            error_text::<()>(Err(failed)).unwrap_err(),
            "count failed: Collection `x` doesn't exist"
        );
        assert_eq!(error_text(Ok(7)), Ok(7));
    }
}
