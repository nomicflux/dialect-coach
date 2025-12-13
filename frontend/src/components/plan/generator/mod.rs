use crate::services::PlanService;
use std::rc::Rc;
use dialect_coach_shared::models::plan::import::SimpleImportLanguagePlan;
use dialect_coach_shared::models::Dialect;
use web_sys::HtmlInputElement;
use yew::prelude::*;

mod view;
use view::{render_buttons, render_dialect_selector, render_file_tab, render_tabs, render_text_tab};

#[derive(Properties, PartialEq)]
pub struct PlanGeneratorProps {
    pub plan_service: Rc<PlanService>,
    pub on_plan_generated: Callback<SimpleImportLanguagePlan>,
    pub on_cancel: Callback<()>,
}

#[derive(Clone, PartialEq)]
pub enum Tab {
    Text,
    File,
}

#[function_component(PlanGenerator)]
pub fn plan_generator(props: &PlanGeneratorProps) -> Html {
    let active_tab = use_state(|| Tab::Text);
    let text_content = use_state(String::new);
    let file_content = use_state(|| None::<web_sys::File>);
    let selected_dialect = use_state(|| Dialect::SpanishMexican);
    let loading = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    let plan_service = props.plan_service.clone();

    let on_tab_click = create_tab_handler(active_tab.clone(), error_message.clone());
    let on_text_change = create_text_handler(text_content.clone());
    let on_file_change = create_file_handler(file_content.clone());
    let on_dialect_change = create_dialect_handler(selected_dialect.clone());

    let on_generate = create_generate_handler(GenerateHandlerContext {
        loading: loading.clone(),
        error_message: error_message.clone(),
        active_tab: active_tab.clone(),
        text_content: text_content.clone(),
        file_content: file_content.clone(),
        selected_dialect: selected_dialect.clone(),
        on_plan_generated: props.on_plan_generated.clone(),
        plan_service,
    });

    let is_loading = *loading;

    html! {
        <div class="plan-generator">
            <h4 class="section-title">{"AI Plan Generator"}</h4>
            
            {render_tabs(&active_tab, &on_tab_click, is_loading)}

            <div class="generator-content">
                if *active_tab == Tab::Text {
                    {render_text_tab(&text_content, on_text_change, is_loading)}
                } else {
                    {render_file_tab(on_file_change, is_loading)}
                }

                {render_dialect_selector(&selected_dialect, on_dialect_change, is_loading)}

                if let Some(msg) = (*error_message).as_ref() {
                    <div class="error-message">{msg}</div>
                }

                {render_buttons(is_loading, on_generate, props.on_cancel.clone())}
            </div>
        </div>
    }
}

fn create_tab_handler(
    active_tab: UseStateHandle<Tab>,
    error_message: UseStateHandle<Option<String>>,
) -> Callback<Tab> {
    Callback::from(move |tab: Tab| {
        active_tab.set(tab);
        error_message.set(None);
    })
}

fn create_text_handler(text_content: UseStateHandle<String>) -> Callback<InputEvent> {
    Callback::from(move |e: InputEvent| {
        let input: HtmlInputElement = e.target_unchecked_into();
        text_content.set(input.value());
    })
}

fn create_file_handler(file_content: UseStateHandle<Option<web_sys::File>>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        let input: HtmlInputElement = e.target_unchecked_into();
        if let Some(files) = input.files()
            && let Some(file) = files.get(0)
        {
            file_content.set(Some(file));
        }
    })
}

fn create_dialect_handler(selected_dialect: UseStateHandle<Dialect>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        let input: HtmlInputElement = e.target_unchecked_into();
        if let Some(d) = Dialect::from_id(&input.value()) {
            selected_dialect.set(d);
        }
    })
}

struct GenerateHandlerContext {
    loading: UseStateHandle<bool>,
    error_message: UseStateHandle<Option<String>>,
    active_tab: UseStateHandle<Tab>,
    text_content: UseStateHandle<String>,
    file_content: UseStateHandle<Option<web_sys::File>>,
    selected_dialect: UseStateHandle<Dialect>,
    on_plan_generated: Callback<SimpleImportLanguagePlan>,
    plan_service: Rc<PlanService>,
}

fn create_generate_handler(ctx: GenerateHandlerContext) -> Callback<MouseEvent> {
    Callback::from(move |e: MouseEvent| {
        e.prevent_default();
        ctx.loading.set(true);
        ctx.error_message.set(None);

        let service = ctx.plan_service.clone();
        let text = if *ctx.active_tab == Tab::Text {
            Some((*ctx.text_content).clone())
        } else {
            None
        };
        let file = if *ctx.active_tab == Tab::File {
            (*ctx.file_content).clone()
        } else {
            None
        };
        let dialect = *ctx.selected_dialect;
        let loading = ctx.loading.clone();
        let error_message = ctx.error_message.clone();
        let on_plan_generated = ctx.on_plan_generated.clone();

        if text.is_none() && file.is_none() {
            error_message.set(Some("Please provide text or upload a file.".to_string()));
            loading.set(false);
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            match service.generate_plan(text, file, dialect).await {
                Ok(plan) => {
                    loading.set(false);
                    on_plan_generated.emit(plan);
                }
                Err(err) => {
                    loading.set(false);
                    error_message.set(Some(err.to_string()));
                }
            }
        });
    })
}
