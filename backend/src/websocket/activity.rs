use super::user_state;
use crate::{AppState, agent_service::analysis::AnalysisRequestParams};
use dialect_coach_shared::{
    ActivityDefinition, AgentAnalysis, AgentUsageStats, EvaluateActivityRequest,
    GenerateActivityRequest, TeachingMode, UsageStats,
};

pub(super) fn check_analysis_quota(state: &AppState, usage: &UsageStats) -> anyhow::Result<()> {
    use crate::rate_limiter::service::RateLimiterService;
    anyhow::ensure!(
        state
            .rate_limiter
            .can_make_analysis_call(usage, &state.rate_limit_config),
        "Analysis agent rate limit exceeded"
    );
    Ok(())
}

pub async fn generate(
    state: &AppState,
    request: GenerateActivityRequest,
) -> Result<ActivityDefinition, String> {
    let usage =
        user_state::check_rate_limits(state, request.user_id, TeachingMode::Immersive, false)
            .await
            .map_err(|e| e.to_string())?;
    let (result, response_usage) =
        crate::agent_service::activity::generate(state.agent.planning_agent.as_ref(), &request)
            .await;
    user_state::update_and_save_usage(
        state,
        request.user_id,
        usage,
        &AgentUsageStats {
            response_usage,
            learning_usage: vec![],
            analysis_usage: vec![],
        },
        chrono::Utc::now().timestamp(),
    )
    .await;
    result.map_err(|e| e.to_string())
}

pub async fn evaluate(
    state: &AppState,
    request: EvaluateActivityRequest,
) -> Result<AgentAnalysis, String> {
    let usage = state
        .user_persistence
        .load_usage_stats(request.user_id)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let empty = String::new();
    let context = &request.context;
    let (result, analysis_usage) = state
        .agent
        .generate_analysis(AnalysisRequestParams {
            allowed: check_analysis_quota(state, &usage).is_ok(),
            activity: Some(context),
            dialect: context.dialect,
            msg: &empty,
            mistakes: &[],
            explained: &[],
            translated: &[],
            exploratory: &[],
            language_option: &context.language_option,
        })
        .await;
    user_state::update_and_save_usage(
        state,
        request.user_id,
        usage,
        &AgentUsageStats {
            response_usage: vec![],
            learning_usage: vec![],
            analysis_usage,
        },
        chrono::Utc::now().timestamp(),
    )
    .await;
    result.map_err(|e| e.to_string())
}
