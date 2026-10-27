use dialect_coach_shared::models::{ActivityDefinition, Message};
use gloo::{
    events::{EventListener, EventListenerOptions},
    timers::callback::Timeout,
};
use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum SelectionAction {
    Translate,
    ExplainGrammar,
}

#[derive(Clone, Debug, PartialEq)]
struct TextSelection {
    source_id: Uuid,
    text: String,
    position: (f64, f64),
}

type SelectionState = UseStateHandle<Option<TextSelection>>;
type SelectionRequest = (Uuid, SelectionAction, String, String);

#[derive(Properties, PartialEq)]
pub struct Props {
    pub messages: Arc<Vec<Message>>,
    #[prop_or_default]
    pub activity: Option<ActivityDefinition>,
    pub translate_loading: HashSet<Uuid>,
    pub on_action: Callback<SelectionRequest>,
}

#[hook]
fn use_text_selection(buttons: NodeRef) -> SelectionState {
    let selection = use_state(|| None);
    let pending = use_mut_ref(|| None::<Timeout>);
    use_effect_with((), {
        let selection = selection.clone();
        move |_| {
            let document = gloo::utils::document();
            let listener = selection_listener(selection, pending.clone());
            let scroll = {
                let buttons = buttons.clone();
                EventListener::new_with_options(
                    &document,
                    "scroll",
                    EventListenerOptions::run_in_capture_phase(),
                    move |_| position_buttons(&buttons),
                )
            };
            let resize = EventListener::new(&gloo::utils::window(), "resize", move |_| {
                position_buttons(&buttons);
            });
            move || {
                drop((listener, scroll, resize));
                pending.borrow_mut().take();
            }
        }
    });
    selection
}

fn selection_listener(
    selection: SelectionState,
    pending: Rc<RefCell<Option<Timeout>>>,
) -> EventListener {
    EventListener::new(&gloo::utils::document(), "selectionchange", move |_| {
        let selection = selection.clone();
        *pending.borrow_mut() = Some(Timeout::new(50, move || {
            selection.set(read_selection());
        }));
    })
}

fn read_selection() -> Option<TextSelection> {
    let selection = gloo::utils::document().get_selection().ok()??;
    if selection.is_collapsed() {
        return None;
    }
    let range = selection.get_range_at(0).ok()?;
    let anchor = source_at(&range.start_container().ok()?)?;
    let focus = source_at(&range.end_container().ok()?)?;
    if anchor != focus {
        return None;
    }
    let id = anchor
        .get_attribute("data-message-id")
        .or_else(|| anchor.get_attribute("data-activity-id"))?;
    let (source_id, text) = selection_target(selection.to_string().as_string()?, &id, &id)?;
    let rect = range.get_bounding_client_rect();
    Some(TextSelection {
        source_id,
        text,
        position: (rect.right(), rect.bottom()),
    })
}

fn position_buttons(buttons: &NodeRef) {
    let Some(buttons) = buttons.cast::<web_sys::Element>() else {
        return;
    };
    let Some(selection) = gloo::utils::document().get_selection().ok().flatten() else {
        return;
    };
    let Ok(range) = selection.get_range_at(0) else {
        return;
    };
    let window = gloo::utils::window();
    let (Some(width), Some(height)) = (
        window.inner_width().ok().and_then(|v| v.as_f64()),
        window.inner_height().ok().and_then(|v| v.as_f64()),
    ) else {
        return;
    };
    let rect = range.get_bounding_client_rect();
    let size = buttons.get_bounding_client_rect();
    let left = rect.right().clamp(0.0, (width - size.width()).max(0.0));
    let top = rect.bottom().clamp(0.0, (height - size.height()).max(0.0));
    let _ = buttons.set_attribute("style", &format!("left: {left}px; top: {top}px;"));
}

fn source_at(node: &web_sys::Node) -> Option<web_sys::Element> {
    node.dyn_ref::<web_sys::Element>()
        .cloned()
        .or_else(|| node.parent_element())?
        .closest(".message-content[data-message-id], [data-activity-id]")
        .ok()?
}

fn selection_target(text: String, anchor_id: &str, focus_id: &str) -> Option<(Uuid, String)> {
    if text.trim().is_empty() || anchor_id != focus_id {
        return None;
    }
    Some((Uuid::parse_str(anchor_id).ok()?, text))
}

fn activity_context(activity: &ActivityDefinition) -> String {
    let goals = activity
        .goals
        .iter()
        .map(|goal| goal.goal.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}\n{}\n{}", activity.title, activity.learner_brief, goals)
}

fn selection_context(props: &Props, selected: &TextSelection) -> Option<String> {
    if let Some(activity) = props
        .activity
        .as_ref()
        .filter(|a| a.activity_id == selected.source_id)
    {
        return Some(activity_context(activity));
    }
    message_context(&props.messages, selected.source_id)
}

fn message_context(messages: &[Message], id: Uuid) -> Option<String> {
    messages
        .iter()
        .find(|message| message.id == id)
        .map(Message::get_content)
}

fn action_callback(
    state: SelectionState,
    on_action: Callback<SelectionRequest>,
    request: SelectionRequest,
) -> Callback<MouseEvent> {
    Callback::from(move |event: MouseEvent| {
        event.stop_propagation();
        state.set(None);
        on_action.emit(request.clone());
    })
}

fn render_action_button(
    props: &Props,
    selected: &TextSelection,
    state: SelectionState,
    action: SelectionAction,
    context: String,
    label: &str,
) -> Html {
    let request = (selected.source_id, action, selected.text.clone(), context);
    let onclick = action_callback(state, props.on_action.clone(), request);
    let disabled = props.translate_loading.contains(&selected.source_id);
    let onpointerdown = Callback::from(|event: PointerEvent| event.prevent_default());
    html! { <button type="button" class="translate-selection-btn" {onpointerdown} {onclick} {disabled}>{label}</button> }
}

#[function_component(SelectionActionButtons)]
pub fn selection_action_buttons(props: &Props) -> Html {
    let buttons = use_node_ref();
    let state = use_text_selection(buttons.clone());
    {
        let buttons = buttons.clone();
        use_effect_with((*state).clone(), move |_| position_buttons(&buttons));
    }
    let Some(selected) = state.as_ref() else {
        return html! {};
    };
    let Some(context) = selection_context(props, selected) else {
        return html! {};
    };
    let style = format!(
        "left: {}px; top: {}px;",
        selected.position.0, selected.position.1
    );
    html! {
        <div ref={buttons} class="selection-action-buttons" {style}>
            {render_action_button(props, selected, state.clone(), SelectionAction::Translate, context.clone(), "Translate")}
            {render_action_button(props, selected, state.clone(), SelectionAction::ExplainGrammar, context, "Grammar")}
        </div>
    }
}

pub use SelectionActionButtons as TranslateSelectionButton;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::agent_message;

    #[test]
    fn rejects_blank_cross_message_and_invalid_targets() {
        let id = Uuid::new_v4().to_string();
        assert_eq!(selection_target(" \n ".into(), &id, &id), None);
        assert_eq!(
            selection_target("word".into(), &id, &Uuid::new_v4().to_string()),
            None
        );
        assert_eq!(selection_target("word".into(), "invalid", "invalid"), None);
        assert_eq!(selection_target("word".into(), &id, ""), None);
    }

    #[test]
    fn keeps_selected_text_and_message_identity() {
        let message_id = Uuid::new_v4();
        let id = message_id.to_string();
        assert_eq!(
            selection_target(" أهلا ".into(), &id, &id),
            Some((message_id, " أهلا ".into()))
        );
    }

    #[test]
    fn finds_full_context_by_message_id() {
        let first = agent_message("first message");
        let second = agent_message("full second message");
        let messages = vec![first, second.clone()];
        assert_eq!(
            message_context(&messages, second.id),
            Some("full second message".into())
        );
        assert_eq!(message_context(&messages, Uuid::new_v4()), None);
        assert_eq!(message_context(&[], second.id), None);
    }

    #[test]
    fn activity_lookup_contains_only_visible_text() {
        use dialect_coach_shared::models::{Dialect, LearningGoal};
        let activity = ActivityDefinition {
            activity_id: Uuid::new_v4(),
            title: "Title".into(),
            learner_brief: "Visible brief".into(),
            shared_context: "Not displayed".into(),
            partner_brief: "Hidden answer".into(),
            goals: vec![LearningGoal {
                goal: "Saved goal".into(),
                dialect: Dialect::ArabicEgyptian,
                completed: None,
            }],
        };
        assert_eq!(
            activity_context(&activity),
            "Title\nVisible brief\nSaved goal"
        );
    }
}
