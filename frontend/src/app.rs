use yew::prelude::*;
use dialect_coach_shared::models::{Language, Dialect};

#[function_component(App)]
pub fn app() -> Html {
    let selected_language = use_state(|| Language::Spanish);
    let selected_dialect = use_state(|| Dialect::SpanishMexican);

    html! {
        <div class="app-container">
            <header class="app-header">
                <h1>{"Dialect Coach"}</h1>
                <p>{"Practice Spanish, Arabic, and French dialects with AI agents"}</p>
            </header>

            <main class="app-main">
                <div class="language-selection">
                    <label>{"Language: "}</label>
                    <select onchange={
                        let selected_language = selected_language.clone();
                        Callback::from(move |e: Event| {
                            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                let value = select.value();
                                let lang = match value.as_str() {
                                    "spanish" => Language::Spanish,
                                    "arabic" => Language::Arabic,
                                    "french" => Language::French,
                                    _ => Language::Spanish,
                                };
                                selected_language.set(lang);
                            }
                        })
                    }>
                        <option value="spanish" selected=true>{"Spanish"}</option>
                        <option value="arabic">{"Arabic"}</option>
                        <option value="french">{"French"}</option>
                    </select>
                </div>

                <div class="dialect-selection">
                    <label>{"Dialect: "}</label>
                    <p>{format!("Selected: {}", (*selected_dialect).name())}</p>
                </div>

                <div class="chat-placeholder">
                    <p>{"Chat interface coming soon..."}</p>
                    <p class="info">
                        {"Current selection: "}
                        <strong>{format!("{} ({})", (*selected_dialect).name(), (*selected_dialect).bcp47_tag())}</strong>
                    </p>
                </div>
            </main>
        </div>
    }
}
