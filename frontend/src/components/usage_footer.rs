use dialect_coach_shared::models::{AgentUsage, TtsUsage, UsageStats};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct UsageFooterProps {
    pub usage_stats: UsageStats,
    pub is_collapsed: bool,
    pub on_toggle: Callback<()>,
}

fn count_agent_calls(events: &[AgentUsage]) -> usize {
    events.len()
}

fn sum_agent_tokens(events: &[AgentUsage]) -> u64 {
    events
        .iter()
        .map(|e| e.input_tokens + e.output_tokens)
        .sum()
}

fn count_tts_calls(events: &[TtsUsage]) -> usize {
    events.len()
}

fn sum_tts_characters(events: &[TtsUsage]) -> u64 {
    events.iter().map(|e| e.characters).sum()
}

fn render_stat(label: &str, value: String) -> Html {
    html! {
        <div class="usage-footer__stat">
            <span class="usage-footer__label">{label}</span>
            <span class="usage-footer__value">{value}</span>
        </div>
    }
}

fn render_section(title: &str, calls: usize, metric_label: &str, metric_value: u64) -> Html {
    html! {
        <div class="usage-footer__section">
            <h3 class="usage-footer__section-title">{title}</h3>
            <div class="usage-footer__stats">
                {render_stat("Calls:", calls.to_string())}
                {render_stat(metric_label, metric_value.to_string())}
            </div>
        </div>
    }
}

fn render_toggle_button(is_collapsed: bool, on_toggle: Callback<()>) -> Html {
    html! {
        <button class="usage-footer__toggle" onclick={Callback::from(move |_| on_toggle.emit(()))}>
            <span class="usage-footer__icon">{if is_collapsed { "▲" } else { "▼" }}</span>
            <span class="usage-footer__title">{"Usage Statistics"}</span>
        </button>
    }
}

fn render_content(stats: &UsageStats) -> Html {
    let response_calls = count_agent_calls(&stats.response_events);
    let response_tokens = sum_agent_tokens(&stats.response_events);
    let analysis_calls = count_agent_calls(&stats.analysis_events);
    let analysis_tokens = sum_agent_tokens(&stats.analysis_events);
    let tts_calls = count_tts_calls(&stats.tts_events);
    let tts_chars = sum_tts_characters(&stats.tts_events);

    html! {
        <div class="usage-footer__content">
            {render_section("Response Agent", response_calls, "Tokens:", response_tokens)}
            {render_section("Analysis Agent", analysis_calls, "Tokens:", analysis_tokens)}
            {render_section("Text-to-Speech", tts_calls, "Characters:", tts_chars)}
        </div>
    }
}

#[function_component(UsageFooter)]
pub fn usage_footer(props: &UsageFooterProps) -> Html {
    html! {
        <div class="usage-footer">
            {render_toggle_button(props.is_collapsed, props.on_toggle.clone())}
            {if !props.is_collapsed { render_content(&props.usage_stats) } else { html! {} }}
        </div>
    }
}
