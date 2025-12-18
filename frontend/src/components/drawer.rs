use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DrawerProps {
    pub is_open: bool,
    pub on_close: Callback<()>,
    pub title: String,
    pub children: Children,
}

fn get_close_callback(on_close: Callback<()>) -> Callback<MouseEvent> {
    Callback::from(move |_| on_close.emit(()))
}

fn get_panel_callback() -> Callback<MouseEvent> {
    Callback::from(|e: MouseEvent| e.stop_propagation())
}

fn render_header(title: &str, on_close: Callback<MouseEvent>) -> Html {
    html! {
        <div class="drawer-header">
            <h2 class="drawer-title">{title}</h2>
            <button class="drawer-close-button" onclick={on_close} title="Close">
                {"×"}
            </button>
        </div>
    }
}

#[function_component(Drawer)]
pub fn drawer(props: &DrawerProps) -> Html {
    let on_close = get_close_callback(props.on_close.clone());
    let on_panel = get_panel_callback();
    let open_class = if props.is_open { "open" } else { "" };

    html! {
        <div class={classes!("drawer-backdrop", open_class)} onclick={on_close.clone()}>
            <div class="drawer-panel" onclick={on_panel}>
                {render_header(&props.title, on_close)}
                <div class="drawer-content">
                    { for props.children.iter() }
                </div>
            </div>
        </div>
    }
}
