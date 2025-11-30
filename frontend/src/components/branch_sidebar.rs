use crate::components::LearningGoalsPanel;
use dialect_coach_shared::models::{ConversationBranch, LearningGoal, Message};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct BranchSidebarProps {
    pub branches: Vec<ConversationBranch>,
    pub active_branch_id: Uuid,
    pub messages: Vec<Message>,
    pub learning_goals: Vec<LearningGoal>,
    pub on_add_goal: Callback<String>,
    pub on_delete_goal: Callback<usize>,
    pub is_collapsed: bool,
    pub on_toggle: Callback<()>,
    #[prop_or_default]
    pub on_switch_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_delete_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_rename_branch: Option<Callback<(Uuid, String)>>,
    #[prop_or_default]
    pub goal_input_ref: Option<NodeRef>,
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
    // If this branch has no parent (root branch), show first message in path
    // If this branch has a parent (branched off), show first message after the parent

    let leaf_id = match branch.leaf_message_id {
        Some(id) => id,
        None => return "Empty".to_string(),
    };

    // Walk backwards from leaf to find either:
    // - The first message (if parent_message_id is None)
    // - The first message after the branch point (if parent_message_id is Some)
    let mut current_id = Some(leaf_id);
    let mut target_msg = None;

    while let Some(msg_id) = current_id {
        if let Some(msg) = messages.iter().find(|m| m.id == msg_id) {
            // If this message's parent matches the branch's parent, this is the message we want
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

fn render_collapsed_branch_indicator(
    branch: &ConversationBranch,
    is_active: bool,
    message_count: usize,
    on_switch: &Option<Callback<Uuid>>,
) -> Html {
    let indicator_class = if is_active {
        "branch-indicator branch-indicator--active"
    } else {
        "branch-indicator"
    };
    let branch_id = branch.id;

    html! {
        <div
            class={indicator_class}
            title={format!("{} messages", message_count)}
            onclick={if let Some(callback) = on_switch {
                let cb = callback.clone();
                Some(Callback::from(move |_| cb.emit(branch_id)))
            } else {
                None
            }}
        >
        </div>
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
    let dialect_name = branch.dialect.map(|d| d.name()).unwrap_or("New conversation");

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

fn render_expanded_view(props: &BranchSidebarProps) -> Html {
    html! {
        <>
            <div class="branch-sidebar-header">
                <h3>{"Conversation Branches"}</h3>
            </div>
            <button
                class="sidebar-toggle"
                onclick={Callback::from({
                    let on_toggle = props.on_toggle.clone();
                    move |_| on_toggle.emit(())
                })}
                title="Collapse sidebar"
            >
                {"◄"}
            </button>
            <div class="branch-list">
                {for props.branches.iter().map(|branch| {
                    let is_active = branch.id == props.active_branch_id;
                    let msg_count = count_branch_messages(&props.messages, branch);
                    render_branch_item(branch, &props.messages, is_active, msg_count, &props.on_switch_branch, &props.on_delete_branch)
                })}
            </div>
            <LearningGoalsPanel
                goals={props.learning_goals.clone()}
                on_add={props.on_add_goal.clone()}
                on_delete={props.on_delete_goal.clone()}
                input_ref={props.goal_input_ref.clone()}
            />
        </>
    }
}

fn render_collapsed_view(props: &BranchSidebarProps) -> Html {
    let goals_count = props.learning_goals.len();

    html! {
        <>
            <div class="branch-sidebar-header">
            </div>
            <button
                class="sidebar-toggle"
                onclick={Callback::from({
                    let on_toggle = props.on_toggle.clone();
                    move |_| on_toggle.emit(())
                })}
                title="Expand sidebar"
            >
                {"►"}
            </button>
            <div class="branch-indicators">
                {for props.branches.iter().map(|branch| {
                    let is_active = branch.id == props.active_branch_id;
                    let msg_count = count_branch_messages(&props.messages, branch);
                    render_collapsed_branch_indicator(branch, is_active, msg_count, &props.on_switch_branch)
                })}
            </div>
            <div class="learning-goals-count" title="Learning Goals">
                <span class="learning-goals-label">{"Goals"}</span>
                <span class="learning-goals-number">{goals_count}</span>
            </div>
        </>
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_preview;

    #[test]
    fn truncate_preview_keeps_short_ascii() {
        assert_eq!(truncate_preview("Hello", 10), "Hello");
    }

    #[test]
    fn truncate_preview_truncates_ascii() {
        assert_eq!(truncate_preview("abcdef", 3), "abc...");
    }

    #[test]
    fn truncate_preview_handles_arabic() {
        assert_eq!(truncate_preview("مرحبا", 3), "مرح...");
    }

    #[test]
    fn truncate_preview_handles_japanese() {
        assert_eq!(truncate_preview("こんにちは世界", 4), "こんにち...");
    }

    #[test]
    fn truncate_preview_handles_emoji() {
        assert_eq!(truncate_preview("🙂🙂🙂🙂", 2), "🙂🙂...");
    }
}

#[function_component(BranchSidebar)]
pub fn branch_sidebar(props: &BranchSidebarProps) -> Html {
    let sidebar_class = if props.is_collapsed {
        "branch-sidebar branch-sidebar--collapsed"
    } else {
        "branch-sidebar"
    };

    html! {
        <div class={sidebar_class}>
            {if props.is_collapsed {
                render_collapsed_view(props)
            } else {
                render_expanded_view(props)
            }}
        </div>
    }
}
