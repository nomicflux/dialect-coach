use crate::app::app_state::{AppState, UIState, UIStateAction};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DashboardButtonProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
}

fn dashboard_click(ui_state: UseReducerHandle<UIState>) -> Callback<MouseEvent> {
    Callback::from(move |_| ui_state.dispatch(UIStateAction::ToggleDrawer))
}

#[function_component(DashboardButton)]
pub fn dashboard_button(props: &DashboardButtonProps) -> Html {
    if props.app_state.current_user.is_none() {
        return html! {};
    }
    let onclick = dashboard_click(props.ui_state.clone());
    html! {
        <div class="dashboard-float-container">
            <button class="btn-dashboard-float" aria-label="Dashboard" {onclick}>
                <span class="dashboard-icon">{"☰"}</span>
                <span class="dashboard-label">{"Dashboard"}</span>
            </button>
        </div>
    }
}
