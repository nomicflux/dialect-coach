use crate::services::speech::SpeechRecognitionService;
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SpeechControlsProps {
    pub on_speech: Callback<String>,
    pub language_code: String,
    #[prop_or_default]
    pub on_dialect_cycle: Option<Callback<()>>,
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
            <button onclick={toggle_listening} class={if *is_listening { "listening" } else { "" }}>
                {if *is_listening { "🎤 Listening..." } else { "🎤 Speak" }}
            </button>
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
        </div>
    }
}
