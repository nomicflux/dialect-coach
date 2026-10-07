use dialect_coach_shared::models::Message;
use gloo::{events::EventListener, timers::callback::Timeout};
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
struct BarSelection {
    message_id: Uuid,
    text: String,
}

type SelectionState = UseStateHandle<Option<BarSelection>>;
type SelectionRequest = (Uuid, SelectionAction, String, String);

#[derive(Properties, PartialEq)]
pub struct Props {
    pub messages: Arc<Vec<Message>>,
    pub translate_loading: HashSet<Uuid>,
    pub on_action: Callback<SelectionRequest>,
}

#[hook]
fn use_bar_selection() -> SelectionState {
    let selection = use_state(|| None);
    let pending = use_mut_ref(|| None::<Timeout>);
    use_effect_with((), {
        let selection = selection.clone();
        move |_| {
            let listener = selection_listener(selection, pending.clone());
            move || {
                drop(listener);
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
    let document = gloo::utils::document();
    EventListener::new(&document, "selectionchange", move |_| {
        let selection = selection.clone();
        *pending.borrow_mut() = Some(Timeout::new(300, move || selection.set(read_selection())));
    })
}

fn read_selection() -> Option<BarSelection> {
    let selection = gloo::utils::document().get_selection().ok()??;
    if selection.is_collapsed() {
        return None;
    }
    let text = selection.to_string().as_string()?;
    let anchor = message_id_at(&selection.anchor_node()?)?;
    let focus = message_id_at(&selection.focus_node()?)?;
    selection_target(text, &anchor, &focus)
}

fn message_id_at(node: &web_sys::Node) -> Option<String> {
    let element = node
        .dyn_ref::<web_sys::Element>()
        .cloned()
        .or_else(|| node.parent_element())?;
    element
        .closest(".message-content[data-message-id]")
        .ok()??
        .get_attribute("data-message-id")
}

fn selection_target(text: String, anchor_id: &str, focus_id: &str) -> Option<BarSelection> {
    if text.trim().is_empty() || anchor_id != focus_id {
        return None;
    }
    Some(BarSelection {
        message_id: Uuid::parse_str(anchor_id).ok()?,
        text,
    })
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
    selected: &BarSelection,
    state: SelectionState,
    action: SelectionAction,
    context: String,
    label: &str,
) -> Html {
    let request = (selected.message_id, action, selected.text.clone(), context);
    let onclick = action_callback(state, props.on_action.clone(), request);
    let disabled = props.translate_loading.contains(&selected.message_id);
    html! { <button class="translate-selection-btn" {onclick} {disabled}>{label}</button> }
}

#[function_component(SelectionActionButtons)]
pub fn selection_action_buttons(props: &Props) -> Html {
    let state = use_bar_selection();
    let Some(selected) = state.as_ref() else {
        return html! {};
    };
    let Some(context) = message_context(&props.messages, selected.message_id) else {
        return html! {};
    };
    html! {
        <div class="selection-action-bar">
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
            Some(BarSelection {
                message_id,
                text: " أهلا ".into()
            })
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
}
