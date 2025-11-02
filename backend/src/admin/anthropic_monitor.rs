use crate::admin::types::AnthropicStats;
use anyhow::Result;
use serde_json::Value;

pub async fn get_anthropic_stats(admin_api_key: &str) -> Result<AnthropicStats> {
    let (starting_at, ending_at) = get_date_range();
    let usage_data = fetch_usage_data(admin_api_key, &starting_at, &ending_at).await?;
    let cost_data = fetch_cost_data(admin_api_key, &starting_at, &ending_at).await?;
    Ok(parse_anthropic_response(usage_data, cost_data))
}

fn get_date_range() -> (String, String) {
    let ending_at = chrono::Utc::now();
    let starting_at = ending_at - chrono::Duration::days(30);
    (starting_at.to_rfc3339(), ending_at.to_rfc3339())
}

async fn fetch_usage_data(api_key: &str, start: &str, end: &str) -> Result<Value> {
    let url = format!(
        "https://api.anthropic.com/v1/organizations/usage_report/messages?starting_at={}&ending_at={}&bucket_width=1d",
        start, end
    );
    fetch_anthropic_api(&url, api_key).await
}

async fn fetch_cost_data(api_key: &str, start: &str, end: &str) -> Result<Value> {
    let url = format!(
        "https://api.anthropic.com/v1/organizations/cost_report?starting_at={}&ending_at={}&bucket_width=1d",
        start, end
    );
    fetch_anthropic_api(&url, api_key).await
}

async fn fetch_anthropic_api(url: &str, api_key: &str) -> Result<Value> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?
        .json::<Value>()
        .await?;
    Ok(response)
}

fn parse_anthropic_response(usage_data: Value, cost_data: Value) -> AnthropicStats {
    let (uncached, cached, cache_creation, output) = sum_token_usage(&usage_data);
    let total_cost = sum_costs(&cost_data);

    AnthropicStats {
        uncached_input_tokens: Some(uncached),
        cached_input_tokens: Some(cached),
        cache_creation_tokens: Some(cache_creation),
        output_tokens: Some(output),
        total_cost_usd: Some(format!("{:.2}", total_cost)),
        error: None,
    }
}

fn sum_token_usage(usage_data: &Value) -> (u64, u64, u64, u64) {
    let buckets = usage_data.get("data").and_then(|d| d.as_array());
    let mut uncached = 0u64;
    let mut cached = 0u64;
    let mut cache_creation = 0u64;
    let mut output = 0u64;

    if let Some(buckets) = buckets {
        for bucket in buckets {
            uncached += bucket
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            cached += bucket
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            cache_creation += bucket
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            output += bucket
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
        }
    }
    (uncached, cached, cache_creation, output)
}

fn sum_costs(cost_data: &Value) -> f64 {
    let buckets = cost_data.get("data").and_then(|d| d.as_array());
    let mut total = 0.0;

    if let Some(buckets) = buckets {
        for bucket in buckets {
            if let Some(cost_str) = bucket.get("amount").and_then(|v| v.as_str())
                && let Ok(cost) = cost_str.parse::<f64>()
            {
                total += cost;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_date_range() {
        let (start, end) = get_date_range();
        assert!(start.len() > 0);
        assert!(end.len() > 0);
        assert!(start < end);
    }

    #[test]
    fn test_sum_token_usage() {
        let usage_data = json!({
            "data": [
                {"input_tokens": 100, "cache_read_input_tokens": 50, "cache_creation_input_tokens": 25, "output_tokens": 75},
                {"input_tokens": 200, "cache_read_input_tokens": 100, "cache_creation_input_tokens": 50, "output_tokens": 150}
            ]
        });
        let (uncached, cached, cache_creation, output) = sum_token_usage(&usage_data);
        assert_eq!(uncached, 300);
        assert_eq!(cached, 150);
        assert_eq!(cache_creation, 75);
        assert_eq!(output, 225);
    }

    #[test]
    fn test_sum_costs() {
        let cost_data = json!({
            "data": [
                {"amount": "1.50"},
                {"amount": "2.75"}
            ]
        });
        let total = sum_costs(&cost_data);
        assert_eq!(total, 4.25);
    }

    #[test]
    fn test_parse_anthropic_response() {
        let usage_data = json!({"data": [{"input_tokens": 100, "cache_read_input_tokens": 50, "cache_creation_input_tokens": 25, "output_tokens": 75}]});
        let cost_data = json!({"data": [{"amount": "1.50"}]});
        let stats = parse_anthropic_response(usage_data, cost_data);
        assert_eq!(stats.uncached_input_tokens, Some(100));
        assert_eq!(stats.cached_input_tokens, Some(50));
        assert_eq!(stats.cache_creation_tokens, Some(25));
        assert_eq!(stats.output_tokens, Some(75));
        assert_eq!(stats.total_cost_usd, Some("1.50".to_string()));
        assert_eq!(stats.error, None);
    }
}
