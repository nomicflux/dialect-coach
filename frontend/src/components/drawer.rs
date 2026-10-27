use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DrawerProps {
    #[prop_or_default]
    pub trap_focus: bool,
    #[prop_or_default]
    pub suspend_focus: bool,
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

pub(super) fn dialog_keydown(
    panel: NodeRef,
    close: Callback<()>,
    trap: bool,
) -> Callback<KeyboardEvent> {
    Callback::from(move |event: KeyboardEvent| {
        if !trap {
            return;
        }
        if event.key() == "Escape" {
            event.prevent_default();
            close.emit(());
            return;
        }
        if event.key() != "Tab" {
            return;
        }
        let Some(panel) = panel.cast::<web_sys::HtmlElement>() else {
            return;
        };
        let Ok(nodes) = panel.query_selector_all("button:not([disabled]), textarea:not([disabled]), input:not([disabled]), [tabindex='0']") else { return; };
        let elements: Vec<_> = (0..nodes.length())
            .filter_map(|i| nodes.item(i)?.dyn_into::<web_sys::HtmlElement>().ok())
            .collect();
        let Some(active) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
        else {
            return;
        };
        if let (Some(first), Some(last)) = (elements.first(), elements.last()) {
            let target = if event.shift_key() && active == **first {
                Some(last)
            } else if !event.shift_key() && active == **last {
                Some(first)
            } else {
                None
            };
            if let Some(target) = target {
                event.prevent_default();
                let _ = target.focus();
            }
        }
    })
}

#[function_component(Drawer)]
pub fn drawer(props: &DrawerProps) -> Html {
    let panel = use_node_ref();
    {
        let panel = panel.clone();
        use_effect_with((props.is_open, props.trap_focus), move |(open, trap)| {
            if *open
                && *trap
                && let Some(panel) = panel.cast::<web_sys::HtmlElement>()
                && let Ok(Some(first)) = panel.query_selector("button, textarea, input")
                && let Ok(first) = first.dyn_into::<web_sys::HtmlElement>()
            {
                let _ = first.focus();
            }
        });
    }
    let onkeydown = dialog_keydown(
        panel.clone(),
        props.on_close.clone(),
        props.trap_focus && props.is_open && !props.suspend_focus,
    );
    let on_close = get_close_callback(props.on_close.clone());
    let on_panel = get_panel_callback();
    let open_class = if props.is_open { "open" } else { "" };

    html! {
        <>
            <div
                class={classes!("drawer-backdrop", open_class)}
                onclick={on_close.clone()}
            />
            <div
                ref={panel}
                role={props.trap_focus.then_some("dialog")}
                aria-modal={(props.trap_focus && !props.suspend_focus).then_some("true")}
                aria-label={props.title.clone()}
                {onkeydown}
                class={classes!("drawer-panel", open_class)}
                onclick={on_panel}
            >
                {render_header(&props.title, on_close)}
                <div class="drawer-content">
                    { for props.children.iter() }
                </div>
            </div>
        </>
    }
}
