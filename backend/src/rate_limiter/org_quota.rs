use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};

#[derive(Debug, Clone)]
pub struct QuotaStatus {
    pub anthropic_has_quota: bool,
    pub elevenlabs_has_quota: bool,
}

impl Default for QuotaStatus {
    fn default() -> Self {
        Self {
            anthropic_has_quota: true,
            elevenlabs_has_quota: true,
        }
    }
}

#[derive(Clone, Default)]
pub struct OrgQuotaChecker {
    status: Arc<RwLock<QuotaStatus>>,
}

impl OrgQuotaChecker {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(QuotaStatus::default())),
        }
    }

    pub async fn has_anthropic_quota(&self) -> bool {
        self.status.read().await.anthropic_has_quota
    }

    pub async fn has_elevenlabs_quota(&self) -> bool {
        self.status.read().await.elevenlabs_has_quota
    }

    pub fn spawn_background_task(
        self: Arc<Self>,
        anthropic_key: Option<String>,
        elevenlabs_key: Option<String>,
    ) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(300));
            loop {
                ticker.tick().await;
                self.check_quotas(anthropic_key.as_deref(), elevenlabs_key.as_deref())
                    .await;
            }
        });
    }

    async fn check_quotas(&self, anthropic_key: Option<&str>, elevenlabs_key: Option<&str>) {
        let anthropic_ok = check_anthropic(anthropic_key).await;
        let elevenlabs_ok = check_elevenlabs(elevenlabs_key).await;

        let mut status = self.status.write().await;
        status.anthropic_has_quota = anthropic_ok;
        status.elevenlabs_has_quota = elevenlabs_ok;
    }
}

async fn check_anthropic(api_key: Option<&str>) -> bool {
    let Some(key) = api_key else {
        return true;
    };

    match crate::admin::anthropic_monitor::get_anthropic_stats(key).await {
        Ok(_stats) => true,
        Err(e) => {
            tracing::warn!("Failed to check Anthropic quota: {}", e);
            true
        }
    }
}

async fn check_elevenlabs(api_key: Option<&str>) -> bool {
    let Some(key) = api_key else {
        return true;
    };

    match crate::admin::elevenlabs_monitor::get_elevenlabs_usage(key).await {
        Ok(stats) => stats.characters_remaining > 0,
        Err(e) => {
            tracing::warn!("Failed to check ElevenLabs quota: {}", e);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_quota_status() {
        let status = QuotaStatus::default();
        assert!(status.anthropic_has_quota);
        assert!(status.elevenlabs_has_quota);
    }

    #[tokio::test]
    async fn test_org_quota_checker_defaults_true() {
        let checker = OrgQuotaChecker::new();
        assert!(checker.has_anthropic_quota().await);
        assert!(checker.has_elevenlabs_quota().await);
    }

    #[tokio::test]
    async fn test_check_anthropic_no_key() {
        let result = check_anthropic(None).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_check_elevenlabs_no_key() {
        let result = check_elevenlabs(None).await;
        assert!(result);
    }
}
