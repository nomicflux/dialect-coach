use crate::admin::types::ElevenLabsStats;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;

pub async fn get_elevenlabs_usage(api_key: &str) -> Result<ElevenLabsStats> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.elevenlabs.io/v1/user")
        .header("xi-api-key", api_key)
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(parse_elevenlabs_response(response))
}

fn parse_elevenlabs_response(json: Value) -> ElevenLabsStats {
    let subscription = &json["subscription"];
    let tier = subscription["tier"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let characters_used = subscription["character_count"].as_u64().unwrap_or(0);
    let characters_limit = subscription["character_limit"].as_u64().unwrap_or(0);
    let reset_unix = subscription["next_character_count_reset_unix"]
        .as_i64()
        .unwrap_or(0);

    ElevenLabsStats {
        tier,
        characters_used,
        characters_limit,
        characters_remaining: characters_limit.saturating_sub(characters_used),
        reset_date: unix_to_iso8601(reset_unix),
    }
}

fn unix_to_iso8601(unix_timestamp: i64) -> String {
    DateTime::from_timestamp(unix_timestamp, 0)
        .unwrap_or_else(Utc::now)
        .to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_elevenlabs_response() {
        let mock_response = json!({
            "subscription": {
                "tier": "creator",
                "character_count": 5000,
                "character_limit": 100000,
                "next_character_count_reset_unix": 1730419200
            }
        });

        let stats = parse_elevenlabs_response(mock_response);
        assert_eq!(stats.tier, "creator");
        assert_eq!(stats.characters_used, 5000);
        assert_eq!(stats.characters_limit, 100000);
        assert_eq!(stats.characters_remaining, 95000);
    }

    #[test]
    fn test_unix_to_iso8601() {
        let iso_date = unix_to_iso8601(1730419200);
        assert!(iso_date.starts_with("2024-11-01"));
    }
}
