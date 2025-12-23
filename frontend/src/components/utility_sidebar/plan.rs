use crate::app::app_state::PlanAction;
use crate::app::app_state::user::UserDomainAction;
use crate::components::LearningGoalsPanel;
use crate::components::plan::generator::PlanGenerator;
use crate::components::plan::{ActivePlan, PlanCreate, PlanList};
use crate::services::enrichment_service::EnrichmentService;
use crate::services::plan_service::PlanService;
use dialect_coach_shared::models::{LanguagePlan, LearningGoal};
use dialect_coach_shared::{Dialect, UserState};
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::{Blob, HtmlAnchorElement, HtmlInputElement, Url};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PlanTabProps {
    pub user: Rc<UserState>,
    pub dispatch: Callback<UserDomainAction>,
    pub plan_service: Rc<PlanService>,
    pub enrichment_service: Rc<EnrichmentService>,
    pub active_branch_dialect: Option<Dialect>,
    pub learning_goals: Vec<LearningGoal>,
    pub on_add_goal: Callback<String>,
    pub on_delete_goal: Callback<usize>,
    #[prop_or_default]
    pub goal_input_ref: Option<NodeRef>,
}

// --- Callback Factories ---

fn make_export_callback(user: Rc<UserState>) -> Callback<Uuid> {
    Callback::from(move |plan_id: Uuid| {
        if let Some(plan) = user.language_plans.iter().find(|p| p.id == plan_id) {
            export_plan_to_yaml(plan);
        }
    })
}

fn export_plan_to_yaml(plan: &LanguagePlan) {
    let Ok(yaml) = serde_yaml::to_string(plan) else {
        gloo::console::error!("Failed to serialize plan");
        return;
    };

    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(&yaml));
    let properties = web_sys::BlobPropertyBag::new();
    properties.set_type("application/x-yaml");

    if let Ok(blob) = Blob::new_with_str_sequence_and_options(&parts, &properties)
        && let Ok(url) = Url::create_object_url_with_blob(&blob)
        && let Some(window) = web_sys::window()
        && let Some(document) = window.document()
        && let Ok(anchor) = document.create_element("a")
        && let Ok(anchor) = anchor.dyn_into::<HtmlAnchorElement>()
    {
        anchor.set_href(&url);
        anchor.set_download(&format!("{}.yaml", plan.title));
        anchor.click();
        let _ = Url::revoke_object_url(&url);
    }
}

fn make_import_click_callback(file_input_ref: NodeRef) -> Callback<MouseEvent> {
    Callback::from(move |_| {
        if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
            input.click();
        }
    })
}

fn make_file_change_callback(
    file_input_ref: NodeRef,
    dispatch: Callback<UserDomainAction>,
    enrichment_service: Rc<EnrichmentService>,
    is_importing: UseStateHandle<bool>,
) -> Callback<Event> {
    Callback::from(move |_e: Event| {
        let Some(input) = file_input_ref.cast::<HtmlInputElement>() else {
            return;
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };

        let dispatch = dispatch.clone();
        let enrichment_service = enrichment_service.clone();
        let is_importing = is_importing.clone();

        is_importing.set(true);
        spawn_import_task(file, dispatch, enrichment_service, is_importing);
        input.set_value("");
    })
}

fn spawn_import_task(
    file: web_sys::File,
    dispatch: Callback<UserDomainAction>,
    enrichment_service: Rc<EnrichmentService>,
    is_importing: UseStateHandle<bool>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let future = wasm_bindgen_futures::JsFuture::from(file.text());
        match future.await {
            Ok(text_val) if text_val.as_string().is_some() => {
                let text = text_val.as_string().unwrap();
                match super::import_workflow::process_imported_text(text, enrichment_service).await
                {
                    Ok(plan) => dispatch.emit(UserDomainAction::Plan(PlanAction::Add(plan))),
                    Err(e) => gloo::console::error!("Import failed", e),
                }
            }
            _ => gloo::console::error!("Failed to read file"),
        }
        is_importing.set(false);
    });
}

// --- Render Helpers ---

fn render_active_plan(user: &Rc<UserState>, dispatch: &Callback<UserDomainAction>) -> Html {
    let Some(active_plan_id) = user.active_plan_id else {
        return html! {};
    };
    let Some(active_plan) = user.language_plans.iter().find(|p| p.id == active_plan_id) else {
        return html! {};
    };

    let on_advance = make_plan_action_callback(dispatch.clone(), PlanAction::AdvanceStep);
    let on_activate = make_plan_action_callback(dispatch.clone(), PlanAction::ActivateStep);
    let on_back = make_deactivate_callback(dispatch.clone());

    html! {
        <>
            <ActivePlan plan={active_plan.clone()} {on_advance} {on_activate} />
            <button class="view-all-plans-btn" onclick={on_back}>{"← Back to All Plans"}</button>
        </>
    }
}

fn make_plan_action_callback(
    dispatch: Callback<UserDomainAction>,
    action_fn: fn(Uuid) -> PlanAction,
) -> Callback<Uuid> {
    Callback::from(move |id| dispatch.emit(UserDomainAction::Plan(action_fn(id))))
}

fn make_deactivate_callback(dispatch: Callback<UserDomainAction>) -> Callback<MouseEvent> {
    Callback::from(move |_| dispatch.emit(UserDomainAction::Plan(PlanAction::SetActive(None))))
}

fn render_generator(
    plan_service: Rc<PlanService>,
    dispatch: Callback<UserDomainAction>,
    enrichment_service: Rc<EnrichmentService>,
    show_generator: UseStateHandle<bool>,
) -> Html {
    let on_plan_generated = {
        let dispatch = dispatch.clone();
        let enrichment_service = enrichment_service.clone();
        let show_generator = show_generator.clone();
        Callback::from(move |simple_plan| {
            super::import_workflow::handle_generated_plan(
                simple_plan,
                dispatch.clone(),
                enrichment_service.clone(),
                show_generator.clone(),
            );
        })
    };
    let on_cancel = {
        let show_generator = show_generator.clone();
        Callback::from(move |_| show_generator.set(false))
    };

    html! { <PlanGenerator {plan_service} {on_plan_generated} {on_cancel} /> }
}

fn render_create_form(
    dialect: Dialect,
    plan_to_edit: Option<LanguagePlan>,
    dispatch: Callback<UserDomainAction>,
    enrichment_service: Rc<EnrichmentService>,
    show_create_plan: UseStateHandle<bool>,
    editing_plan_id: UseStateHandle<Option<Uuid>>,
) -> Html {
    let on_create = {
        let dispatch = dispatch.clone();
        let show_create_plan = show_create_plan.clone();
        let editing_plan_id = editing_plan_id.clone();
        Callback::from(move |plan: LanguagePlan| {
            let action = if editing_plan_id.is_some() {
                PlanAction::Update(plan)
            } else {
                PlanAction::Add(plan)
            };
            dispatch.emit(UserDomainAction::Plan(action));
            show_create_plan.set(false);
            editing_plan_id.set(None);
        })
    };
    let on_cancel = {
        let show_create_plan = show_create_plan.clone();
        let editing_plan_id = editing_plan_id.clone();
        Callback::from(move |_| {
            show_create_plan.set(false);
            editing_plan_id.set(None);
        })
    };

    html! { <PlanCreate {dialect} {plan_to_edit} {on_create} {on_cancel} {enrichment_service} /> }
}

fn render_plan_list(
    plans: Vec<LanguagePlan>,
    active_plan_id: Option<Uuid>,
    dispatch: Callback<UserDomainAction>,
    editing_plan_id: UseStateHandle<Option<Uuid>>,
    on_export_plan: Callback<Uuid>,
) -> Html {
    let on_select_plan = {
        let dispatch = dispatch.clone();
        Callback::from(move |id: Option<Uuid>| {
            dispatch.emit(UserDomainAction::Plan(PlanAction::SetActive(id)));
        })
    };
    let on_delete_plan = make_plan_action_callback(dispatch.clone(), PlanAction::Delete);
    let on_edit_plan = {
        let editing_plan_id = editing_plan_id.clone();
        Callback::from(move |id| editing_plan_id.set(Some(id)))
    };

    html! {
        <PlanList {plans} {active_plan_id} {on_select_plan} {on_delete_plan} {on_edit_plan} {on_export_plan} />
    }
}

fn render_action_buttons(
    show_create_plan: UseStateHandle<bool>,
    show_generator: UseStateHandle<bool>,
    editing_plan_id: UseStateHandle<Option<Uuid>>,
    on_import_click: Callback<MouseEvent>,
    on_file_change: Callback<Event>,
    file_input_ref: NodeRef,
    is_importing: bool,
) -> Html {
    let on_create_click = {
        let show_create_plan = show_create_plan.clone();
        let editing_plan_id = editing_plan_id.clone();
        Callback::from(move |_| {
            editing_plan_id.set(None);
            show_create_plan.set(true);
        })
    };
    let on_generate_click = {
        let show_generator = show_generator.clone();
        Callback::from(move |_| show_generator.set(true))
    };

    html! {
        <div class="plan-list-actions" style="display: flex; gap: var(--s-2);">
            <button class="create-plan-button" onclick={on_create_click}>{"+ Create New Plan"}</button>
            <button
                class="generate-plan-button"
                style="background: var(--surface-muted); color: var(--ink); border: 1px solid rgba(0,0,0,0.1);"
                onclick={on_generate_click}
            >{"✨ Generate with AI"}</button>
            <button
                class="import-plan-button"
                onclick={on_import_click}
                disabled={is_importing}
                style="background: var(--surface-muted); color: var(--ink); border: 1px solid rgba(0,0,0,0.1);"
            >{if is_importing { "Importing..." } else { "Import Plan" }}</button>
            <input type="file" accept=".yaml,.yml" ref={file_input_ref} style="display: none;" onchange={on_file_change} />
        </div>
    }
}

// --- Main Component ---

#[function_component(PlanTab)]
pub fn plan_tab(props: &PlanTabProps) -> Html {
    let show_create_plan = use_state(|| false);
    let show_generator = use_state(|| false);
    let editing_plan_id = use_state(|| None::<Uuid>);
    let is_importing = use_state(|| false);
    let file_input_ref = use_node_ref();

    let plan_to_edit = editing_plan_id.as_ref().and_then(|id| {
        props
            .user
            .language_plans
            .iter()
            .find(|p| p.id == *id)
            .cloned()
    });
    let filtered_plans: Vec<LanguagePlan> = props
        .active_branch_dialect
        .map(|d| {
            props
                .user
                .language_plans
                .iter()
                .filter(|p| p.dialect == d)
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    let on_export_plan = make_export_callback(props.user.clone());
    let on_import_click = make_import_click_callback(file_input_ref.clone());
    let on_file_change = make_file_change_callback(
        file_input_ref.clone(),
        props.dispatch.clone(),
        props.enrichment_service.clone(),
        is_importing.clone(),
    );

    html! {
        <div class="plan-tab-content">
            <LearningGoalsPanel
                goals={props.learning_goals.clone()}
                on_add={props.on_add_goal.clone()}
                on_delete={props.on_delete_goal.clone()}
                input_ref={props.goal_input_ref.clone()}
            />
            <hr class="plan-divider" />
            <div class="plans-section">
                {render_plans_content(PlanContext {
                    props,
                    plan_to_edit,
                    filtered_plans,
                    on_export_plan,
                    on_import_click,
                    on_file_change,
                    file_input_ref,
                    show_create_plan,
                    show_generator,
                    editing_plan_id,
                    is_importing: *is_importing,
                })}
            </div>
        </div>
    }
}

/// Groups render context to avoid too_many_arguments warning
struct PlanContext<'a> {
    props: &'a PlanTabProps,
    plan_to_edit: Option<LanguagePlan>,
    filtered_plans: Vec<LanguagePlan>,
    on_export_plan: Callback<Uuid>,
    on_import_click: Callback<MouseEvent>,
    on_file_change: Callback<Event>,
    file_input_ref: NodeRef,
    show_create_plan: UseStateHandle<bool>,
    show_generator: UseStateHandle<bool>,
    editing_plan_id: UseStateHandle<Option<Uuid>>,
    is_importing: bool,
}

fn render_plans_content(ctx: PlanContext<'_>) -> Html {
    if ctx.props.user.active_plan_id.is_some() {
        render_active_plan(&ctx.props.user, &ctx.props.dispatch)
    } else if *ctx.show_create_plan || ctx.editing_plan_id.is_some() || *ctx.show_generator {
        render_edit_mode(
            ctx.props,
            ctx.plan_to_edit,
            ctx.show_create_plan,
            ctx.show_generator,
            ctx.editing_plan_id,
        )
    } else {
        render_list_mode(&ctx)
    }
}

fn render_edit_mode(
    props: &PlanTabProps,
    plan_to_edit: Option<LanguagePlan>,
    show_create_plan: UseStateHandle<bool>,
    show_generator: UseStateHandle<bool>,
    editing_plan_id: UseStateHandle<Option<Uuid>>,
) -> Html {
    if *show_generator {
        render_generator(
            props.plan_service.clone(),
            props.dispatch.clone(),
            props.enrichment_service.clone(),
            show_generator,
        )
    } else if let Some(dialect) = props.active_branch_dialect {
        render_create_form(
            dialect,
            plan_to_edit,
            props.dispatch.clone(),
            props.enrichment_service.clone(),
            show_create_plan,
            editing_plan_id,
        )
    } else {
        html! {}
    }
}

fn render_list_mode(ctx: &PlanContext<'_>) -> Html {
    html! {
        <>
            {render_plan_list(ctx.filtered_plans.clone(), ctx.props.user.active_plan_id, ctx.props.dispatch.clone(), ctx.editing_plan_id.clone(), ctx.on_export_plan.clone())}
            if ctx.props.active_branch_dialect.is_some() {
                {render_action_buttons(ctx.show_create_plan.clone(), ctx.show_generator.clone(), ctx.editing_plan_id.clone(), ctx.on_import_click.clone(), ctx.on_file_change.clone(), ctx.file_input_ref.clone(), ctx.is_importing)}
            }
        </>
    }
}
