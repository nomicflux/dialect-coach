use dialect_coach_shared::models::UsageStats;
use dialect_coach_shared::models::usage_stats::AgentUsage;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct UsagePanelProps {
    pub usage_stats: UsageStats,
}

/// One section of the panel: a service, its call count, and its metric total.
#[derive(Debug, PartialEq)]
pub struct UsageRow {
    pub title: &'static str,
    pub calls: usize,
    pub metric_label: &'static str,
    pub metric_value: u64,
}

fn agent_row(title: &'static str, events: &[AgentUsage]) -> UsageRow {
    UsageRow {
        title,
        calls: events.len(),
        metric_label: "Tokens:",
        metric_value: AgentUsage::input_tokens_total(events)
            + AgentUsage::output_tokens_total(events),
    }
}

pub fn usage_rows(stats: &UsageStats) -> [UsageRow; 4] {
    [
        agent_row("Response Agent", &stats.response_events),
        agent_row("Analysis Agent", &stats.analysis_events),
        agent_row("Learning Agent", &stats.learning_events),
        UsageRow {
            title: "Text-to-Speech",
            calls: stats.tts_count(),
            metric_label: "Characters:",
            metric_value: stats.tts_characters(),
        },
    ]
}

fn render_stat(label: &str, value: String) -> Html {
    html! {
        <div class="usage-panel__stat">
            <span class="usage-panel__label">{label}</span>
            <span class="usage-panel__value">{value}</span>
        </div>
    }
}

fn render_section(row: &UsageRow) -> Html {
    html! {
        <div class="panel-section">
            <h4 class="panel-section-title">{row.title}</h4>
            {render_stat("Calls:", row.calls.to_string())}
            {render_stat(row.metric_label, row.metric_value.to_string())}
        </div>
    }
}

#[function_component(UsagePanel)]
pub fn usage_panel(props: &UsagePanelProps) -> Html {
    html! {
        <div class="usage-panel">
            {for usage_rows(&props.usage_stats).iter().map(render_section)}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::models::usage_stats::TtsUsage;

    fn agent_usage(input_tokens: u64, output_tokens: u64) -> AgentUsage {
        AgentUsage {
            timestamp: 0,
            input_tokens,
            output_tokens,
            is_retry: false,
            is_estimate: false,
            provider: "anthropic".to_string(),
            model: "claude-test".to_string(),
        }
    }

    #[test]
    fn test_agent_row_counts_events_and_sums_input_and_output_tokens() {
        let row = agent_row("Response Agent", &[agent_usage(7, 3), agent_usage(1, 1)]);
        assert_eq!(
            row,
            UsageRow {
                title: "Response Agent",
                calls: 2,
                metric_label: "Tokens:",
                metric_value: 12,
            }
        );
    }

    #[test]
    fn test_usage_rows_count_calls_and_sum_metrics() {
        let stats = UsageStats {
            response_events: vec![agent_usage(100, 50), agent_usage(10, 5)],
            analysis_events: vec![agent_usage(200, 100)],
            learning_events: vec![],
            tts_events: vec![TtsUsage {
                timestamp: 0,
                characters: 250,
            }],
        };

        let rows = usage_rows(&stats);

        let summary: Vec<_> = rows
            .iter()
            .map(|r| (r.title, r.calls, r.metric_label, r.metric_value))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("Response Agent", 2, "Tokens:", 165),
                ("Analysis Agent", 1, "Tokens:", 300),
                ("Learning Agent", 0, "Tokens:", 0),
                ("Text-to-Speech", 1, "Characters:", 250),
            ]
        );
    }
}
