use dialect_coach_shared::UserState;
use dialect_coach_shared::models::{ConversationBranch, Message};
use std::collections::HashSet;
use uuid::Uuid;
use web_sys::HtmlElement;
use yew::prelude::*;

use super::MessageBubble;

#[derive(Properties, PartialEq)]
pub struct ChatWindowProps {
    pub user_state: UserState,
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
    pub on_translate: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub explain_loading: HashSet<Uuid>,
    #[prop_or_default]
    pub translate_loading: HashSet<Uuid>,
}

fn has_child_branches(message_id: Uuid, branches: &[ConversationBranch]) -> bool {
    branches
        .iter()
        .any(|b| b.parent_message_id == Some(message_id))
}

fn needs_continue_button(messages: &[&Message]) -> bool {
    messages.last().map(|m| !m.is_agent()).unwrap_or(false)
}

fn render_start_button(on_auto_start: &Option<Callback<()>>, is_loading: bool) -> Html {
    match on_auto_start {
        Some(callback) => {
            let onclick = callback.reform(|_| ());
            html! {
                <div class="continue-branch-container">
                    <button
                        class="continue-button"
                        onclick={onclick}
                        disabled={is_loading}
                    >
                        {"Start conversation"}
                    </button>
                </div>
            }
        }
        None => html! {},
    }
}

#[function_component(ChatWindow)]
pub fn chat_window(props: &ChatWindowProps) -> Html {
    let chat_container_ref = use_node_ref();

    // Auto-scroll to bottom when new messages arrive
    {
        let chat_container_ref = chat_container_ref.clone();
        let message_count = props.user_state.conversation_history.len();

        use_effect_with(message_count, move |_| {
            if let Some(container) = chat_container_ref.cast::<HtmlElement>() {
                container.set_scroll_top(container.scroll_height());
            }
            || ()
        });
    }

    let active_messages = props.user_state.get_active_branch_messages();

    html! {
        <div class="chat" ref={chat_container_ref} role="log" aria-live="polite" aria-relevant="additions">
            <div class="chat-scroll">
                {if props.user_state.conversation_history.is_empty() {
                    html! {
                        <div class="empty">
                            <div class="empty-icon">{"💬"}</div>
                            <h3 class="empty-title">{"Ready to practice!"}</h3>
                            <p class="empty-body">{"Start a conversation to practice your dialect."}</p>
                            {render_start_button(&props.on_auto_start, props.is_loading)}
                        </div>
                    }
                } else {
                    html! {
                        <div class="messages-list">
                            {for active_messages.iter().map(|msg| {
                                let is_own = !msg.is_agent();
                                let has_children = has_child_branches(msg.id, &props.user_state.branches);
                                let language_option = props.user_state.language_options.for_language(msg.metadata.language);
                                let is_explain_loading = props.explain_loading.contains(&msg.id);
                                let is_translate_loading = props.translate_loading.contains(&msg.id);
                                html! {
                                    <MessageBubble
                                        message={(*msg).clone()}
                                        is_own_message={is_own}
                                        on_replay={props.on_replay_message.clone()}
                                        on_delete={props.on_delete_message.clone()}
                                        on_create_branch={props.on_create_branch.clone()}
                                        has_child_branches={has_children}
                                        on_explain={props.on_explain.clone()}
                                        on_translate={props.on_translate.clone()}
                                        {language_option}
                                        {is_explain_loading}
                                        {is_translate_loading}
                                    />
                                }
                            })}
                        </div>
                    }
                }}

                {if props.is_loading {
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
                } else {
                    html! {}
                }}

                {if !props.is_loading && needs_continue_button(&active_messages) {
                    if let Some(callback) = &props.on_continue_branch {
                        if let Some(last_msg) = active_messages.last() {
                            let id = last_msg.id;
                            html! {
                                <div class="continue-branch-container">
                                    <button
                                        class="continue-button"
                                        onclick={callback.reform(move |_| id)}
                                    >
                                        {"Continue conversation"}
                                    </button>
                                </div>
                            }
                        } else {
                            html! {}
                        }
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
