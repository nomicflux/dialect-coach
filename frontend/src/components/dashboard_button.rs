use crate::app::app_state::{AppState, UIState, UIStateAction};
use yew::prelude::*;

#[derive(Properties)]
pub struct DashboardButtonProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
}

impl PartialEq for DashboardButtonProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[function_component(DashboardButton)]
pub fn dashboard_button(props: &DashboardButtonProps) -> Html {
    let DashboardButtonProps {
        app_state,
        ui_state,
    } = props;

    if app_state.current_user.is_none() {
        return html! {};
    }

    let onclick = {
        let ui_state = ui_state.clone();
        Callback::from(move |_| ui_state.dispatch(UIStateAction::ToggleDrawer))
    };

    html! {
        <div class="dashboard-float-container">
            <button class="btn-dashboard-float" {onclick}>
                <span class="dashboard-icon">{"☰"}</span>
                <span>{"Dashboard"}</span>
            </button>
        </div>
    }
}
