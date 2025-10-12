use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SpeechControlsProps {
    pub on_speech: Callback<String>,
    pub language_code: String,
}

#[function_component(SpeechControls)]
pub fn speech_controls(props: &SpeechControlsProps) -> Html {
    let is_listening = use_state(|| false);

    let toggle_listening = {
        let is_listening = is_listening.clone();
        Callback::from(move |_| {
            is_listening.set(!*is_listening);
            // TODO: Integrate Web Speech API
        })
    };

    html! {
        <div class="speech-controls">
            <button onclick={toggle_listening} class={if *is_listening { "listening" } else { "" }}>
                {if *is_listening { "🎤 Listening..." } else { "🎤 Speak" }}
            </button>
            <span class="language-indicator">{&props.language_code}</span>
        </div>
    }
}
