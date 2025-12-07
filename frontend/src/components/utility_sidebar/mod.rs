pub mod branches;
pub mod learning;
pub mod settings;
use branches::Branches;
use learning::Learning;
use settings::Settings;
use crate::app::app_state::{OptionalUserState, UIState};
use crate::services::enrichment_service::EnrichmentService;
use dialect_coach_shared::models::{ConversationBranch, LearningGoal, Message, LearningItem};
use dialect_coach_shared::Dialect;
use uuid::Uuid;
use std::rc::Rc;
use yew::prelude::*;

#[derive(PartialEq, Clone, Copy)]
pub enum SidebarTab {
    Branches,
    Learning,
    Settings,
}

#[derive(Properties, PartialEq)]
pub struct UtilitySidebarProps {
    pub is_collapsed: bool,
    pub active_tab: SidebarTab,
    pub on_tab_change: Callback<SidebarTab>,
    pub user_state: UseReducerHandle<OptionalUserState>,
    pub ui_state: UseReducerHandle<UIState>,
    // Branch Data
    pub branches: Vec<ConversationBranch>,
    pub active_branch_id: Uuid,
    pub messages: Vec<Message>,
    pub learning_goals: Vec<LearningGoal>,
    pub on_add_goal: Callback<String>,
    pub on_delete_goal: Callback<usize>,
    #[prop_or_default]
    pub on_switch_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_delete_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub goal_input_ref: Option<NodeRef>,
    // Learning Data
    pub learning_items: Vec<LearningItem>,
    pub active_branch_dialect: Option<Dialect>,
    pub enrichment_service: Rc<EnrichmentService>,
    pub on_delete_learning_item: Callback<Uuid>,
    pub on_undo_delete_learning_item: Callback<()>,
    pub deleted_learning_items_count: usize,
}

#[function_component(UtilitySidebar)]
pub fn utility_sidebar(props: &UtilitySidebarProps) -> Html {
    let on_tab_click = |tab: SidebarTab| {
        let on_tab_change = props.on_tab_change.clone();
        Callback::from(move |_| on_tab_change.emit(tab))
    };

    let sidebar_class = if props.is_collapsed {
        "utility-sidebar collapsed"
    } else {
        "utility-sidebar"
    };

    html! {
        <div class={sidebar_class}>
            <div class="sidebar-tabs">
                <button
                    class={classes!("sidebar-tab", (props.active_tab == SidebarTab::Branches).then_some("active"))}
                    onclick={on_tab_click(SidebarTab::Branches)}
                >
                    {"Branches"}
                </button>
                <button
                    class={classes!("sidebar-tab", (props.active_tab == SidebarTab::Learning).then_some("active"))}
                    onclick={on_tab_click(SidebarTab::Learning)}
                >
                    {"Learning"}
                </button>
                <button
                    class={classes!("sidebar-tab", (props.active_tab == SidebarTab::Settings).then_some("active"))}
                    onclick={on_tab_click(SidebarTab::Settings)}
                >
                    {"Settings"}
                </button>
            </div>

            <div class="sidebar-content">
                {match props.active_tab {
                    SidebarTab::Branches => html! {
                        <Branches
                            branches={props.branches.clone()}
                            active_branch_id={props.active_branch_id}
                            messages={props.messages.clone()}
                            learning_goals={props.learning_goals.clone()}
                            on_add_goal={props.on_add_goal.clone()}
                            on_delete_goal={props.on_delete_goal.clone()}
                            on_switch_branch={props.on_switch_branch.clone()}
                            on_delete_branch={props.on_delete_branch.clone()}
                            goal_input_ref={props.goal_input_ref.clone()}
                        />
                    },
                    SidebarTab::Learning => html! {
                        <Learning
                            items={props.learning_items.clone()}
                            on_delete={props.on_delete_learning_item.clone()}
                            on_undo={props.on_undo_delete_learning_item.clone()}
                            deleted_count={props.deleted_learning_items_count}
                            active_branch_dialect={props.active_branch_dialect}
                            user_state={props.user_state.clone()}
                            enrichment_service={props.enrichment_service.clone()}
                        />
                    },
                    SidebarTab::Settings => html! {
                        <Settings
                            user_state={props.user_state.clone()}
                            ui_state={props.ui_state.clone()}
                        />
                    },
                }}
            </div>
        </div>
    }
}
