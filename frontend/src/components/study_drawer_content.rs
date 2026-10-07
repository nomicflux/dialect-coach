use crate::app::app_state::user::UserDomainAction;
use crate::components::usage_panel::UsagePanel;
use crate::components::utility_sidebar::branches::Branches;
use crate::components::utility_sidebar::learning::Learning;
use crate::components::utility_sidebar::plan::PlanTab;
use crate::components::utility_sidebar::settings::Settings;
use crate::services::enrichment_service::EnrichmentService;
use crate::services::plan_service::PlanService;
use crate::utils::perf::PerfGuard;
use dialect_coach_shared::Dialect;
use dialect_coach_shared::UserState;
use dialect_coach_shared::models::{ConversationBranch, LearningGoal, LearningItem, Message};
use std::rc::Rc;
use std::sync::Arc;
use uuid::Uuid;
use yew::prelude::*;

#[derive(PartialEq, Clone, Copy)]
pub enum DrawerTab {
    Branches,
    Learning,
    Plan,
    Settings,
    Usage,
}

#[derive(Properties, PartialEq)]
pub struct StudyDrawerContentProps {
    pub active_tab: DrawerTab,
    pub on_tab_change: Callback<DrawerTab>,
    pub user: Rc<UserState>, // Strict prop
    pub is_admin: bool,
    pub dispatch: Callback<UserDomainAction>, // Strict prop
    pub branches: Arc<Vec<ConversationBranch>>,
    pub active_branch_id: Uuid,
    pub messages: Arc<Vec<Message>>,
    pub learning_goals: Arc<Vec<LearningGoal>>,
    pub on_add_goal: Callback<String>,
    pub on_delete_goal: Callback<usize>,
    #[prop_or_default]
    pub on_switch_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_delete_branch: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub goal_input_ref: Option<NodeRef>,
    pub learning_items: Rc<Vec<LearningItem>>,
    pub active_branch_dialect: Option<Dialect>,
    pub enrichment_service: Rc<EnrichmentService>,
    pub plan_service: Rc<PlanService>,
    pub on_delete_learning_item: Callback<Uuid>,
    pub on_undo_delete_learning_item: Callback<()>,
    pub deleted_learning_items_count: usize,
    pub on_log_out: Callback<MouseEvent>,
}

fn render_tab_button(
    label: &str,
    tab: DrawerTab,
    active_tab: DrawerTab,
    on_click: Callback<DrawerTab>,
) -> Html {
    let active_class = if active_tab == tab { "active" } else { "" };
    let onclick = Callback::from(move |_| on_click.emit(tab));
    html! {
        <button class={classes!("drawer-tab", active_class)} {onclick}>{label}</button>
    }
}

fn render_tab_bar(props: &StudyDrawerContentProps) -> Html {
    html! {
        <div class="study-tabs">
            {render_tab_button("Branches", DrawerTab::Branches, props.active_tab, props.on_tab_change.clone())}
            {render_tab_button("Learning", DrawerTab::Learning, props.active_tab, props.on_tab_change.clone())}
            {render_tab_button("Plan", DrawerTab::Plan, props.active_tab, props.on_tab_change.clone())}
            {render_tab_button("Settings", DrawerTab::Settings, props.active_tab, props.on_tab_change.clone())}
            {render_tab_button("Usage", DrawerTab::Usage, props.active_tab, props.on_tab_change.clone())}
        </div>
    }
}

fn render_branches(props: &StudyDrawerContentProps) -> Html {
    html! {
        <Branches branches={props.branches.clone()} active_branch_id={props.active_branch_id}
            messages={props.messages.clone()} on_switch_branch={props.on_switch_branch.clone()}
            on_delete_branch={props.on_delete_branch.clone()} />
    }
}

fn render_learning(props: &StudyDrawerContentProps) -> Html {
    html! {
        <Learning items={props.learning_items.clone()} on_delete={props.on_delete_learning_item.clone()}
            on_undo={props.on_undo_delete_learning_item.clone()} deleted_count={props.deleted_learning_items_count}
            active_branch_dialect={props.active_branch_dialect} user={props.user.clone()}
            dispatch={props.dispatch.clone()} enrichment_service={props.enrichment_service.clone()} />
    }
}

fn render_plan(props: &StudyDrawerContentProps) -> Html {
    html! {
        <PlanTab learning_goals={(*props.learning_goals).clone()} on_add_goal={props.on_add_goal.clone()}
            on_delete_goal={props.on_delete_goal.clone()} goal_input_ref={props.goal_input_ref.clone()}
            user={props.user.clone()} dispatch={props.dispatch.clone()} plan_service={props.plan_service.clone()}
            enrichment_service={props.enrichment_service.clone()} active_branch_dialect={props.active_branch_dialect} />
    }
}

fn render_settings(props: &StudyDrawerContentProps) -> Html {
    html! {
        <Settings user={props.user.clone()} is_admin={props.is_admin}
            dispatch={props.dispatch.clone()} on_log_out={props.on_log_out.clone()} />
    }
}

fn render_tab_content(props: &StudyDrawerContentProps) -> Html {
    match props.active_tab {
        DrawerTab::Branches => render_branches(props),
        DrawerTab::Learning => render_learning(props),
        DrawerTab::Plan => render_plan(props),
        DrawerTab::Settings => render_settings(props),
        DrawerTab::Usage => html! { <UsagePanel usage_stats={props.user.usage_stats.clone()} /> },
    }
}

#[function_component(StudyDrawerContent)]
pub fn study_drawer_content(props: &StudyDrawerContentProps) -> Html {
    let _render_guard = PerfGuard::new("StudyDrawerContent::render");
    html! {
        <div class="study-drawer-content">
            {render_tab_bar(props)}
            <div class="study-tab-content">{render_tab_content(props)}</div>
        </div>
    }
}
