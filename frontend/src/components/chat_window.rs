use dialect_coach_shared::UserState;
use dialect_coach_shared::models::{ConversationBranch, LanguageOption, Message};
use std::collections::HashSet;
use std::rc::Rc;
use uuid::Uuid;
use web_sys::{HtmlElement, MouseEvent};
use yew::prelude::*;

use super::MessageBubble;
use super::icons::NeonRope;

#[derive(Properties, PartialEq)]
pub struct ChatWindowProps {
    pub user: Rc<UserState>, // Strict prop
    pub is_loading: bool,
    #[prop_or_default]
    pub on_replay_message: Option<Callback<Message>>,
    #[prop_or_default]
    pub on_delete_message: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_create_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_auto_start: Option<Callback<()>>,
    #[prop_or_default]
    pub on_continue_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_explain: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub explain_loading: HashSet<Uuid>,
    #[prop_or_default]
    pub translate_loading: HashSet<Uuid>,
    #[prop_or_default]
    pub on_selection_translate: Option<Callback<(Uuid, String, String)>>,
}

fn has_child_branches(message_id: Uuid, branches: &[ConversationBranch]) -> bool {
    branches
        .iter()
        .any(|b| b.parent_message_id == Some(message_id))
}

fn needs_continue_button(messages: &[&Message]) -> bool {
    messages.last().map(|m| !m.is_agent()).unwrap_or(false)
}

fn render_empty_state(on_auto_start: &Option<Callback<()>>, is_loading: bool) -> Html {
    html! {
        <div class="empty">
            <div class="empty-icon">{"💬"}</div>
            <h3 class="empty-title">{"Ready to practice!"}</h3>
            <p class="empty-body">{"Start a conversation to practice your dialect."}</p>
            {render_start_button(on_auto_start, is_loading)}
        </div>
    }
}

fn render_start_button(on_auto_start: &Option<Callback<()>>, is_loading: bool) -> Html {
    match on_auto_start {
        Some(callback) => {
            let onclick = callback.reform(|_| ());
            html! {
                <div class="continue-branch-container">
                    <button class="continue-button" onclick={onclick} disabled={is_loading}>
                        {"Start conversation"}
                    </button>
                </div>
            }
        }
        None => html! {},
    }
}

fn render_rope(
    msg_id: Uuid,
    is_own: bool,
    has_children: bool,
    on_create_branch: &Option<Callback<Uuid>>,
) -> Html {
    if let Some(cb) = on_create_branch.clone() {
        let onclick = Callback::from(move |_: MouseEvent| cb.emit(msg_id));
        let row_class = if is_own {
            "rope-row--user"
        } else {
            "rope-row--agent"
        };
        html! {
            <div class={classes!("rope-row", row_class)}>
                <div class="neon-rope-container" onclick={onclick}>
                    <NeonRope id={msg_id} has_children={has_children} is_user_message={is_own} />
                </div>
            </div>
        }
    } else {
        html! {}
    }
}

fn render_message_with_rope(
    props: &ChatWindowProps,
    msg: &Message,
    is_own: bool,
    has_children: bool,
    language_option: Option<LanguageOption>,
) -> Html {
    let is_explain_loading = props.explain_loading.contains(&msg.id);
    let is_translate_loading = props.translate_loading.contains(&msg.id);

    html! {
        <>
            <MessageBubble
                message={msg.clone()}
                is_own_message={is_own}
                on_replay={props.on_replay_message.clone()}
                on_delete={props.on_delete_message.clone()}
                on_create_branch={props.on_create_branch.clone()}
                has_child_branches={has_children}
                on_explain={props.on_explain.clone()}
                {language_option}
                {is_explain_loading}
                {is_translate_loading}
                on_selection_translate={props.on_selection_translate.clone()}
            />
            {render_rope(msg.id, is_own, has_children, &props.on_create_branch)}
        </>
    }
}

fn render_messages_list(props: &ChatWindowProps, active_messages: &[&Message]) -> Html {
    html! {
        <div class="messages-list">
            {for active_messages.iter().map(|msg| {
                let is_own = !msg.is_agent();
                let has_children = has_child_branches(msg.id, &props.user.branches);
                let language_option = props.user.language_options.for_language(msg.metadata.language);

                render_message_with_rope(props, msg, is_own, has_children, language_option)
            })}
        </div>
    }
}

fn render_loading_indicator() -> Html {
    html! {
        <div class="loading-indicator">
            <div class="typing">
                <div class="typing-dots">
                    <div class="typing-dot"></div>
                    <div class="typing-dot"></div>
                    <div class="typing-dot"></div>
                </div>
                <span class="loading-text">{"Agent is typing..."}</span>
            </div>
        </div>
    }
}

fn render_continue_button(on_continue_branch: &Option<Callback<Uuid>>, last_msg_id: Uuid) -> Html {
    if let Some(callback) = on_continue_branch {
        html! {
            <div class="continue-branch-container">
                <button class="continue-button" onclick={callback.reform(move |_| last_msg_id)}>
                    {"Continue conversation"}
                </button>
            </div>
        }
    } else {
        html! {}
    }
}

#[function_component(ChatWindow)]
pub fn chat_window(props: &ChatWindowProps) -> Html {
    let chat_container_ref = use_node_ref();

    // Auto-scroll to bottom when new messages arrive
    {
        let chat_container_ref = chat_container_ref.clone();
        let message_count = props.user.conversation_history.len();

        use_effect_with(message_count, move |_| {
            if let Some(container) = chat_container_ref.cast::<HtmlElement>() {
                container.set_scroll_top(container.scroll_height());
            }
            || ()
        });
    }

    let active_messages = props.user.get_active_branch_messages();

    html! {
        <div class="chat" ref={chat_container_ref} role="log" aria-live="polite" aria-relevant="additions">
            <div class="chat-scroll">
                {if props.user.conversation_history.is_empty() {
                    render_empty_state(&props.on_auto_start, props.is_loading)
                } else {
                    render_messages_list(props, &active_messages)
                }}

                {if props.is_loading {
                    render_loading_indicator()
                } else {
                    html! {}
                }}

                {if !props.is_loading && needs_continue_button(&active_messages) {
                    if let Some(last_msg) = active_messages.last() {
                        render_continue_button(&props.on_continue_branch, last_msg.id)
                    } else {
                        html! {}
                    }
                } else {
                    html! {}
                }}
            </div>
        </div>
    }
}
