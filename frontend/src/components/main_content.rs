use crate::app::app_callbacks::{on_prompt_click, on_send_message, on_tts_toggle};
use crate::app::app_helpers::render_message_undo_notification;
use crate::app::app_state::callbacks::on_replay_message;
use crate::app::app_state::{AppState, OptionalUserState, UIState, UIStateAction, UserStateAction};
use crate::app::user_state_callbacks::{
    on_add_goal, on_create_branch, on_delete_branch, on_delete_goal,
    on_delete_learning_item_callback, on_delete_message_callback, on_dialect_cycle,
    on_formality_cycle, on_switch_branch, on_teaching_mode_cycle, on_undo_message_callback,
};
use crate::components::{
    BranchSidebar, ChatWindow, InputBox, LearningPanel, SettingsPanel, SpeechControls, UsageFooter,
};
use crate::keyboard_shortcuts::{default_shortcuts, matches_binding, ShortcutAction};
use crate::services::websocket::ConnectionState;
use gloo::events::EventListener;
use gloo::utils::window;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlTextAreaElement, KeyboardEvent};
use yew::prelude::*;

#[derive(Properties)]
pub struct MainContentProps {
    pub app_state: UseReducerHandle<AppState>,
    pub ui_state: UseReducerHandle<UIState>,
    pub user_state: UseReducerHandle<OptionalUserState>,
}

impl PartialEq for MainContentProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

fn handle_shortcut_action(
    action: &ShortcutAction,
    ui_state: &UseReducerHandle<UIState>,
    app_state: &UseReducerHandle<AppState>,
    user_state: &UseReducerHandle<OptionalUserState>,
    chat_input_ref: &NodeRef,
    goal_input_ref: &NodeRef,
) {
    match action {
        ShortcutAction::ToggleSidebar => {
            ui_state.dispatch(UIStateAction::ToggleSidebar);
        }
        ShortcutAction::ToggleLearningPanel => {
            ui_state.dispatch(UIStateAction::ToggleLearningPanel);
        }
        ShortcutAction::ToggleUsageFooter => {
            ui_state.dispatch(UIStateAction::ToggleUsageFooter);
        }
        ShortcutAction::TogglePracticeSettings => {
            toggle_practice_settings(ui_state);
        }
        ShortcutAction::ToggleAutoSpeak => {
            on_tts_toggle(app_state.clone(), user_state.clone()).emit(());
        }
        ShortcutAction::ReplayLastMessage => {
            replay_last_message(app_state, user_state);
        }
        ShortcutAction::CycleDialect => {
            on_dialect_cycle(user_state.clone()).emit(());
        }
        ShortcutAction::CycleTeachingMode => {
            on_teaching_mode_cycle(user_state.clone()).emit(());
        }
        ShortcutAction::CycleFormality => {
            on_formality_cycle(user_state.clone()).emit(());
        }
        ShortcutAction::FocusGoalInput => {
            focus_goal_input(goal_input_ref);
        }
        ShortcutAction::FocusChatInput => {
            focus_chat_input(chat_input_ref);
        }
    }
}

fn toggle_practice_settings(ui_state: &UseReducerHandle<UIState>) {
    if ui_state.panel_open {
        ui_state.dispatch(UIStateAction::ClosePanel);
    } else {
        ui_state.dispatch(UIStateAction::OpenPanel);
    }
}

fn replay_last_message(
    app_state: &UseReducerHandle<AppState>,
    user_state: &UseReducerHandle<OptionalUserState>,
) {
    if let Some(state) = user_state.0.as_ref()
        && let Some(msg) = state.get_active_branch_messages().last()
    {
        on_replay_message(app_state.clone()).emit((*msg).clone());
    }
}

fn focus_goal_input(goal_input_ref: &NodeRef) {
    if let Some(input) = goal_input_ref.cast::<HtmlInputElement>() {
        let _ = input.focus();
    }
}

fn focus_chat_input(chat_input_ref: &NodeRef) {
    if let Some(textarea) = chat_input_ref.cast::<HtmlTextAreaElement>() {
        let _ = textarea.focus();
    }
}

fn is_typing_in_input(target: &Option<web_sys::EventTarget>) -> bool {
    target
        .as_ref()
        .and_then(|t| t.dyn_ref::<HtmlInputElement>())
        .is_some()
        || target
            .as_ref()
            .and_then(|t| t.dyn_ref::<HtmlTextAreaElement>())
            .is_some()
}

fn should_handle_shortcut(action: &ShortcutAction, is_input: bool) -> bool {
    match action {
        ShortcutAction::FocusGoalInput | ShortcutAction::FocusChatInput => true,
        _ => !is_input,
    }
}

#[function_component(MainContent)]
pub fn main_content(props: &MainContentProps) -> Html {
    let MainContentProps {
        app_state,
        ui_state,
        user_state,
    } = props;

    let us = match user_state.0.as_ref() {
        Some(s) => s,
        None => return html! {},
    };

    let chat_input_ref = use_node_ref();
    let goal_input_ref = use_node_ref();

    // Global keyboard shortcut handler
    {
        let ui_state = ui_state.clone();
        let app_state = app_state.clone();
        let user_state = user_state.clone();
        let chat_input_ref = chat_input_ref.clone();
        let goal_input_ref = goal_input_ref.clone();

        use_effect_with((), move |_| {
            let shortcuts = default_shortcuts();

            let listener = EventListener::new(&window(), "keydown", move |e| {
                if let Some(event) = e.dyn_ref::<KeyboardEvent>() {
                    let target = event.target();
                    let is_input = is_typing_in_input(&target);

                    for (action, binding) in &shortcuts {
                        if matches_binding(event, binding) {
                            let should_handle = should_handle_shortcut(action, is_input);

                            if should_handle {
                                event.prevent_default();
                                handle_shortcut_action(
                                    action,
                                    &ui_state,
                                    &app_state,
                                    &user_state,
                                    &chat_input_ref,
                                    &goal_input_ref,
                                );
                                break;
                            }
                        }
                    }
                }
            });
            move || drop(listener)
        });
    }

    html! {
        <>
            // Branch navigation sidebar - positioned off to the side
            <BranchSidebar
                branches={us.branches.clone()}
                active_branch_id={us.active_branch_id}
                messages={us.conversation_history.clone()}
                learning_goals={us.learning_goals.clone()}
                on_add_goal={on_add_goal(user_state.clone())}
                on_delete_goal={on_delete_goal(user_state.clone())}
                is_collapsed={ui_state.sidebar_collapsed}
                on_toggle={Callback::from({
                    let ui_state = ui_state.clone();
                    move |_| {
                        ui_state.dispatch(UIStateAction::ToggleSidebar);
                    }
                })}
                on_switch_branch={Some(on_switch_branch(user_state.clone()))}
                on_delete_branch={Some(on_delete_branch(user_state.clone()))}
                goal_input_ref={Some(goal_input_ref.clone())}
            />

            <div class="container">
                // Main chat card
                <div class="card card--chat" id="main-chat">
                    // Chat interface
                    <ChatWindow
                        user_state={us.clone()}
                        is_loading={app_state.is_loading}
                        on_replay_message={Some(on_replay_message(app_state.clone()))}
                        on_prompt_click={Some(on_prompt_click(app_state.clone(), user_state.clone(), ui_state.clone()))}
                        translating_button={(ui_state.translating_button).clone()}
                        on_delete_message={Some(on_delete_message_callback(ui_state.clone(), user_state.clone()))}
                        on_create_branch={Some(on_create_branch(user_state.clone()))}
                    />
                    <SpeechControls
                        on_speech={on_send_message(app_state.clone(), user_state.clone())}
                        dialect={us.selected_dialect}
                        teaching_mode={us.teaching_mode_display().to_string()}
                        formality={us.formality_display().to_string()}
                        tts_enabled={us.tts_enabled}
                        on_dialect_cycle={Some(on_dialect_cycle(user_state.clone()))}
                        on_teaching_mode_cycle={Some(on_teaching_mode_cycle(user_state.clone()))}
                        on_formality_cycle={Some(on_formality_cycle(user_state.clone()))}
                        on_tts_toggle={Some(on_tts_toggle(app_state.clone(), user_state.clone()))}
                    />
                    <InputBox
                        on_send={{
                            let ui_state = ui_state.clone();
                            let send_message = on_send_message(app_state.clone(), user_state.clone());
                            Callback::from(move |content: String| {
                                // Clear the prompt value after use
                                ui_state.dispatch(UIStateAction::ClearInputPrompt);
                                send_message.emit(content);
                            })
                        }}
                        disabled={!matches!(app_state.connection_state, ConnectionState::Connected)}
                        external_value={(ui_state.input_prompt_value).clone()}
                        textarea_ref={Some(chat_input_ref.clone())}
                    />
                    {render_message_undo_notification(
                        ui_state.deleted_messages.len(),
                        on_undo_message_callback(ui_state.clone(), user_state.clone())
                    )}
                </div>

                // Floating panel toggle button
                <button class="panel-toggle" onclick={{
                    let ui_state = ui_state.clone();
                    Callback::from(move |_| {
                        ui_state.dispatch(if ui_state.panel_open {
                            UIStateAction::ClosePanel
                        } else {
                            UIStateAction::OpenPanel
                        })
                    })
                }}>
                    <span>{"⚙️"}</span>
                    <span>{"Practice Settings"}</span>
                </button>

                // Learning panel
                <LearningPanel
                    items={us.learning_items.clone()}
                    is_open={true}
                    is_collapsed={ui_state.learning_panel_collapsed}
                    on_close={{
                        let ui_state = ui_state.clone();
                        Callback::from(move |_| {
                            ui_state.dispatch(UIStateAction::CloseLearningPanel);
                        })
                    }}
                    on_toggle={Callback::from({
                        let ui_state = ui_state.clone();
                        move |_| {
                            ui_state.dispatch(UIStateAction::ToggleLearningPanel);
                        }
                    })}
                    on_delete={on_delete_learning_item_callback(ui_state.clone(), user_state.clone())}
                    on_undo={{
                        let ui_state = ui_state.clone();
                        let user_state = user_state.clone();
                        let deleted_items = ui_state.deleted_learning_items.clone();
                        Callback::from(move |_| {
                            if let Some(item) = deleted_items.back() {
                                user_state.dispatch(UserStateAction::UndoDeleteLearningItem(item.clone()));
                                ui_state.dispatch(UIStateAction::PopDeletedLearningItem);
                            }
                        })
                    }}
                    deleted_count={ui_state.deleted_learning_items.len()}
                />
            </div>

            <SettingsPanel
                user_state={user_state.clone()}
                ui_state={ui_state.clone()}
            />

            <UsageFooter
                usage_stats={us.usage_stats.clone()}
                is_collapsed={ui_state.usage_footer_collapsed}
                on_toggle={Callback::from({
                    let ui_state = ui_state.clone();
                    move |_| {
                        ui_state.dispatch(UIStateAction::ToggleUsageFooter);
                    }
                })}
            />
        </>
    }
}
