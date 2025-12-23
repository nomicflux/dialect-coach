use dialect_coach_shared::models::{ConversationBranch, Message};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct BranchesProps {
    pub branches: Vec<ConversationBranch>,
    pub active_branch_id: Uuid,
    pub messages: Vec<Message>,
    #[prop_or_default]
    pub on_switch_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_delete_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_rename_branch: Option<Callback<(Uuid, String)>>,
}

fn count_branch_messages(messages: &[Message], branch: &ConversationBranch) -> usize {
    let mut count = 0;
    let mut current_id = branch.leaf_message_id;

    while let Some(msg_id) = current_id {
        if let Some(msg) = messages.iter().find(|m| m.id == msg_id) {
            count += 1;
            current_id = msg.parent_id;
        } else {
            break;
        }
    }

    count
}

fn truncate_preview(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let mut preview = String::new();

    for _ in 0..max_chars {
        match chars.next() {
            Some(ch) => preview.push(ch),
            None => return preview,
        }
    }

    if chars.next().is_some() {
        preview.push_str("...");
    }

    preview
}

fn get_branch_display_name(messages: &[Message], branch: &ConversationBranch) -> String {
    let leaf_id = match branch.leaf_message_id {
        Some(id) => id,
        None => return "Empty".to_string(),
    };

    let mut current_id = Some(leaf_id);
    let mut target_msg = None;

    while let Some(msg_id) = current_id {
        if let Some(msg) = messages.iter().find(|m| m.id == msg_id) {
            if msg.parent_id == branch.parent_message_id {
                target_msg = Some(msg);
                break;
            }
            current_id = msg.parent_id;
        } else {
            break;
        }
    }

    if let Some(msg) = target_msg {
        let content = msg.get_content();
        truncate_preview(&content, 30)
    } else {
        "Empty".to_string()
    }
}

fn render_branch_item(
    branch: &ConversationBranch,
    messages: &[Message],
    is_active: bool,
    message_count: usize,
    on_switch: &Option<Callback<Uuid>>,
    on_delete: &Option<Callback<Uuid>>,
) -> Html {
    let branch_class = if is_active {
        "branch-item branch-item--active"
    } else {
        "branch-item"
    };
    let branch_id = branch.id;
    let branch_name = get_branch_display_name(messages, branch);
    let dialect_name = branch
        .dialect
        .map(|d| d.name())
        .unwrap_or("New conversation");

    html! {
        <div class={branch_class}>
            <div class="branch-info">
                <span class="branch-dialect">{dialect_name}</span>
                <span class="branch-name">{branch_name}</span>
                <span class="branch-count">{format!("({} msgs)", message_count)}</span>
            </div>
            <div class="branch-actions">
                {if !is_active {
                    if let Some(callback) = on_switch {
                        let cb = callback.clone();
                        html! {
                            <button
                                class="branch-button"
                                onclick={Callback::from(move |_| cb.emit(branch_id))}
                                title="Switch to this branch"
                            >{"→"}</button>
                        }
                    } else {
                        html! {}
                    }
                } else {
                    html! {}
                }}
                {if let Some(callback) = on_delete {
                    let cb = callback.clone();
                    html! {
                        <button
                            class="branch-button branch-button--delete"
                            onclick={Callback::from(move |_| cb.emit(branch_id))}
                            title="Delete branch"
                        >{"×"}</button>
                    }
                } else {
                    html! {}
                }}
            </div>
        </div>
    }
}

#[function_component(Branches)]
pub fn branches(props: &BranchesProps) -> Html {
    html! {
        <div class="branch-list-container">
            // Removed header and toggle button as they are now handled by parent container

            <div class="branch-list">
                {for props.branches.iter().map(|branch| {
                    let is_active = branch.id == props.active_branch_id;
                    let msg_count = count_branch_messages(&props.messages, branch);
                    render_branch_item(branch, &props.messages, is_active, msg_count, &props.on_switch_branch, &props.on_delete_branch)
                })}
            </div>
        </div>
    }
}
