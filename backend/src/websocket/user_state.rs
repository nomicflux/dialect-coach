use dialect_coach_shared::{AgentUsageStats, UserState};
use uuid::Uuid;

use super::registry;
use crate::AppState;
use crate::rate_limiter::service::RateLimiterService;

pub async fn save(state: &AppState, mut user_state: UserState) -> Result<(), String> {
    for plan in user_state.language_plans.iter() {
        tracing::info!(
            "Backend received plan to save: {} (id: {})",
            plan.title,
            plan.id
        );
    }
    prepare_user_state_for_save(&mut user_state);
    tracing::info!("Saving user state for user: {}", user_state.user_id);
    state.user_persistence.save(&user_state).await.map_err(|e| {
        tracing::error!("Failed to save user state: {}", e);
        e.to_string()
    })
}

pub async fn update_and_save_usage(
    state: &AppState,
    user_id: Uuid,
    mut usage_stats: dialect_coach_shared::UsageStats,
    agent_usage: &AgentUsageStats,
    now: i64,
) {
    log_response_usage(&user_id, &agent_usage.response_usage);
    crate::usage_tracker::add_response_usage(
        &mut usage_stats,
        agent_usage.response_usage.clone(),
        now,
        24,
    );

    if !agent_usage.learning_usage.is_empty() {
        log_learning_usage(&user_id, &agent_usage.learning_usage);
        crate::usage_tracker::add_learning_usage(
            &mut usage_stats,
            agent_usage.learning_usage.clone(),
            now,
            24,
        );
    }

    if !agent_usage.analysis_usage.is_empty() {
        log_analysis_usage(&user_id, &agent_usage.analysis_usage);
        crate::usage_tracker::add_analysis_usage(
            &mut usage_stats,
            agent_usage.analysis_usage.clone(),
            now,
            24,
        );
    }

    log_total_usage(&user_id, &usage_stats);
    save_usage_and_notify(state, usage_stats, user_id).await;
}

fn log_response_usage(
    user_id: &Uuid,
    usage: &[dialect_coach_shared::models::usage_stats::AgentUsage],
) {
    let input_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::input_tokens_total(usage);
    let output_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::output_tokens_total(usage);
    let retry_count = dialect_coach_shared::models::usage_stats::AgentUsage::retry_count(usage);
    let estimate_count =
        dialect_coach_shared::models::usage_stats::AgentUsage::estimate_count(usage);

    tracing::info!(
        user_id = %user_id,
        "Updating usage stats: {} response events ({} input, {} output tokens, {} retries, {} estimates)",
        usage.len(),
        input_tokens,
        output_tokens,
        retry_count,
        estimate_count
    );
}

fn log_analysis_usage(
    user_id: &Uuid,
    usage: &[dialect_coach_shared::models::usage_stats::AgentUsage],
) {
    let input_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::input_tokens_total(usage);
    let output_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::output_tokens_total(usage);
    let retry_count = dialect_coach_shared::models::usage_stats::AgentUsage::retry_count(usage);
    let estimate_count =
        dialect_coach_shared::models::usage_stats::AgentUsage::estimate_count(usage);

    tracing::info!(
        user_id = %user_id,
        "Updating usage stats: {} analysis events ({} input, {} output tokens, {} retries, {} estimates)",
        usage.len(),
        input_tokens,
        output_tokens,
        retry_count,
        estimate_count
    );
}

fn log_total_usage(user_id: &Uuid, stats: &dialect_coach_shared::UsageStats) {
    tracing::info!(
        user_id = %user_id,
        "Usage stats updated: {} total response events, {} total analysis events, {} total learning events",
        stats.response_count(),
        stats.analysis_count(),
        stats.learning_count()
    );
}

fn log_learning_usage(
    user_id: &Uuid,
    usage: &[dialect_coach_shared::models::usage_stats::AgentUsage],
) {
    let input_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::input_tokens_total(usage);
    let output_tokens =
        dialect_coach_shared::models::usage_stats::AgentUsage::output_tokens_total(usage);
    let retry_count = dialect_coach_shared::models::usage_stats::AgentUsage::retry_count(usage);
    let estimate_count =
        dialect_coach_shared::models::usage_stats::AgentUsage::estimate_count(usage);

    tracing::info!(
        user_id = %user_id,
        "Updating usage stats: {} learning events ({} input, {} output tokens, {} retries, {} estimates)",
        usage.len(),
        input_tokens,
        output_tokens,
        retry_count,
        estimate_count
    );
}

pub fn log_branch_metadata(event: &str, state: &dialect_coach_shared::UserState) {
    let branch_count = state.branches.len();
    let active_branch_id = state.active_branch_id;
    let has_active_branch = state.branches.iter().any(|b| b.id == active_branch_id);
    let missing_leaf_count = state
        .branches
        .iter()
        .filter(|branch| branch.leaf_message_id.is_none())
        .count();

    tracing::info!(user_id = %state.user_id, event, branch_count, conversation_len = state.conversation_history.len(), has_active_branch, missing_leaf_count, "Branch metadata snapshot");
}

async fn save_usage_and_notify(
    state: &AppState,
    usage_stats: dialect_coach_shared::UsageStats,
    user_id: Uuid,
) {
    match state
        .user_persistence
        .save_usage_stats(user_id, &usage_stats)
        .await
    {
        Ok(_) => {
            tracing::info!(user_id = %user_id, "Successfully saved usage stats");
            registry::push_usage(&state.connections, user_id, usage_stats).await;
        }
        Err(e) => tracing::error!(user_id = %user_id, "Failed to save usage stats: {}", e),
    }
}

fn should_check_analysis_limit(
    teaching_mode: dialect_coach_shared::TeachingMode,
    needs_analysis: bool,
) -> bool {
    if !needs_analysis {
        return false;
    }
    teaching_mode != dialect_coach_shared::TeachingMode::Immersive
        && teaching_mode != dialect_coach_shared::TeachingMode::Debug
        && teaching_mode != dialect_coach_shared::TeachingMode::ErrorFinding
}

pub async fn check_rate_limits(
    state: &AppState,
    user_id: Uuid,
    teaching_mode: dialect_coach_shared::TeachingMode,
    needs_analysis: bool,
) -> Result<dialect_coach_shared::UsageStats, anyhow::Error> {
    let usage_stats = state
        .user_persistence
        .load_usage_stats(user_id)
        .await?
        // No usage row yet means the account has used nothing.
        .unwrap_or_default();

    if !state.rate_limiter.anthropic_has_quota().await {
        return Err(anyhow::anyhow!("Anthropic quota exceeded"));
    }

    if !state
        .rate_limiter
        .can_make_response_call(&usage_stats, &state.rate_limit_config)
    {
        return Err(anyhow::anyhow!("Response agent rate limit exceeded"));
    }

    if should_check_analysis_limit(teaching_mode, needs_analysis)
        && !state
            .rate_limiter
            .can_make_analysis_call(&usage_stats, &state.rate_limit_config)
    {
        return Err(anyhow::anyhow!("Analysis agent rate limit exceeded"));
    }

    Ok(usage_stats)
}

pub fn prepare_user_state_for_save(user_state: &mut UserState) {
    // Removed all migrations - no version gating meant they ran forever
    // - migrate_branch_message_ids: Old tree format → message_ids list
    // - rebuild_branches_from_history: Prevented legitimate deletion of all branches
    log_branch_metadata("save_user_state", user_state);
}
