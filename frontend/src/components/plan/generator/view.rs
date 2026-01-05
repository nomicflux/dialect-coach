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
    selected_dialect: &Dialect,
    on_dialect_change: Callback<Event>,
    on_language_change: Callback<Event>,
    disabled: bool,
) -> Html {
    let current_language = selected_dialect.language();
    let dialects = Dialect::for_language(current_language, false, false);

    html! {
        <>
            <div class="panel-field">
                <label>{"Target Language"}</label>
                 <select onchange={on_language_change} disabled={disabled} value={current_language.name()}>
                    {for dialect_coach_shared::models::Language::all().iter().map(|l| {
                        html! {
                            <option value={l.name()} selected={*l == current_language}>
                                {l.name()}
                            </option>
                        }
                    })}
                </select>
            </div>
            <div class="panel-field">
                <label>{"Target Dialect"}</label>
                 <select onchange={on_dialect_change} disabled={disabled} value={selected_dialect.id()}>
                    {for dialects.iter().map(|d| {
                        html! {
                            <option value={d.id()} selected={*d == *selected_dialect}>
                                {d.name()}
                            </option>
                        }
                    })}
                </select>
            </div>
        </>
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
