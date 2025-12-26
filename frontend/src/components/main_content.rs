use crate::app::app_callbacks::{
    on_auto_start, on_continue_branch, on_explain_message, on_send_message,
};
use crate::app::app_helpers::render_message_undo_notification;
use crate::app::app_state::callbacks::on_replay_message;
use crate::app::app_state::user::UserDomainAction;
use crate::app::app_state::{
    AppState, LearningAction, SessionAction, SessionState, SettingsAction, UIState, UIStateAction,
    UserStateGamificationExt,
};
use crate::app::user_state_callbacks::{
    on_add_goal, on_create_branch, on_delete_branch, on_delete_goal,
    on_delete_learning_item_callback, on_delete_message_callback, on_switch_branch,
    on_undo_message_callback,
};
use crate::components::study_drawer_content::DrawerTab;
use crate::components::{
    ChatWindow, Drawer, DynamicIsland, InputBox, StudyDrawerContent, TranslationModal,
};
use crate::keyboard_shortcuts::{ShortcutAction, default_shortcuts, matches_binding};
use crate::services::websocket::ConnectionState;
use dialect_coach_shared::models::{
    LearningGoal, LearningItem, PhraseTranslation, Translated, UserState,
};
use gloo::events::EventListener;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum TranslationModalState {
    Loading {
        original_sentence: String,
    },
    Loaded {
        original_sentence: String,
        phrases: Vec<PhraseTranslation>,
    },
}

#[derive(Properties)]
pub struct MainContentProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub session: UseReducerHandle<SessionState>,
}

impl PartialEq for MainContentProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[function_component(MainContent)]
pub fn main_content(props: &MainContentProps) -> Html {
    let MainContentProps {
        app_state,
        ui_state,
        session,
    } = props;

    let us = match session.user.as_ref() {
        Some(s) => s,
        None => return html! {},
    };
    let user_rc = Rc::new(us.clone());
    let current_step_title = us
        .language_plans
        .iter()
        .find(|p| Some(p.id) == us.active_plan_id)
        .and_then(|plan| plan.steps.get(plan.current_step_index))
        .map(|step| step.title.clone());

    let chat_input_ref = use_node_ref();
    let goal_input_ref = use_node_ref();

    let drawer_active_tab = use_state(|| DrawerTab::Branches);
    let modal_state = use_state(|| None::<TranslationModalState>);

    let on_close_modal = {
        let modal_state = modal_state.clone();
        Callback::from(move |_| {
            modal_state.set(None);
        })
    };

    let on_save_phrase = {
        let session = session.clone();
        let modal_state = modal_state.clone();
        Callback::from(
            move |(target, english, context): (String, String, String)| {
                let translated = Translated::new(english, target, Some(context));
                session.dispatch(SessionAction::Domain(UserDomainAction::Learning(
                    LearningAction::AddItems(vec![], vec![], vec![(translated, 0)], vec![]),
                )));
                modal_state.set(None);
            },
        )
    };

    let on_selection_translate_click = {
        let translation_service = app_state.translation_service.clone();
        let session_handle = session.clone();
        let modal_state = modal_state.clone();
        let ui_dispatch = ui_state.clone();

        Callback::from(
            move |(message_id, selected_text, context): (Uuid, String, String)| {
                let translation_service = translation_service.clone();
                let session_handle = session_handle.clone();
                let modal_state = modal_state.clone();
                let ui_dispatch = ui_dispatch.clone();

                // Open modal immediately with loading state
                modal_state.set(Some(TranslationModalState::Loading {
                    original_sentence: context.clone(),
                }));

                ui_dispatch.dispatch(UIStateAction::SetTranslateLoading { message_id });

                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(user) = session_handle.user.as_ref() {
                        let dialect = user.current_dialect();
                        let formality = Some(user.formality);

                        match translation_service
                            .translate_phrase(&selected_text, context.clone(), dialect, formality)
                            .await
                        {
                            Ok(response) => {
                                modal_state.set(Some(TranslationModalState::Loaded {
                                    original_sentence: context,
                                    phrases: response.segmented_phrases,
                                }));
                            }
                            Err(e) => {
                                web_sys::console::error_1(
                                    &format!("Translation failed: {}", e).into(),
                                );
                                modal_state.set(None);
                            }
                        }
                    }

                    ui_dispatch.dispatch(UIStateAction::ClearTranslateLoading { message_id });
                });
            },
        )
    };

    let dispatch_domain = {
        let session = session.clone();
        Callback::from(move |action: UserDomainAction| {
            session.dispatch(SessionAction::Domain(action));
        })
    };

    {
        let ui_state = ui_state.clone();
        let app_state = app_state.clone();
        let session = session.clone();
        let chat_input_ref = chat_input_ref.clone();
        // let goal_input_ref = goal_input_ref.clone();
        use_effect_with(session.clone(), move |_| {
            let shortcuts = default_shortcuts();
            let window = web_sys::window().unwrap();
            let listener = EventListener::new(&window, "keydown", move |event| {
                let event = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
                for (action, binding) in &shortcuts {
                    if matches_binding(event, binding) {
                        event.prevent_default();
                        match action {
                            ShortcutAction::ToggleDrawer => {
                                ui_state.dispatch(UIStateAction::ToggleDrawer);
                            }
                            ShortcutAction::ToggleLearningPanel => {
                                ui_state.dispatch(UIStateAction::ToggleLearningPanel);
                            }
                            ShortcutAction::TogglePracticeSettings => {
                                ui_state.dispatch(if ui_state.panel_open {
                                    UIStateAction::ClosePanel
                                } else {
                                    UIStateAction::OpenPanel
                                });
                            }
                            ShortcutAction::ToggleAutoSpeak => {
                                session.dispatch(SessionAction::Domain(
                                    UserDomainAction::Settings(SettingsAction::ToggleTTS),
                                ));
                            }
                            ShortcutAction::ReplayLastMessage => {
                                if let Some(user) = session.user.as_ref()
                                    && let Some(msg) = user.get_active_branch_messages().last()
                                {
                                    on_replay_message(app_state.clone()).emit((*msg).clone());
                                }
                            }
                            ShortcutAction::CycleDialect => {
                                session.dispatch(SessionAction::Domain(
                                    UserDomainAction::Settings(SettingsAction::CycleDialect),
                                ));
                            }
                            ShortcutAction::CycleTeachingMode => {
                                let is_admin = app_state
                                    .current_user
                                    .as_ref()
                                    .map(|u| u.is_admin)
                                    .unwrap_or(false);
                                session.dispatch(SessionAction::Domain(
                                    UserDomainAction::Settings(SettingsAction::CycleTeachingMode(
                                        is_admin,
                                    )),
                                ));
                            }
                            ShortcutAction::CycleFormality => {
                                session.dispatch(SessionAction::Domain(
                                    UserDomainAction::Settings(SettingsAction::CycleFormality),
                                ));
                            }
                            ShortcutAction::FocusGoalInput => {
                                // Disabled in this phase
                                // if let Some(input) =
                                //     goal_input_ref.cast::<web_sys::HtmlInputElement>()
                                // {
                                //     let _ = input.focus();
                                // }
                            }
                            ShortcutAction::FocusChatInput => {
                                if let Some(textarea) =
                                    chat_input_ref.cast::<web_sys::HtmlTextAreaElement>()
                                {
                                    let _ = textarea.focus();
                                }
                            }
                        }
                        break;
                    }
                }
            });
            move || drop(listener)
        });
    }

    html! {
        <>
                // Chat Canvas: The immersive center
                <div class="chat-canvas">

                    <ChatWindow
                        user={user_rc.clone()}
                        is_loading={app_state.is_loading}
                        on_replay_message={Some(on_replay_message(app_state.clone()))}
                        on_delete_message={Some(on_delete_message_callback(ui_state.clone(), user_rc.clone(), dispatch_domain.clone()))}
                        on_create_branch={Some(on_create_branch(dispatch_domain.clone()))}
                        on_auto_start={Some(on_auto_start(app_state.clone(), session.clone()))}
                        on_continue_branch={Some(on_continue_branch(app_state.clone(), session.clone()))}
                        on_explain={Some(on_explain_message(app_state.clone(), session.clone(), ui_state.clone()))}
                        explain_loading={ui_state.explain_loading.clone()}
                        translate_loading={ui_state.translate_loading.clone()}
                        on_selection_translate={Some(on_selection_translate_click.clone())}
                    />

                    <DynamicIsland
                        items={
                            let items = get_filtered_items(us);
                            gloo::console::log!("DynamicIsland items:", items.len());
                            items
                        }
                        current_step_title={current_step_title.clone()}
                        learning_goals={get_filtered_goals(us)}
                        quests={
                            let stats = us.gamification_stats();
                            stats.quests.iter()
                                .filter(|q| q.dialect == us.selected_dialect)
                                .cloned()
                                .collect::<Vec<_>>()
                        }
                    />
                    <InputBox
                        on_send={{
                            let send_message = on_send_message(app_state.clone(), session.clone());
                            Callback::from(move |content: String| {
                                send_message.emit(content);
                            })
                        }}
                        disabled={!matches!(app_state.connection_state, ConnectionState::Connected)}
                        textarea_ref={Some(chat_input_ref.clone())}
                        language_option={us.current_language_option()}
                    />
                    {render_message_undo_notification(
                        ui_state.deleted_messages.len(),
                        on_undo_message_callback(ui_state.clone(), dispatch_domain.clone())
                    )}
                </div>

                // Study Drawer: Replaces sidebar
                <Drawer
                    is_open={ui_state.drawer_open}
                    on_close={{
                        let ui_state = ui_state.clone();
                        Callback::from(move |_| ui_state.dispatch(UIStateAction::ToggleDrawer))
                    }}
                    title="Study Tools"
                >
                    <StudyDrawerContent
                        active_tab={*drawer_active_tab}
                        on_tab_change={{
                            let drawer_active_tab = drawer_active_tab.clone();
                            Callback::from(move |tab| drawer_active_tab.set(tab))
                        }}
                        user={user_rc.clone()}
                        is_admin={app_state.current_user.as_ref().map(|u| u.is_admin).unwrap_or(false)}
                        dispatch={dispatch_domain.clone()}
                        ui_state={ui_state.clone()}
                        branches={us.branches.clone()}
                        active_branch_id={us.active_branch_id}
                        messages={us.conversation_history.clone()}
                        learning_goals={get_filtered_goals(us)}
                        on_add_goal={on_add_goal(user_rc.clone(), dispatch_domain.clone())}
                        on_delete_goal={on_delete_goal(dispatch_domain.clone())}
                        on_switch_branch={Some(on_switch_branch(dispatch_domain.clone()))}
                        on_delete_branch={Some(on_delete_branch(dispatch_domain.clone()))}
                        goal_input_ref={Some(goal_input_ref.clone())}
                        learning_items={get_filtered_items(us)}
                        active_branch_dialect={Some(us.selected_dialect)}
                        enrichment_service={app_state.enrichment_service.clone()}
                        plan_service={app_state.plan_service.clone()}
                        on_delete_learning_item={on_delete_learning_item_callback(ui_state.clone(), user_rc.clone(), dispatch_domain.clone())}
                        on_undo_delete_learning_item={{
                            let ui_state = ui_state.clone();
                            let session = session.clone();
                            let deleted_items = ui_state.deleted_learning_items.clone();
                            Callback::from(move |_| {
                                if let Some(item) = deleted_items.back() {
                                    session.dispatch(SessionAction::Domain(UserDomainAction::Learning(LearningAction::UndoDeleteItem(
                                        item.clone(),
                                    ))));
                                    ui_state.dispatch(UIStateAction::PopDeletedLearningItem);
                                }
                            })
                        }}
                        deleted_learning_items_count={ui_state.deleted_learning_items.len()}
                    />
                </Drawer>

            {render_modal(&modal_state, &on_close_modal, &on_save_phrase)}
        </>
    }
}

fn render_modal(
    modal_state: &UseStateHandle<Option<TranslationModalState>>,
    on_close: &Callback<()>,
    on_save: &Callback<(String, String, String)>,
) -> Html {
    match modal_state.as_ref() {
        Some(TranslationModalState::Loading { original_sentence }) => html! {
            <TranslationModal
                original_sentence={original_sentence.clone()}
                phrases={None}
                on_close={on_close.clone()}
                on_save_phrase={on_save.clone()}
            />
        },
        Some(TranslationModalState::Loaded {
            original_sentence,
            phrases,
        }) => html! {
            <TranslationModal
                original_sentence={original_sentence.clone()}
                phrases={Some(phrases.clone())}
                on_close={on_close.clone()}
                on_save_phrase={on_save.clone()}
            />
        },
        None => html! {},
    }
}

fn get_filtered_items(user_state: &UserState) -> Vec<LearningItem> {
    let items = user_state
        .get_learning_items_for_dialect(&user_state.selected_dialect)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    gloo::console::log!(
        "get_filtered_items count:",
        items.len(),
        "for dialect:",
        user_state.selected_dialect.to_string()
    );
    items
}

fn get_filtered_goals(user_state: &UserState) -> Vec<LearningGoal> {
    user_state
        .get_learning_goals_for_dialect(&user_state.selected_dialect)
        .into_iter()
        .cloned()
        .collect()
}
