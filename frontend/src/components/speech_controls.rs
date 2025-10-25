use crate::services::speech::SpeechRecognitionService;
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SpeechControlsProps {
    pub on_speech: Callback<String>,
    pub language_code: String,
    pub teaching_mode: String,
    pub formality: String,
    pub tts_enabled: bool,
    #[prop_or_default]
    pub on_dialect_cycle: Option<Callback<()>>,
    #[prop_or_default]
    pub on_teaching_mode_cycle: Option<Callback<()>>,
    #[prop_or_default]
    pub on_formality_cycle: Option<Callback<()>>,
    #[prop_or_default]
    pub on_tts_toggle: Option<Callback<()>>,
}

#[function_component(SpeechControls)]
pub fn speech_controls(props: &SpeechControlsProps) -> Html {
    let is_listening = use_state(|| false);
    let stt_service = use_state(
        || match SpeechRecognitionService::new(&props.language_code) {
            Ok(service) => Some(Rc::new(RefCell::new(service))),
            Err(e) => {
                error!("Failed to initialize STT service: {}", e);
                None
            }
        },
    );

    let toggle_listening = {
        let is_listening = is_listening.clone();
        let stt_service = stt_service.clone();
        let on_speech = props.on_speech.clone();

        Callback::from(move |_| {
            let new_state = !*is_listening;
            is_listening.set(new_state);

            if let Some(stt) = stt_service.as_ref() {
                if new_state {
                    // Start listening
                    let on_speech_clone = on_speech.clone();
                    if let Err(e) = stt.borrow_mut().start_listening(
                        Callback::from(move |transcript: String| {
                            info!("Final transcript: {}", transcript);
                            on_speech_clone.emit(transcript);
                        }),
                        None, // No interim results for now
                        Some(Callback::from(|err: String| {
                            error!("STT error: {}", err);
                        })),
                        Some(Callback::from({
                            let is_listening = is_listening.clone();
                            move |_| {
                                is_listening.set(false);
                            }
                        })),
                    ) {
                        error!("Failed to start listening: {}", e);
                        is_listening.set(false);
                    }
                } else {
                    // Stop listening
                    stt.borrow_mut().stop_listening();
                }
            }
        })
    };

    html! {
        <div class="speech-controls">
            <div class="button-group">
                <button onclick={toggle_listening} class={if *is_listening { "listening" } else { "" }}>
                    {if *is_listening { "🎤 Listening..." } else { "🎤 Speak" }}
                </button>
                <button
                    onclick={{
                        let on_tts_toggle = props.on_tts_toggle.clone();
                        Callback::from(move |_| {
                            if let Some(callback) = &on_tts_toggle {
                                callback.emit(());
                            }
                        })
                    }}
                    class={if props.tts_enabled { "tts-enabled" } else { "tts-disabled" }}
                    title={if props.tts_enabled { "Auto-play ON" } else { "Auto-play OFF" }}>
                    {if props.tts_enabled { "🔊" } else { "🔇" }}
                </button>
            </div>
            <div class="control-indicators">
                <button class="language-indicator clickable"
                        onclick={{
                            let on_dialect_cycle = props.on_dialect_cycle.clone();
                            Callback::from(move |_| {
                                if let Some(callback) = &on_dialect_cycle {
                                    callback.emit(());
                                }
                            })
                        }}
                        title="Click to cycle through dialects">
                    {&props.language_code}
                </button>
                <button class="mode-indicator clickable"
                        onclick={{
                            let on_teaching_mode_cycle = props.on_teaching_mode_cycle.clone();
                            Callback::from(move |_| {
                                if let Some(callback) = &on_teaching_mode_cycle {
                                    callback.emit(());
                                }
                            })
                        }}
                        title="Click to cycle through teaching modes">
                    {&props.teaching_mode}
                </button>
                <button class="formality-indicator clickable"
                        onclick={{
                            let on_formality_cycle = props.on_formality_cycle.clone();
                            Callback::from(move |_| {
                                if let Some(callback) = &on_formality_cycle {
                                    callback.emit(());
                                }
                            })
                        }}
                        title="Click to cycle through formality levels">
                    {&props.formality}
                </button>
            </div>
        </div>
    }
}
