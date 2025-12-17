use super::Tab;
use dialect_coach_shared::models::Dialect;
use yew::prelude::*;

pub fn render_tabs(active_tab: &Tab, on_tab_click: &Callback<Tab>, disabled: bool) -> Html {
    html! {
        <div class="tabs">
            <button
                class={classes!("tab-button", if *active_tab == Tab::Text { "active" } else { "" })}
                onclick={on_tab_click.reform(|_| Tab::Text)}
                disabled={disabled}
            >{"Paste Text"}</button>
            <button
                class={classes!("tab-button", if *active_tab == Tab::File { "active" } else { "" })}
                onclick={on_tab_click.reform(|_| Tab::File)}
                disabled={disabled}
            >{"Upload File"}</button>
        </div>
    }
}

pub fn render_text_tab(text: &str, on_change: Callback<InputEvent>, disabled: bool) -> Html {
    html! {
        <div class="panel-field">
            <label>{"Paste Content (Article, Blog, Story)"}</label>
            <textarea
                class="generator-textarea"
                placeholder="Paste the text you want to learn from here..."
                value={text.to_string()}
                oninput={on_change}
                disabled={disabled}
            />
            <div class="field-help">{"Max ~20,000 characters. Identifying grammar and vocab."}</div>
        </div>
    }
}

pub fn render_file_tab(on_change: Callback<Event>, disabled: bool) -> Html {
    html! {
        <div class="panel-field">
            <label>{"Upload Document (PDF, HTML, TXT)"}</label>
            <input
                type="file"
                accept=".txt,.html,.pdf"
                onchange={on_change}
                disabled={disabled}
            />
             <div class="field-help">{"PDF support enabled. Large files will be truncated."}</div>
        </div>
    }
}

pub fn render_dialect_selector(
    selected: &Dialect,
    on_change: Callback<Event>,
    disabled: bool,
) -> Html {
    html! {
        <div class="panel-field">
            <label>{"Target Dialect"}</label>
             <select onchange={on_change} disabled={disabled} value={selected.id()}>
                {for Dialect::all().iter().map(|d| {
                    html! {
                        <option value={d.id()} selected={*selected == *d}>
                            {d.name()}
                        </option>
                    }
                })}
            </select>
        </div>
    }
}

pub fn render_buttons(
    is_loading: bool,
    on_generate: Callback<MouseEvent>,
    on_cancel: Callback<()>,
) -> Html {
    html! {
        <div class="form-buttons">
            <button
                class={classes!("save-button", if is_loading { "loading" } else { "" })}
                onclick={on_generate}
                disabled={is_loading}
            >
                {if is_loading { "Generating..." } else { "Generate Plan" }}
            </button>
            <button
                class="cancel-button"
                onclick={on_cancel.reform(|_| ())}
                disabled={is_loading}
            >
                {"Cancel"}
            </button>
        </div>
    }
}
