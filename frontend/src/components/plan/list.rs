use dialect_coach_shared::models::LanguagePlan;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PlanListProps {
    pub plans: Vec<LanguagePlan>,
    pub active_plan_id: Option<uuid::Uuid>,
    pub on_select_plan: Callback<Option<uuid::Uuid>>,
    pub on_delete_plan: Callback<uuid::Uuid>,
    pub on_edit_plan: Callback<uuid::Uuid>,
    pub on_export_plan: Callback<uuid::Uuid>,
}

#[function_component(PlanList)]
pub fn plan_list(props: &PlanListProps) -> Html {
    html! {
        <div class="plan-list">
            <h4 class="section-title">{"Language Plans"}</h4>
            if props.plans.is_empty() {
                <p class="empty-message">{"No plans created yet."}</p>
            } else {
                <ul class="plans">
                    {for props.plans.iter().map(|plan| {
                        let is_active = props.active_plan_id == Some(plan.id);
                        let plan_id = plan.id;
                        let on_select = props.on_select_plan.clone();
                        let on_delete = props.on_delete_plan.clone();
                        let on_edit = props.on_edit_plan.clone();
                        let on_export = props.on_export_plan.clone();

                        html! {
                            <li class={classes!("plan-item", if is_active { "active" } else { "" })}>
                                <div class="plan-info" onclick={Callback::from(move |_| on_select.emit(Some(plan_id)))}>
                                    <div class="plan-title">{&plan.title}</div>
                                    <div class="plan-status">{format!("{:?}", plan.status)}</div>
                                </div>
                                <div class="plan-actions">
                                    <button class="edit-plan-btn" onclick={Callback::from(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        on_edit.emit(plan_id);
                                    })} title="Edit Plan">{"✎"}</button>
                                    <button class="export-plan-btn" onclick={Callback::from(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        on_export.emit(plan_id);
                                    })} title="Export Plan">{"⬇"}</button>
                                    <button class="delete-plan-btn" onclick={Callback::from(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        on_delete.emit(plan_id);
                                    })} title="Delete Plan">{"×"}</button>
                                </div>
                            </li>
                        }
                    })}
                </ul>
            }
        </div>
    }
}
