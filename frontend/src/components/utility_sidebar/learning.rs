use crate::app::app_state::{OptionalUserState, UserStateAction};
use crate::components::plan::{PlanList, PlanCreate, ActivePlan};
use crate::services::enrichment_service::EnrichmentService;
use dialect_coach_shared::{
    Dialect, EnrichRequest, Explained, Exploratory, LearningItem, LearningItemType, Mistake,
    MistakeCategory, PartialExplained, PartialExploratory, PartialLearningItem, PartialMistake,
    PartialTranslated, Translated,
};
use std::rc::Rc;
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
pub struct LearningProps {
    pub items: Vec<LearningItem>,
    pub on_delete: Callback<Uuid>,
    pub on_undo: Callback<()>,
    pub deleted_count: usize,
    pub active_branch_dialect: Option<Dialect>,
    pub user_state: UseReducerHandle<OptionalUserState>,
    pub enrichment_service: Rc<EnrichmentService>,
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

fn is_mistake_valid(mistake: &str, corr: &str, cat: &str) -> bool {
    !mistake.trim().is_empty() && !corr.trim().is_empty() && !cat.is_empty()
}

fn is_explanation_valid(phrase: &str, expl: &str) -> bool {
    !phrase.trim().is_empty() && !expl.trim().is_empty()
}

fn is_translation_valid(word: &str, trans: &str) -> bool {
    !word.trim().is_empty() && !trans.trim().is_empty()
}

fn is_exploration_valid(point: &str, instr: &str) -> bool {
    !point.trim().is_empty() && !instr.trim().is_empty()
}

fn is_mistake_partial(mistake: &str, corr: &str) -> bool {
    !mistake.trim().is_empty() && (corr.trim().is_empty())
}

fn is_translation_partial(word: &str, trans: &str) -> bool {
    let word_filled = !word.trim().is_empty();
    let trans_filled = !trans.trim().is_empty();
    (word_filled && !trans_filled) || (!word_filled && trans_filled)
}

fn is_explanation_partial(phrase: &str, expl: &str) -> bool {
    let phrase_filled = !phrase.trim().is_empty();
    let expl_filled = !expl.trim().is_empty();
    (phrase_filled || expl_filled) && !(phrase_filled && expl_filled)
}

fn is_exploration_partial(point: &str, instr: &str) -> bool {
    let point_filled = !point.trim().is_empty();
    let instr_filled = !instr.trim().is_empty();
    (point_filled || instr_filled) && !(point_filled && instr_filled)
}

fn create_mistake_from_form(mistake: &str, corr: &str, cat: &str) -> Mistake {
    let ctx = "user-provided context".to_string();
    let category = match cat {
        "SpellingError" => MistakeCategory::SpellingError { context: ctx },
        "VocabularyError" => MistakeCategory::VocabularyError { context: ctx },
        "GrammarError" => MistakeCategory::GrammarError { context: ctx },
        "DialectUsageError" => MistakeCategory::DialectUsageError { context: ctx },
        _ => MistakeCategory::Other { context: ctx },
    };
    Mistake::new(mistake.trim().to_string(), corr.trim().to_string(), category)
}

fn create_explanation_from_form(phrase: &str, expl: &str) -> Explained {
    Explained::new(phrase.trim().to_string(), expl.trim().to_string())
}

fn create_translation_from_form(word: &str, trans: &str, ctx: &str) -> Translated {
    let context = if ctx.trim().is_empty() {
        None
    } else {
        Some(ctx.trim().to_string())
    };
    Translated::new(word.trim().to_string(), trans.trim().to_string(), context)
}

fn create_exploration_from_form(point: &str, instr: &str) -> Exploratory {
    Exploratory::new(point.trim().to_string(), instr.trim().to_string())
}

fn opt_str(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.trim().to_string())
    }
}

fn create_partial_mistake(mistake: &str, corr: &str, cat: &str) -> PartialMistake {
    PartialMistake {
        specific_mistake: opt_str(mistake),
        correction: opt_str(corr),
        mistake_category: opt_str(cat),
    }
}

fn create_partial_explanation(phrase: &str, expl: &str) -> PartialExplained {
    PartialExplained {
        new_phrase: opt_str(phrase),
        explanation: opt_str(expl),
    }
}

fn create_partial_translation(word: &str, trans: &str, ctx: &str) -> PartialTranslated {
    PartialTranslated {
        translated_word: opt_str(word),
        translated_to: opt_str(trans),
        context: opt_str(ctx),
    }
}

fn create_partial_exploration(point: &str, instr: &str) -> PartialExploratory {
    PartialExploratory {
        point_to_try: opt_str(point),
        instructions_for_use: opt_str(instr),
    }
}

fn is_form_valid(selected_type: Option<&String>, fields: &FormFields) -> bool {
    match selected_type.map(|s| s.as_str()) {
        Some("Mistake") => is_mistake_valid(&fields.mistake, &fields.correction, &fields.category),
        Some("Explanation") => is_explanation_valid(&fields.phrase, &fields.explanation),
        Some("Translation") => is_translation_valid(&fields.word, &fields.translation),
        Some("Exploration") => is_exploration_valid(&fields.point, &fields.instructions),
        _ => false,
    }
}

fn is_form_partial(selected_type: Option<&String>, fields: &FormFields) -> bool {
    match selected_type.map(|s| s.as_str()) {
        Some("Mistake") => is_mistake_partial(&fields.mistake, &fields.correction),
        Some("Explanation") => is_explanation_partial(&fields.phrase, &fields.explanation),
        Some("Translation") => is_translation_partial(&fields.word, &fields.translation),
        Some("Exploration") => is_exploration_partial(&fields.point, &fields.instructions),
        _ => false,
    }
}

fn dispatch_learning_item(selected_type: Option<&String>, fields: &FormFields, user_state: &UseReducerHandle<OptionalUserState>) {
    match selected_type.map(|s| s.as_str()) {
        Some("Mistake") => {
            let item = create_mistake_from_form(&fields.mistake, &fields.correction, &fields.category);
            user_state.dispatch(UserStateAction::AddLearningItems(vec![item], vec![], vec![], vec![]));
        }
        Some("Explanation") => {
            let item = create_explanation_from_form(&fields.phrase, &fields.explanation);
            user_state.dispatch(UserStateAction::AddLearningItems(vec![], vec![item], vec![], vec![]));
        }
        Some("Translation") => {
            let item = create_translation_from_form(&fields.word, &fields.translation, &fields.context);
            user_state.dispatch(UserStateAction::AddLearningItems(vec![], vec![], vec![item], vec![]));
        }
        Some("Exploration") => {
            let item = create_exploration_from_form(&fields.point, &fields.instructions);
            user_state.dispatch(UserStateAction::AddLearningItems(vec![], vec![], vec![], vec![item]));
        }
        _ => {}
    }
}

struct ClearStates {
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

fn create_clear_fields_callback(states: ClearStates) -> Callback<()> {
    Callback::from(move |_| {
        states.mistake.set(String::new());
        states.correction.set(String::new());
        states.category.set(String::new());
        states.phrase.set(String::new());
        states.explanation.set(String::new());
        states.word.set(String::new());
        states.translation.set(String::new());
        states.context.set(String::new());
        states.point.set(String::new());
        states.instructions.set(String::new());
    })
}

fn create_clear_all_callback(
    selected_type: UseStateHandle<Option<String>>,
    clear_fields: Callback<()>,
) -> Callback<()> {
    Callback::from(move |_| {
        selected_type.set(None);
        clear_fields.emit(());
    })
}

#[derive(Clone)]
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

fn render_save_cancel_buttons(
    save_enabled: bool,
    enrich_enabled: bool,
    enriching: bool,
    on_save: Callback<()>,
    on_enrich: Callback<()>,
    on_cancel: Callback<()>,
) -> Html {
    html! {
        <div class="form-buttons">
            <button
                class="save-learning-item-button"
                disabled={!save_enabled}
                onclick={on_save.reform(|_| ())}
            >
                {"Add"}
            </button>
            <button
                class="save-learning-item-button"
                disabled={!enrich_enabled || enriching}
                onclick={on_enrich.reform(|_| ())}
            >
                {if enriching { "Enriching..." } else { "Enrich" }}
            </button>
            <button
                class="cancel-learning-item-button"
                onclick={on_cancel.reform(|_| ())}
            >
                {"Cancel"}
            </button>
        </div>
    }
}

struct FormRenderProps<'a> {
    form_expanded: &'a UseStateHandle<bool>,
    selected_type: &'a UseStateHandle<Option<String>>,
    branch_dialect: Option<Dialect>,
    fields: &'a FormFields,
    enriching: bool,
    on_save: Callback<()>,
    on_enrich: Callback<()>,
    on_cancel: Callback<()>,
}

fn render_add_item_form(p: FormRenderProps) -> Html {
    if !has_active_dialect(p.branch_dialect) {
        return html! {
            <div class="add-learning-item-form">
                <p class="empty-message">
                    {"Send a message to start practicing before adding items"}
                </p>
            </div>
        };
    }

    if !**p.form_expanded {
        return html! {
            <div class="add-learning-item-form">
                <button
                    class="save-learning-item-button"
                    onclick={{
                        let form_expanded = p.form_expanded.clone();
                        Callback::from(move |_| form_expanded.set(true))
                    }}
                >
                    {"+ Add Learning Item"}
                </button>
            </div>
        };
    }
    let field_html = match p.selected_type.as_ref().map(|s| s.as_str()) {
        Some("Mistake") => render_mistake_fields(
            p.fields.mistake.clone(),
            p.fields.correction.clone(),
            p.fields.category.clone()
        ),
        Some("Explanation") => render_explanation_fields(
            p.fields.phrase.clone(),
            p.fields.explanation.clone()
        ),
        Some("Translation") => render_translation_fields(
            p.fields.word.clone(),
            p.fields.translation.clone(),
            p.fields.context.clone()
        ),
        Some("Exploration") => render_exploration_fields(
            p.fields.point.clone(),
            p.fields.instructions.clone()
        ),
        _ => html! {},
    };
    let save_enabled = is_form_valid(p.selected_type.as_ref(), p.fields);
    let enrich_enabled = is_form_partial(p.selected_type.as_ref(), p.fields);
    html! {
        <div class="add-learning-item-form">
            <div class="form-buttons">
                <h4 class="section-title">{"Add Learning Item"}</h4>
                <button
                    class="cancel-learning-item-button"
                    onclick={{
                        let form_expanded = p.form_expanded.clone();
                        Callback::from(move |_| form_expanded.set(false))
                    }}
                >
                    {"−"}
                </button>
            </div>
            {render_type_selector(p.selected_type.clone())}
            {field_html}
            {render_save_cancel_buttons(save_enabled, enrich_enabled, p.enriching, p.on_save, p.on_enrich, p.on_cancel)}
        </div>
    }
}

fn set_field_if_present(obj: &serde_json::Map<String, serde_json::Value>, key: &str, state: &UseStateHandle<String>) {
    if let Some(val) = obj.get(key)
        && let Some(s) = val.as_str() {
            state.set(s.to_string());
        }
}

fn get_category_name(cat_obj: &serde_json::Map<String, serde_json::Value>) -> &'static str {
    if cat_obj.contains_key("spelling_error") {
        "SpellingError"
    } else if cat_obj.contains_key("vocabulary_error") {
        "VocabularyError"
    } else if cat_obj.contains_key("grammar_error") {
        "GrammarError"
    } else if cat_obj.contains_key("dialect_usage_error") {
        "DialectUsageError"
    } else {
        "Other"
    }
}

#[derive(Clone)]
struct FieldStates {
    specific_mistake: UseStateHandle<String>,
    correction: UseStateHandle<String>,
    category: UseStateHandle<String>,
    new_phrase: UseStateHandle<String>,
    explanation_text: UseStateHandle<String>,
    translated_word: UseStateHandle<String>,
    translated_to: UseStateHandle<String>,
    point_to_try: UseStateHandle<String>,
    instructions: UseStateHandle<String>,
}

fn populate_fields(enriched_item: serde_json::Value, states: &FieldStates) {
    if let Some(obj) = enriched_item.as_object() {
        set_field_if_present(obj, "specific_mistake", &states.specific_mistake);
        set_field_if_present(obj, "correction", &states.correction);
        set_field_if_present(obj, "new_phrase", &states.new_phrase);
        set_field_if_present(obj, "explanation", &states.explanation_text);
        set_field_if_present(obj, "translated_word", &states.translated_word);
        set_field_if_present(obj, "translated_to", &states.translated_to);
        set_field_if_present(obj, "point_to_try", &states.point_to_try);
        set_field_if_present(obj, "instructions_for_use", &states.instructions);

        if let Some(cat_val) = obj.get("mistake_category")
            && let Some(cat_obj) = cat_val.as_object() {
                states.category.set(get_category_name(cat_obj).to_string());
            }
    }
}

fn get_item_parts(item: &LearningItem) -> (String, String, &'static str) {
    match &item.item {
        LearningItemType::Mistake(m) => (
            m.correction.clone(), 
            format!("Instead of: {}", m.specific_mistake),
            "🛠️"
        ),
        LearningItemType::Explanation(e) => (
            e.new_phrase.clone(),
            e.explanation.clone(),
            "💡"
        ),
        LearningItemType::Translation(t) => (
            t.translated_to.clone(),
            t.translated_word.clone(),
            "🌐"
        ),
        LearningItemType::Exploration(e) => (
            e.point_to_try.clone(),
            e.instructions_for_use.clone(),
            "🎯"
        ),
    }
}

fn get_accent_color(item: &LearningItemType) -> &'static str {
    match item {
        LearningItemType::Mistake(_) => "var(--coral)",
        LearningItemType::Explanation(_) => "var(--teal)",
        LearningItemType::Translation(_) => "var(--yellow)",
        LearningItemType::Exploration(_) => "var(--green)",
    }
}

fn render_learning_item(item: &LearningItem, on_delete: Callback<Uuid>) -> Html {
    let (title, subtitle, icon) = get_item_parts(item);
    let tooltip = get_tooltip(item);
    let accent_color = get_accent_color(&item.item);
    let item_id = get_learning_item_id(item);
    let score_pct = item.score;

    // Subtle glass card style
    html! {
        <li class="learning-card" style={format!("--accent-color: {}", accent_color)} title={tooltip}>
            <div class="card-icon">{icon}</div>
            <div class="card-content">
                <div class="card-title">{title}</div>
                <div class="card-subtitle">{subtitle}</div>
            </div>
            <div class="card-meta">
                <div class="score-ring" style={format!("--score: {}%", score_pct)}>
                     // Visual ring or text handled by CSS/SVG, or just simple text for now
                    <span class="score-text">{format!("{}%", score_pct)}</span>
                </div>
                <button
                    class="card-delete-button"
                    onclick={on_delete.reform(move |_| item_id)}
                >
                    {"×"}
                </button>
            </div>
            <div class="card-progress-line">
                <div class="progress-fill" style={format!("width: {}%; background: {}", score_pct, accent_color)}></div>
            </div>
        </li>
    }
}

#[function_component(Learning)]
pub fn learning(props: &LearningProps) -> Html {
    let form_expanded = use_state(|| false);
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
    let enriching = use_state(|| false);
    let enrich_error = use_state(|| None::<String>);
    let show_create_plan = use_state(|| false);
    let editing_plan_id = use_state(|| None::<Uuid>);

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

    let plan_to_edit = props.user_state.state.as_ref()
        .and_then(|state| editing_plan_id.as_ref().and_then(|id| 
            state.language_plans.iter().find(|p| p.id == *id).cloned()
        ));

    let clear_fields = create_clear_fields_callback(ClearStates {
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
    });

    let clear_all = create_clear_all_callback(selected_type.clone(), clear_fields.clone());

    let on_save = {
        let selected_type = selected_type.clone();
        let fields = fields.clone();
        let user_state = props.user_state.clone();
        let clear_fields = clear_fields.clone();
        Callback::from(move |_| {
            if !is_form_valid(selected_type.as_ref(), &fields) {
                return;
            }
            dispatch_learning_item(selected_type.as_ref(), &fields, &user_state);
            clear_fields.emit(());
        })
    };

    let on_enrich = {
        let enrichment_service = props.enrichment_service.clone();
        let dialect = props.active_branch_dialect;
        let selected_type = selected_type.clone();
        let fields = fields.clone();
        let enriching = enriching.clone();
        let enrich_error = enrich_error.clone();

        let field_states = FieldStates {
            specific_mistake: specific_mistake.clone(),
            correction: correction.clone(),
            category: category.clone(),
            new_phrase: new_phrase.clone(),
            explanation_text: explanation_text.clone(),
            translated_word: translated_word.clone(),
            translated_to: translated_to.clone(),
            point_to_try: point_to_try.clone(),
            instructions: instructions.clone(),
        };

        Callback::from(move |_| {
            let Some(dialect) = dialect else { return; };
            if !is_form_partial(selected_type.as_ref(), &fields) {
                return;
            }

            enriching.set(true);
            enrich_error.set(None);

            let partial_data = match selected_type.as_ref().map(|s| s.as_str()) {
                Some("Mistake") => {
                    let partial = create_partial_mistake(&fields.mistake, &fields.correction, &fields.category);
                    PartialLearningItem::Mistake(partial)
                }
                Some("Explanation") => {
                    let partial = create_partial_explanation(&fields.phrase, &fields.explanation);
                    PartialLearningItem::Explained(partial)
                }
                Some("Translation") => {
                    let partial = create_partial_translation(&fields.word, &fields.translation, &fields.context);
                    PartialLearningItem::Translated(partial)
                }
                Some("Exploration") => {
                    let partial = create_partial_exploration(&fields.point, &fields.instructions);
                    PartialLearningItem::Exploratory(partial)
                }
                _ => return,
            };

            let request = EnrichRequest {
                dialect,
                partial_data,
            };

            let enrichment_service = enrichment_service.clone();
            let enriching = enriching.clone();
            let enrich_error = enrich_error.clone();
            let field_states = field_states.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match enrichment_service.enrich_learning_item(request).await {
                    Ok(response) => {
                        populate_fields(response.enriched_item, &field_states);
                        enriching.set(false);
                    }
                    Err(e) => {
                        enrich_error.set(Some(e.to_string()));
                        enriching.set(false);
                    }
                }
            });
        })
    };

    let on_cancel = clear_all;

    let (accomplishments, still_learning): (Vec<_>, Vec<_>) =
        props.items.iter().partition(|item| item.score == 100);

    html! {
        <div class="learning-panel-content">
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

            if let Some(user_state_val) = &props.user_state.state {
                <div class="plans-section">
                    if let Some(active_plan_id) = user_state_val.active_plan_id
                        && let Some(active_plan) = user_state_val.language_plans.iter().find(|p| p.id == active_plan_id)
                    {
                        <ActivePlan 
                            plan={active_plan.clone()}
                            on_advance={
                                let user_state = props.user_state.clone();
                                Callback::from(move |id| {
                                    user_state.dispatch(UserStateAction::AdvancePlanStep(id));
                                })
                            }
                        />
                        <button 
                            class="view-all-plans-btn"
                            onclick={
                                let user_state = props.user_state.clone();
                                Callback::from(move |_| {
                                    user_state.dispatch(UserStateAction::SetActivePlan(None));
                                })
                            }
                        >
                            {"← Back to All Plans"}
                        </button>
                    } else if *show_create_plan || editing_plan_id.is_some() {
                        if let Some(dialect) = props.active_branch_dialect {
                            <PlanCreate 
                                dialect={dialect}
                                plan_to_edit={plan_to_edit}
                                on_create={
                                    let user_state = props.user_state.clone();
                                    let show_create_plan = show_create_plan.clone();
                                    let editing_plan_id = editing_plan_id.clone();
                                    Callback::from(move |plan: dialect_coach_shared::models::LanguagePlan| {
                                        if editing_plan_id.is_some() {
                                            user_state.dispatch(UserStateAction::UpdateLanguagePlan(plan));
                                        } else {
                                            user_state.dispatch(UserStateAction::AddLanguagePlan(plan));
                                        }
                                        show_create_plan.set(false);
                                        editing_plan_id.set(None);
                                    })
                                }
                                on_cancel={
                                    let show_create_plan = show_create_plan.clone();
                                    let editing_plan_id = editing_plan_id.clone();
                                    Callback::from(move |_| {
                                        show_create_plan.set(false);
                                        editing_plan_id.set(None);
                                    })
                                }
                            />
                        }
                    } else {
                        <PlanList 
                            plans={user_state_val.language_plans.clone()}
                            active_plan_id={user_state_val.active_plan_id}
                            on_select_plan={
                                let user_state = props.user_state.clone();
                                Callback::from(move |id| {
                                    user_state.dispatch(UserStateAction::SetActivePlan(id));
                                })
                            }
                            on_delete_plan={
                                let user_state = props.user_state.clone();
                                Callback::from(move |id| {
                                    user_state.dispatch(UserStateAction::DeleteLanguagePlan(id));
                                })
                            }
                            on_edit_plan={
                                let editing_plan_id = editing_plan_id.clone();
                                Callback::from(move |id| {
                                    editing_plan_id.set(Some(id));
                                })
                            }
                        />
                        if has_active_dialect(props.active_branch_dialect) {
                            <button 
                                class="create-plan-button"
                                onclick={
                                    let show_create_plan = show_create_plan.clone();
                                    let editing_plan_id = editing_plan_id.clone();
                                    Callback::from(move |_| {
                                        editing_plan_id.set(None);
                                        show_create_plan.set(true)
                                    })
                                }
                            >
                                {"+ Create New Plan"}
                            </button>
                        }
                    }
                </div>
                <hr class="learning-divider" />
            }

            {render_add_item_form(FormRenderProps {
                form_expanded: &form_expanded,
                selected_type: &selected_type,
                branch_dialect: props.active_branch_dialect,
                fields: &fields,
                enriching: *enriching,
                on_save,
                on_enrich,
                on_cancel,
            })}

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
        </div>
    }
}
