use crate::app::app_callbacks::{
    on_auto_start, on_continue_branch, on_explain_message, on_send_message,
};
use crate::app::app_helpers::render_message_undo_notification;
use crate::app::app_state::callbacks::on_replay_message;
use crate::app::app_state::user::{BranchAction, UserDomainAction};
use crate::app::app_state::{
    AppState, LearningAction, SessionAction, SessionState, SettingsAction, UIState, UIStateAction,
    UserStateGamificationExt,
};
use crate::app::user_state_callbacks::{
    on_create_branch, on_delete_message_callback, on_undo_message_callback,
};
use crate::components::study_drawer_content::DrawerTab;
use crate::components::{
    ChatWindow, Drawer, DynamicIsland, InputBox, LearningItemsFlash, SelectionModal,
    SelectionResult, StudyDrawerContent,
};
use crate::keyboard_shortcuts::{ShortcutAction, default_shortcuts, matches_binding};
use crate::services::websocket::ConnectionState;
use crate::utils::perf::PerfGuard;
use dialect_coach_shared::models::Translated;

use gloo::events::EventListener;
use std::rc::Rc;
use std::sync::Arc;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum SelectionModalState {
    Loading {
        original_text: String,
    },
    Loaded {
        original_text: String,
        result: SelectionResult,
    },
}

#[derive(Properties, PartialEq)]
pub struct MainContentProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub session: UseReducerHandle<SessionState>,
}

#[function_component(MainContent)]
pub fn main_content(props: &MainContentProps) -> Html {
    let MainContentProps {
        app_state,
        ui_state,
        session,
    } = props;

    // Performance measurement - logs to console when PERF_LOG=true in localStorage
    let _render_guard = PerfGuard::new("MainContent::render");

    // Memoize the UserState Rc to avoid deep cloning on every render (e.g. when UI state changes).
    // Dependencies: session (if session changes, we likely have new user struture).
    let user_rc = use_memo(session.clone(), |session| {
        session.user.as_ref().map(|u| Rc::new(u.clone()))
    });

    let us = match user_rc.as_ref() {
        Some(u) => u.clone(),
        None => return html! {},
    };

    let current_step_title = us.active_branch_plan_id().and_then(|plan_id| {
        us.language_plans
            .iter()
            .find(|p| p.id == plan_id)
            .and_then(|plan| plan.steps.get(plan.current_step_index))
            .map(|step| step.title.clone())
    });

    let chat_input_ref = use_node_ref();
    let goal_input_ref = use_node_ref();

    let drawer_active_tab = use_state(|| DrawerTab::Branches);
    let modal_state = use_state(|| None::<SelectionModalState>);

    // Memoize filtered items to avoid iterating/cloning on every render
    let filtered_items = use_memo(us.clone(), |user| {
        let dialect = user.active_branch_dialect();
        let items = user
            .get_learning_items_for_dialect(&dialect)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        gloo::console::log!(
            "get_filtered_items count:",
            items.len(),
            "for dialect:",
            dialect.to_string()
        );
        items
    });

    // Memoize filtered goals
    let filtered_goals = use_memo(us.clone(), |user| {
        let dialect = user.active_branch_dialect();
        let goals = user
            .get_learning_goals_for_dialect(&dialect)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        Arc::new(goals)
    });

    // Memoize branches Rc to prevent new pointer on every render
    let branches_rc = use_memo(us.clone(), |user| user.branches.clone());

    // Memoize messages Rc to prevent new pointer on every render
    let messages_rc = use_memo(us.clone(), |user| user.conversation_history.clone());

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
                modal_state.set(Some(SelectionModalState::Loading {
                    original_text: context.clone(),
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
                                modal_state.set(Some(SelectionModalState::Loaded {
                                    original_text: context,
                                    result: SelectionResult::Translation(response.segmented_phrases),
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

    let dispatch_domain = use_callback(session.clone(), |action: UserDomainAction, session| {
        session.dispatch(SessionAction::Domain(action));
    });

    let on_add_goal = use_callback(dispatch_domain.clone(), |goal: String, dispatch| {
        if !goal.trim().is_empty() {
            dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(goal)));
        }
    });

    let on_delete_goal = use_callback(dispatch_domain.clone(), |index: usize, dispatch| {
        dispatch.emit(UserDomainAction::Learning(LearningAction::DeleteGoal(
            index,
        )));
    });

    let on_switch_branch = use_callback(dispatch_domain.clone(), |id: Uuid, dispatch| {
        dispatch.emit(UserDomainAction::Branch(BranchAction::Switch(id)));
    });

    let on_delete_branch = use_callback(dispatch_domain.clone(), |id: Uuid, dispatch| {
        dispatch.emit(UserDomainAction::Branch(BranchAction::Delete(id)));
    });

    let on_drawer_close = use_callback(ui_state.clone(), |_, ui_state| {
        ui_state.dispatch(UIStateAction::ToggleDrawer);
    });

    let on_tab_change = use_callback(
        drawer_active_tab.clone(),
        |tab: DrawerTab, drawer_active_tab| {
            drawer_active_tab.set(tab);
        },
    );

    let on_delete_learning_item = use_callback(
        (ui_state.clone(), us.clone(), dispatch_domain.clone()),
        |id: Uuid, (ui_state, us, dispatch)| {
            if let Some(item) = us.learning_items.iter().find(|i| i.id() == id) {
                dispatch.emit(UserDomainAction::Learning(LearningAction::DeleteItem(
                    item.id(),
                )));
                ui_state.dispatch(UIStateAction::PushDeletedLearningItem(item.clone()));
            }
        },
    );

    let on_dismiss_flash_item = use_callback(ui_state.clone(), |id: Uuid, ui_state| {
        ui_state.dispatch(UIStateAction::DismissFlashItem(id));
    });

    let on_undo_delete_learning_item = use_callback(
        (ui_state.clone(), session.clone()),
        |_, (ui_state, session)| {
            if let Some(item) = ui_state.deleted_learning_items.back() {
                session.dispatch(SessionAction::Domain(UserDomainAction::Learning(
                    LearningAction::UndoDeleteItem(item.clone()),
                )));
                ui_state.dispatch(UIStateAction::PopDeletedLearningItem);
            }
        },
    );

    {
        let ui_state = ui_state.clone();
        let app_state = app_state.clone();
        let session = session.clone();
        let chat_input_ref = chat_input_ref.clone();
        // let goal_input_ref = goal_input_ref.clone();
        use_effect_with((), move |_| {
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
                        user={us.clone()}
                        is_loading={app_state.is_loading}
                        on_replay_message={Some(on_replay_message(app_state.clone()))}
                        on_delete_message={Some(on_delete_message_callback(ui_state.clone(), us.clone(), dispatch_domain.clone()))}
                        on_create_branch={Some(on_create_branch(dispatch_domain.clone()))}
                        on_auto_start={Some(on_auto_start(app_state.clone(), session.clone()))}
                        on_continue_branch={Some(on_continue_branch(app_state.clone(), session.clone()))}
                        on_explain={Some(on_explain_message(app_state.clone(), session.clone(), ui_state.clone()))}
                        explain_loading={ui_state.explain_loading.clone()}
                        translate_loading={ui_state.translate_loading.clone()}
                        on_selection_translate={Some(on_selection_translate_click.clone())}
                    />

                    <DynamicIsland
                        items={filtered_items.clone()}
                        current_step_title={current_step_title.clone()}
                        learning_goals={(*filtered_goals).clone()}
                        quests={
                            let stats = us.gamification_stats();
                            let dialect = us.active_branch_dialect();
                            stats.quests.iter()
                                .filter(|q| q.dialect == dialect)
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

                <Drawer
                    is_open={ui_state.drawer_open}
                    on_close={on_drawer_close}
                    title="Study Tools"
                >
                    <StudyDrawerContent
                        active_tab={*drawer_active_tab}
                        on_tab_change={on_tab_change}
                        user={us.clone()}
                        is_admin={app_state.current_user.as_ref().map(|u| u.is_admin).unwrap_or(false)}
                        dispatch={dispatch_domain.clone()}
                        branches={(*branches_rc).clone()}
                        active_branch_id={us.active_branch_id}
                        messages={(*messages_rc).clone()}
                        learning_goals={(*filtered_goals).clone()}
                        on_add_goal={on_add_goal}
                        on_delete_goal={on_delete_goal}
                        on_switch_branch={Some(on_switch_branch)}
                        on_delete_branch={Some(on_delete_branch)}
                        goal_input_ref={Some(goal_input_ref.clone())}
                        learning_items={filtered_items.clone()}
                        active_branch_dialect={Some(us.active_branch_dialect())}
                        enrichment_service={app_state.enrichment_service.clone()}
                        plan_service={app_state.plan_service.clone()}
                        on_delete_learning_item={on_delete_learning_item}
                        on_undo_delete_learning_item={on_undo_delete_learning_item}
                        deleted_learning_items_count={ui_state.deleted_learning_items.len()}
                    />
                </Drawer>

            {render_modal(&modal_state, &on_close_modal, &on_save_phrase)}

                <LearningItemsFlash
                    items={ui_state.flashed_items.clone()}
                    on_dismiss={on_dismiss_flash_item}
                />
        </>
    }
}

fn render_modal(
    modal_state: &UseStateHandle<Option<SelectionModalState>>,
    on_close: &Callback<()>,
    on_save: &Callback<(String, String, String)>,
) -> Html {
    match modal_state.as_ref() {
        Some(SelectionModalState::Loading { original_text }) => html! {
            <SelectionModal
                original_text={original_text.clone()}
                result={None}
                on_close={on_close.clone()}
                on_save={on_save.clone()}
            />
        },
        Some(SelectionModalState::Loaded { original_text, result }) => html! {
            <SelectionModal
                original_text={original_text.clone()}
                result={Some(result.clone())}
                on_close={on_close.clone()}
                on_save={on_save.clone()}
            />
        },
        None => html! {},
    }
}
