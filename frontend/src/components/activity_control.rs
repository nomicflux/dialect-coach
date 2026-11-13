use crate::app::app_state::{ActivityOperation, ActivityUi};
use crate::components::learning_goals_panel::render_goals_list;
use crate::components::{message_bubble::font_class, ruby_text::render_text_with_ruby};
use dialect_coach_shared::{ActivitySnapshot, LanguageOption};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ActivityControlProps {
    pub activity: Option<ActivitySnapshot>,
    pub language_option: Option<LanguageOption>,
    pub state: ActivityUi,
    pub on_start: Callback<()>,
    pub on_expand: Callback<bool>,
    pub on_edit: Callback<()>,
    pub on_end: Callback<()>,
    pub on_retry_open: Callback<()>,
    pub on_retry_check: Callback<()>,
}

#[function_component(ActivityControl)]
pub fn activity_control(props: &ActivityControlProps) -> Html {
    let state = &props.state;
    let Some(activity) = props.activity.as_ref().filter(|s| s.is_active()) else {
        return html! {
            <div class="activity-launch">
                <button id="activity-start" type="button" class="btn btn--secondary" onclick={props.on_start.reform(|_| ())} aria-haspopup="dialog">{"Start activity"}</button>
                if state.busy == Some(ActivityOperation::Generate) { <span role="status">{"Creating activity…"}</span> }
                if state.error == Some(ActivityOperation::Generate) { <span role="status">{"Couldn't create activity"}</span> }
            </div>
        };
    };
    let definition = &activity.definition;
    let ActivityProgress {
        completed,
        total,
        error,
        checking,
    } = activity_progress(activity, state);
    let status = if !error.is_empty() {
        error.to_string()
    } else if checking {
        "Checking…".into()
    } else if total == 0 {
        "No goals".into()
    } else {
        format!("{completed}/{total}")
    };
    let on_expand = props.on_expand.clone();
    let expanded = state.expanded;
    let toggle = Callback::from(move |_| on_expand.emit(!expanded));
    let end = props.on_end.reform(|_| ());
    html! {
        <section class={classes!("activity-control", font_class(&props.language_option))} aria-label="Activity">
            <div class="activity-row">
                <button type="button" class={classes!("activity-title", expanded.then_some("expanded"))} onclick={toggle} aria-expanded={expanded.to_string()} aria-controls="activity-body">
                    <span aria-hidden="true">{if expanded { "▾ " } else { "▸ " }}</span>{render_text_with_ruby(&definition.title)}
                </button>
                <span class="activity-count" role="status" aria-label={if error.is_empty() && !checking && total > 0 { format!("{completed} of {total} activity goals achieved") } else { status.clone() }}>
                    {status}{if error.is_empty() && !checking && total > 0 { html! { <span class="activity-count-word">{" goals"}</span> } } else { html! {} }}
                </span>
                <button type="button" aria-label="End activity" onclick={end}>{"End"}</button>
            </div>
            if expanded {
                <div id="activity-body" class="activity-body">
                    <p dir="auto" data-activity-id={definition.activity_id.to_string()}>{render_text_with_ruby(&definition.learner_brief)}</p>
                    {render_activity_feedback(activity, state, props.on_retry_open.clone(), props.on_retry_check.clone())}
                    {render_goals_list(&definition.goals, None, None, true, Some(definition.activity_id))}
                    <div class="form-buttons">
                        <button id="activity-edit" type="button" class="btn btn--secondary" onclick={props.on_edit.reform(|_| ())}>{"Edit goals"}</button>
                    </div>
                </div>
            }
        </section>
    }
}

#[derive(Properties, PartialEq)]
pub struct ActivityCreateFormProps {
    pub state: ActivityUi,
    pub on_generate: Callback<String>,
    pub on_description: Callback<String>,
}

#[function_component(ActivityCreateForm)]
pub fn activity_create_form(props: &ActivityCreateFormProps) -> Html {
    let busy = props.state.busy.is_some();
    let generate = props.on_generate.clone();
    let description = props.state.description.clone();
    let onsubmit = Callback::from(move |event: SubmitEvent| {
        event.prevent_default();
        if !busy {
            generate.emit(description.clone());
        }
    });
    let change = props.on_description.clone();
    let oninput = Callback::from(move |event: InputEvent| {
        if let Some(input) = event.target_dyn_into::<web_sys::HtmlTextAreaElement>() {
            change.emit(input.value());
        }
    });
    html! {
        <form class="plan-generator" {onsubmit}>
            <div class="panel-field">
                <label for="activity-description">{"Description (optional)"}</label>
                <textarea id="activity-description" class="generator-textarea" rows="3" value={props.state.description.clone()} {oninput} disabled={busy} aria-describedby="activity-description-help" />
                <div id="activity-description-help" class="field-help">{"Leave blank to generate a random activity based on this conversation and your settings."}</div>
            </div>
            if props.state.error == Some(ActivityOperation::Generate) {
                <div class="error-message" role="alert">{"Couldn't create activity. Try again."}</div>
            }
            <div class="form-buttons">
                <button type="submit" class="btn btn--primary" disabled={busy}>{if busy { "Creating activity…" } else { "Start activity" }}</button>
            </div>
        </form>
    }
}

struct ActivityProgress {
    completed: usize,
    total: usize,
    error: &'static str,
    checking: bool,
}

fn activity_progress(activity: &ActivitySnapshot, state: &ActivityUi) -> ActivityProgress {
    let total = activity.definition.goals.len();
    ActivityProgress {
        completed: activity
            .definition
            .goals
            .iter()
            .filter(|g| g.completed == Some(true))
            .count(),
        total,
        error: match state.error {
            Some(ActivityOperation::Open) => "Opening failed",
            Some(ActivityOperation::Check) if total > 0 => "Check failed",
            _ => "",
        },
        checking: total > 0
            && matches!(
                state.busy,
                Some(ActivityOperation::Check | ActivityOperation::Chat)
            ),
    }
}

pub(super) fn render_activity_feedback(
    activity: &ActivitySnapshot,
    state: &ActivityUi,
    retry_open: Callback<()>,
    retry_check: Callback<()>,
) -> Html {
    let progress = activity_progress(activity, state);
    html! { <div class="activity-feedback">
        if !progress.error.is_empty() { <p>{progress.error}</p> }
        if progress.checking { <p>{"Checking goals…"}</p> }
        if progress.checking || progress.error == "Check failed" { <p class="activity-stale">{"Previous results await a fresh check."}</p> }
        if progress.total == 0 { <p>{"No activity goals"}</p> }
        if progress.total > 0 && progress.completed == progress.total && !progress.checking && progress.error.is_empty() { <p>{"All goals achieved"}</p> }
        if progress.error == "Opening failed" { <button type="button" class="btn btn--secondary" onclick={retry_open.reform(|_| ())} disabled={state.busy.is_some()}>{"Retry opening"}</button> }
        if progress.error == "Check failed" { <button type="button" class="btn btn--secondary" onclick={retry_check.reform(|_| ())} disabled={state.busy.is_some()}>{"Retry check"}</button> }
    </div> }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_current_goals_hide_obsolete_check_failure() {
        let activity = ActivitySnapshot::new(dialect_coach_shared::ActivityDefinition {
            activity_id: uuid::Uuid::new_v4(),
            title: "Task".into(),
            shared_context: "Context".into(),
            learner_brief: "Brief".into(),
            partner_brief: "Hidden".into(),
            goals: vec![],
        });
        let state = ActivityUi {
            error: Some(ActivityOperation::Check),
            busy: Some(ActivityOperation::Chat),
            ..ActivityUi::default()
        };
        let progress = activity_progress(&activity, &state);
        assert_eq!(progress.total, 0);
        assert!(progress.error.is_empty());
        assert!(!progress.checking);
    }
}
