use dialect_coach_shared::models::UsageStats;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct UsageFooterProps {
    pub usage_stats: UsageStats,
    pub is_collapsed: bool,
    pub on_toggle: Callback<()>,
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
        <button class="usage-footer__toggle" accesskey="u" onclick={Callback::from(move |_| on_toggle.emit(()))}>
            <span class="usage-footer__icon">{if is_collapsed { "▲" } else { "▼" }}</span>
            <span class="usage-footer__title">{"Usage Statistics"}</span>
        </button>
    }
}

fn render_content(stats: &UsageStats) -> Html {
    let response_calls = stats.response_count();
    let response_tokens = stats.response_input_tokens() + stats.response_output_tokens();
    let analysis_calls = stats.analysis_count();
    let analysis_tokens = stats.analysis_input_tokens() + stats.analysis_output_tokens();
    let learning_calls = stats.learning_count();
    let learning_tokens = stats.learning_input_tokens() + stats.learning_output_tokens();
    let tts_calls = stats.tts_count();
    let tts_chars = stats.tts_characters();

    html! {
        <div class="usage-footer__content">
            {render_section("Response Agent", response_calls, "Tokens:", response_tokens)}
            {render_section("Analysis Agent", analysis_calls, "Tokens:", analysis_tokens)}
            {render_section("Learning Agent", learning_calls, "Tokens:", learning_tokens)}
            {render_section("Text-to-Speech", tts_calls, "Characters:", tts_chars)}
        </div>
    }
}

#[function_component(UsageFooter)]
pub fn usage_footer(props: &UsageFooterProps) -> Html {
    let footer_class = if props.is_collapsed {
        "usage-footer usage-footer--collapsed"
    } else {
        "usage-footer"
    };

    html! {
        <div class={footer_class}>
            {render_toggle_button(props.is_collapsed, props.on_toggle.clone())}
            {if !props.is_collapsed { render_content(&props.usage_stats) } else { html! {} }}
        </div>
    }
}
