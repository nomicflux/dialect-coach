use dialect_coach_shared::{Dialect, LearningItem, LearningItemType};
use uuid::Uuid;
use yew::prelude::*;

fn get_learning_item_id(item: &LearningItem) -> Uuid {
    match &item.item {
        LearningItemType::Mistake(m) => m.id,
        LearningItemType::Explanation(e) => e.id,
        LearningItemType::Translation(t) => t.id,
        LearningItemType::Exploration(e) => e.id,
    }
}

#[derive(Properties, PartialEq)]
pub struct LearningPanelProps {
    pub items: Vec<LearningItem>,
    pub is_open: bool,
    pub is_collapsed: bool,
    pub on_close: Callback<()>,
    pub on_toggle: Callback<()>,
    pub on_delete: Callback<Uuid>,
    pub on_undo: Callback<()>,
    pub deleted_count: usize,
    pub active_branch_dialect: Option<Dialect>,
}

fn calculate_color_from_score(score: u8, item_type: &LearningItemType) -> String {
    let intensity = 0.5 + (score as f32 / 100.0) * 0.5;
    match item_type {
        LearningItemType::Mistake(_) => format!("rgba(255, 107, 107, {})", intensity), // coral
        LearningItemType::Explanation(_) => format!("rgba(78, 205, 196, {})", intensity), // teal
        LearningItemType::Translation(_) => format!("rgba(255, 230, 109, {})", intensity), // yellow
        LearningItemType::Exploration(_) => format!("rgba(81, 207, 102, {})", intensity), // green
    }
}

fn calculate_bar_width(score: u8) -> String {
    format!("{}%", score)
}

fn get_item_content(item: &LearningItem) -> String {
    match &item.item {
        LearningItemType::Mistake(m) => m.get_content().to_string(),
        LearningItemType::Explanation(e) => e.get_content().to_string(),
        LearningItemType::Translation(t) => t.get_content().to_string(),
        LearningItemType::Exploration(e) => e.get_content().to_string(),
    }
}

fn get_tooltip(item: &LearningItem) -> Option<String> {
    match &item.item {
        LearningItemType::Mistake(m) => Some(m.mistake_category.to_string()),
        LearningItemType::Explanation(e) => Some(e.explanation.clone()),
        LearningItemType::Translation(t) => {
            if let Some(context) = &t.context {
                Some(format!("Translation: {}\nContext: {}", t.translated_to, context))
            } else {
                Some(format!("Translation: {}", t.translated_to))
            }
        }
        LearningItemType::Exploration(e) => Some(e.instructions_for_use.clone()),
    }
}

fn get_item_class(item: &LearningItem) -> &'static str {
    match &item.item {
        LearningItemType::Mistake(_) => "learning-item mistake",
        LearningItemType::Explanation(_) => "learning-item explanation",
        LearningItemType::Translation(_) => "learning-item translation",
        LearningItemType::Exploration(_) => "learning-item exploration",
    }
}

fn count_by_type(items: &[LearningItem]) -> (usize, usize, usize, usize) {
    let mut mistakes = 0;
    let mut explanations = 0;
    let mut translations = 0;
    let mut explorations = 0;

    for item in items {
        match &item.item {
            LearningItemType::Mistake(_) => mistakes += 1,
            LearningItemType::Explanation(_) => explanations += 1,
            LearningItemType::Translation(_) => translations += 1,
            LearningItemType::Exploration(_) => explorations += 1,
        }
    }

    (mistakes, explanations, translations, explorations)
}

fn has_active_dialect(dialect: Option<Dialect>) -> bool {
    dialect.is_some()
}

fn create_type_change_callback(
    selected_type: UseStateHandle<Option<String>>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        let target = e.target();
        let input = web_sys::HtmlSelectElement::from(wasm_bindgen::JsValue::from(target));
        let value = input.value();
        if value == "Select type..." {
            selected_type.set(None);
        } else {
            selected_type.set(Some(value));
        }
    })
}

#[allow(dead_code)]
fn is_mistake_valid(mistake: &str, corr: &str, cat: &str) -> bool {
    !mistake.trim().is_empty() && !corr.trim().is_empty() && !cat.is_empty()
}

#[allow(dead_code)]
fn is_explanation_valid(phrase: &str, expl: &str) -> bool {
    !phrase.trim().is_empty() && !expl.trim().is_empty()
}

#[allow(dead_code)]
fn is_translation_valid(word: &str, trans: &str) -> bool {
    !word.trim().is_empty() && !trans.trim().is_empty()
}

#[allow(dead_code)]
fn is_exploration_valid(point: &str, instr: &str) -> bool {
    !point.trim().is_empty() && !instr.trim().is_empty()
}

struct FormFields {
    mistake: UseStateHandle<String>,
    correction: UseStateHandle<String>,
    category: UseStateHandle<String>,
    phrase: UseStateHandle<String>,
    explanation: UseStateHandle<String>,
    word: UseStateHandle<String>,
    translation: UseStateHandle<String>,
    context: UseStateHandle<String>,
    point: UseStateHandle<String>,
    instructions: UseStateHandle<String>,
}

fn render_type_selector(
    selected_type: UseStateHandle<Option<String>>,
) -> Html {
    let current_value = selected_type.as_ref().cloned().unwrap_or_default();
    html! {
        <select
            class="learning-item-type-selector"
            onchange={create_type_change_callback(selected_type)}
            value={current_value}
        >
            <option value="Select type...">{"Select type..."}</option>
            <option value="Mistake">{"Mistake"}</option>
            <option value="Explanation">{"Explanation"}</option>
            <option value="Translation">{"Translation"}</option>
            <option value="Exploration">{"Exploration"}</option>
        </select>
    }
}

fn create_input_change_callback(state: UseStateHandle<String>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        let target = e.target();
        let input = web_sys::HtmlInputElement::from(wasm_bindgen::JsValue::from(target));
        state.set(input.value());
    })
}

fn create_select_change_callback(state: UseStateHandle<String>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        let target = e.target();
        let select = web_sys::HtmlSelectElement::from(wasm_bindgen::JsValue::from(target));
        state.set(select.value());
    })
}

fn render_mistake_fields(
    mistake: UseStateHandle<String>,
    corr: UseStateHandle<String>,
    cat: UseStateHandle<String>,
) -> Html {
    html! {
        <div class="mistake-fields">
            <input type="text" placeholder="Incorrect *" value={(*mistake).clone()}
                onchange={create_input_change_callback(mistake)} />
            <input type="text" placeholder="Correct *" value={(*corr).clone()}
                onchange={create_input_change_callback(corr)} />
            <select value={(*cat).clone()} onchange={create_select_change_callback(cat)}>
                <option value="">{"Select category *"}</option>
                <option value="SpellingError">{"Spelling Error"}</option>
                <option value="VocabularyError">{"Vocabulary Error"}</option>
                <option value="GrammarError">{"Grammar Error"}</option>
                <option value="DialectUsageError">{"Dialect Usage Error"}</option>
                <option value="Other">{"Other"}</option>
            </select>
        </div>
    }
}

fn render_explanation_fields(
    phrase: UseStateHandle<String>,
    expl: UseStateHandle<String>,
) -> Html {
    html! {
        <div class="explanation-fields">
            <input type="text" placeholder="Phrase *" value={(*phrase).clone()}
                onchange={create_input_change_callback(phrase)} />
            <input type="text" placeholder="Explanation *" value={(*expl).clone()}
                onchange={create_input_change_callback(expl)} />
        </div>
    }
}

fn render_translation_fields(
    word: UseStateHandle<String>,
    trans: UseStateHandle<String>,
    ctx: UseStateHandle<String>,
) -> Html {
    html! {
        <div class="translation-fields">
            <input type="text" placeholder="English *" value={(*word).clone()}
                onchange={create_input_change_callback(word)} />
            <input type="text" placeholder="Translation *" value={(*trans).clone()}
                onchange={create_input_change_callback(trans)} />
            <input type="text" placeholder="Context (optional)" value={(*ctx).clone()}
                onchange={create_input_change_callback(ctx)} />
        </div>
    }
}

fn render_exploration_fields(
    point: UseStateHandle<String>,
    instr: UseStateHandle<String>,
) -> Html {
    html! {
        <div class="exploration-fields">
            <input type="text" placeholder="Item *" value={(*point).clone()}
                onchange={create_input_change_callback(point)} />
            <input type="text" placeholder="Instructions *" value={(*instr).clone()}
                onchange={create_input_change_callback(instr)} />
        </div>
    }
}

fn render_add_item_form(
    selected_type: &UseStateHandle<Option<String>>,
    branch_dialect: Option<Dialect>,
    fields: &FormFields,
) -> Html {
    if !has_active_dialect(branch_dialect) {
        return html! {
            <div class="add-learning-item-form">
                <p class="empty-message">
                    {"Send a message to start practicing before adding items"}
                </p>
            </div>
        };
    }
    let field_html = match selected_type.as_ref().map(|s| s.as_str()) {
        Some("Mistake") => render_mistake_fields(
            fields.mistake.clone(),
            fields.correction.clone(),
            fields.category.clone()
        ),
        Some("Explanation") => render_explanation_fields(
            fields.phrase.clone(),
            fields.explanation.clone()
        ),
        Some("Translation") => render_translation_fields(
            fields.word.clone(),
            fields.translation.clone(),
            fields.context.clone()
        ),
        Some("Exploration") => render_exploration_fields(
            fields.point.clone(),
            fields.instructions.clone()
        ),
        _ => html! {},
    };
    html! {
        <div class="add-learning-item-form">
            <h4 class="section-title">{"Add Learning Item"}</h4>
            {render_type_selector(selected_type.clone())}
            {field_html}
        </div>
    }
}

fn render_collapsed_type_indicator(
    type_name: &str,
    class: String,
    count: usize,
    items: &[LearningItem],
) -> Html {
    let items_of_type: Vec<String> = items
        .iter()
        .filter(|item| {
            matches!(
                (type_name, &item.item),
                ("mistake", LearningItemType::Mistake(_))
                    | ("explanation", LearningItemType::Explanation(_))
                    | ("translation", LearningItemType::Translation(_))
                    | ("exploration", LearningItemType::Exploration(_))
            )
        })
        .map(get_item_content)
        .collect();
    let tooltip_text = if items_of_type.is_empty() {
        format!("{} items", count)
    } else {
        items_of_type.join("\n")
    };

    // Use abbreviated labels that fit in the collapsed panel
    let label = match type_name {
        "mistake" => "MST",
        "explanation" => "EXP",
        "translation" => "TRN",
        "exploration" => "EXR", // Use EXR to distinguish from EXP (explanations)
        _ => "",
    };

    html! {
        <div
            class={format!("learning-type-count-box {}", class.clone())}
            title={tooltip_text.clone()}
        >
            <div class="learning-type-indicator-tooltip">
                {tooltip_text}
            </div>
            <span class="learning-type-label">{label}</span>
            <span class="learning-type-number">{count}</span>
        </div>
    }
}

fn render_collapsed_view(props: &LearningPanelProps) -> Html {
    let (mistakes, explanations, translations, explorations) = count_by_type(&props.items);
    let total = props.items.len();

    html! {
        <>
            <div class="learning-panel-header">
            </div>
            <button
                class="learning-panel-toggle-button"
                onclick={Callback::from({
                    let on_toggle = props.on_toggle.clone();
                    move |_| on_toggle.emit(())
                })}
                title="Expand learning panel"
            >
                {"◄"}
            </button>
            <div class="learning-type-indicators">
                {if mistakes > 0 {
                    html! {
                        {render_collapsed_type_indicator(
                            "mistake",
                            "learning-type-count-box--mistake".to_string(),
                            mistakes,
                            &props.items
                        )}
                    }
                } else {
                    html! {}
                }}
                {if explanations > 0 {
                    html! {
                        {render_collapsed_type_indicator(
                            "explanation",
                            "learning-type-count-box--explanation".to_string(),
                            explanations,
                            &props.items
                        )}
                    }
                } else {
                    html! {}
                }}
                {if translations > 0 {
                    html! {
                        {render_collapsed_type_indicator(
                            "translation",
                            "learning-type-count-box--translation".to_string(),
                            translations,
                            &props.items
                        )}
                    }
                } else {
                    html! {}
                }}
                {if explorations > 0 {
                    html! {
                        {render_collapsed_type_indicator(
                            "exploration",
                            "learning-type-count-box--exploration".to_string(),
                            explorations,
                            &props.items
                        )}
                    }
                } else {
                    html! {}
                }}
            </div>
            <div class="learning-total-count">
                <span class="learning-total-label">{"Total"}</span>
                <span class="learning-total-number">{total}</span>
            </div>
        </>
    }
}

fn render_learning_item(item: &LearningItem, on_delete: Callback<Uuid>) -> Html {
    let content = get_item_content(item);
    let tooltip = get_tooltip(item);
    let color = calculate_color_from_score(item.score, &item.item);
    let bar_width = calculate_bar_width(item.score);
    let item_class = get_item_class(item);
    let item_id = get_learning_item_id(item);

    html! {
        <li class={item_class}>
            <button
                class="delete-button"
                onclick={on_delete.reform(move |_| item_id)}
            >
                {"×"}
            </button>
            <div class="item-with-tooltip">
                <span class="item-content" style={format!("color: {}", color)}>
                    {content}
                </span>
                if let Some(tip) = tooltip {
                    <span class="tooltip-text">{tip}</span>
                }
            </div>
            <div class="progress-bar">
                <div class="progress-fill" style={format!("width: {}", bar_width)}></div>
            </div>
        </li>
    }
}

fn render_expanded_view(
    props: &LearningPanelProps,
    selected_type: &UseStateHandle<Option<String>>,
    fields: &FormFields,
) -> Html {
    let (accomplishments, still_learning): (Vec<_>, Vec<_>) =
        props.items.iter().partition(|item| item.score == 100);

    html! {
        <>
            <div class="learning-panel-header">
                <h3>{"Learning Progress"}</h3>
            </div>
            <button
                class="learning-panel-toggle-button"
                onclick={Callback::from({
                    let on_toggle = props.on_toggle.clone();
                    move |_| on_toggle.emit(())
                })}
                title="Collapse learning panel"
            >
                {"►"}
            </button>
            <div class="learning-panel-content">
                {render_add_item_form(selected_type, props.active_branch_dialect, fields)}
                if !accomplishments.is_empty() {
                    <div class="learning-section">
                        <h4 class="section-title">{"Accomplishments"}</h4>
                        <ul class="learning-items">
                            {for accomplishments.iter().map(|item| render_learning_item(item, props.on_delete.clone()))}
                        </ul>
                    </div>
                }

                if !still_learning.is_empty() {
                    <div class="learning-section">
                        <h4 class="section-title">{"Still Learning"}</h4>
                        <ul class="learning-items">
                            {for still_learning.iter().map(|item| render_learning_item(item, props.on_delete.clone()))}
                        </ul>
                    </div>
                }

                if accomplishments.is_empty() && still_learning.is_empty() {
                    <p class="empty-message">{"No learning items yet. Start chatting to build your learning progress!"}</p>
                }
            </div>

            if props.deleted_count > 0 {
                <div class="undo-notification">
                    <span>{"Item deleted"}</span>
                    <button class="undo-button" onclick={{
                        let on_undo = props.on_undo.clone();
                        Callback::from(move |_| on_undo.emit(()))
                    }}>
                        {"Undo"}
                    </button>
                </div>
            }
        </>
    }
}

#[function_component(LearningPanel)]
pub fn learning_panel(props: &LearningPanelProps) -> Html {
    let selected_type = use_state(|| None::<String>);
    let specific_mistake = use_state(String::new);
    let correction = use_state(String::new);
    let category = use_state(String::new);
    let new_phrase = use_state(String::new);
    let explanation_text = use_state(String::new);
    let translated_word = use_state(String::new);
    let translated_to = use_state(String::new);
    let context_text = use_state(String::new);
    let point_to_try = use_state(String::new);
    let instructions = use_state(String::new);
    let panel_class = if props.is_collapsed {
        "learning-panel learning-panel--collapsed"
    } else {
        "learning-panel"
    };

    let fields = FormFields {
        mistake: specific_mistake.clone(),
        correction: correction.clone(),
        category: category.clone(),
        phrase: new_phrase.clone(),
        explanation: explanation_text.clone(),
        word: translated_word.clone(),
        translation: translated_to.clone(),
        context: context_text.clone(),
        point: point_to_try.clone(),
        instructions: instructions.clone(),
    };

    html! {
        <div class={panel_class}>
            {if props.is_collapsed {
                render_collapsed_view(props)
            } else {
                render_expanded_view(props, &selected_type, &fields)
            }}
        </div>
    }
}
