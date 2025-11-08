use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminStatusResponse {
    pub timestamp: String,
    pub anthropic: AnthropicStats,
    pub elevenlabs: Option<ElevenLabsStats>,
    pub qdrant: Option<QdrantStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicStats {
    pub uncached_input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_cost_usd: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevenLabsStats {
    pub tier: String,
    pub characters_used: u64,
    pub characters_limit: u64,
    pub characters_remaining: u64,
    pub reset_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantStats {
    pub points_count: u64,
    pub vectors_count: u64,
    pub segments_count: u32,
    pub dialect_counts: Vec<DialectCount>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialectCount {
    pub dialect: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInviteRequest {
    pub expires_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteResponse {
    pub code: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteListItem {
    pub code: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub used_at: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteListResponse {
    pub invites: Vec<InviteListItem>,
}
