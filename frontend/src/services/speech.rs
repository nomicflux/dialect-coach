use dialect_coach_shared::tts::TtsRequest;
use gloo_net::http::Request;
use log::{error, info, warn};
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::HtmlAudioElement;
use yew::Callback;

// Speech Recognition Service for Speech-to-Text
pub struct SpeechRecognitionService {
    recognition: web_sys::SpeechRecognition,
    is_listening: Rc<RefCell<bool>>,
}

impl SpeechRecognitionService {
    /// Create a new speech recognition service
    pub fn new(language_code: &str) -> Result<Self, String> {
        let window = web_sys::window().ok_or("No window object available")?;

        // Try to get SpeechRecognition (webkit prefix for Safari)
        let recognition = if let Some(speech_recognition_constructor) =
            js_sys::Reflect::get(&window, &JsValue::from_str("SpeechRecognition"))
                .ok()
                .and_then(|val| val.dyn_into::<js_sys::Function>().ok())
        {
            // Standard API
            js_sys::Reflect::construct(&speech_recognition_constructor, &js_sys::Array::new())
                .ok()
                .and_then(|val| val.dyn_into::<web_sys::SpeechRecognition>().ok())
                .ok_or("Failed to construct SpeechRecognition")?
        } else if let Some(webkit_speech_recognition_constructor) =
            js_sys::Reflect::get(&window, &JsValue::from_str("webkitSpeechRecognition"))
                .ok()
                .and_then(|val| val.dyn_into::<js_sys::Function>().ok())
        {
            // WebKit prefixed API (Safari)
            js_sys::Reflect::construct(
                &webkit_speech_recognition_constructor,
                &js_sys::Array::new(),
            )
            .ok()
            .and_then(|val| val.dyn_into::<web_sys::SpeechRecognition>().ok())
            .ok_or("Failed to construct webkitSpeechRecognition")?
        } else {
            return Err("SpeechRecognition API not supported in this browser".to_string());
        };

        // Configure recognition
        let _ = recognition.set_continuous(false);
        recognition.set_interim_results(true);
        recognition.set_lang(language_code);
        recognition.set_max_alternatives(1);

        Ok(Self {
            recognition,
            is_listening: Rc::new(RefCell::new(false)),
        })
    }

    /// Check if speech recognition is supported
    pub fn is_supported() -> bool {
        if let Some(window) = web_sys::window() {
            js_sys::Reflect::has(&window, &JsValue::from_str("SpeechRecognition")).unwrap_or(false)
                || js_sys::Reflect::has(&window, &JsValue::from_str("webkitSpeechRecognition"))
                    .unwrap_or(false)
        } else {
            false
        }
    }

    /// Start listening with callbacks
    pub fn start_listening(
        &mut self,
        on_result: Callback<String>,
        on_interim: Option<Callback<String>>,
        on_error: Option<Callback<String>>,
        on_end: Option<Callback<()>>,
    ) -> Result<(), String> {
        if *self.is_listening.borrow() {
            warn!("Already listening, ignoring start request");
            return Ok(());
        }

        info!("Starting speech recognition");
        *self.is_listening.borrow_mut() = true;

        // Set up result handler
        let on_result_clone = on_result.clone();
        let on_interim_clone = on_interim.clone();
        let result_closure =
            Closure::wrap(Box::new(move |event: web_sys::SpeechRecognitionEvent| {
                if let Some(results) = event.results() {
                    // Get the latest result
                    if let Some(result) = results.get(results.length() - 1)
                        && let Some(alternative) = result.get(0)
                    {
                        let transcript = alternative.transcript();

                        if result.is_final() {
                            info!("Final transcript: {}", transcript);
                            on_result_clone.emit(transcript);
                        } else if let Some(interim_callback) = &on_interim_clone {
                            info!("Interim transcript: {}", transcript);
                            interim_callback.emit(transcript);
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);
        self.recognition
            .set_onresult(Some(result_closure.as_ref().unchecked_ref()));
        result_closure.forget();

        // Set up error handler
        if let Some(error_callback) = on_error {
            let error_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                // SpeechRecognitionErrorEvent doesn't exist in web-sys, use Event and extract type
                let error_msg = if let Some(error_event) = event.dyn_ref::<web_sys::Event>() {
                    format!("Speech recognition error: {:?}", error_event.type_())
                } else {
                    "Speech recognition error: unknown".to_string()
                };
                error!("{}", error_msg);
                error_callback.emit(error_msg);
            }) as Box<dyn FnMut(_)>);
            self.recognition
                .set_onerror(Some(error_closure.as_ref().unchecked_ref()));
            error_closure.forget();
        }

        // Set up end handler
        let is_listening_clone = self.is_listening.clone();
        let end_callback_clone = on_end.clone();
        let end_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            info!("Speech recognition ended");
            *is_listening_clone.borrow_mut() = false;
            if let Some(callback) = &end_callback_clone {
                callback.emit(());
            }
        }) as Box<dyn FnMut(_)>);
        self.recognition
            .set_onend(Some(end_closure.as_ref().unchecked_ref()));
        end_closure.forget();

        // Start recognition
        self.recognition
            .start()
            .map_err(|e| format!("Failed to start speech recognition: {:?}", e))?;

        Ok(())
    }

    /// Stop listening
    pub fn stop_listening(&mut self) {
        if *self.is_listening.borrow() {
            info!("Stopping speech recognition");
            self.recognition.stop();
            *self.is_listening.borrow_mut() = false;
        }
    }

    /// Abort listening immediately
    pub fn abort_listening(&mut self) {
        if *self.is_listening.borrow() {
            info!("Aborting speech recognition");
            self.recognition.abort();
            *self.is_listening.borrow_mut() = false;
        }
    }

    /// Check if currently listening
    pub fn is_listening(&self) -> bool {
        *self.is_listening.borrow()
    }

    /// Change the recognition language
    pub fn set_language(&mut self, language_code: &str) {
        info!("Changing speech recognition language to: {}", language_code);
        self.recognition.set_lang(language_code);
    }
}

/// Response from cloud TTS synthesis
#[derive(Deserialize)]
struct TtsSynthesizeResponse {
    audio_base64: String,
    duration_ms: u32,
}

/// Cloud TTS Service - calls backend /api/tts/synthesize endpoint
pub struct CloudTtsService {
    backend_url: String,
    audio_element: Option<HtmlAudioElement>,
}

impl CloudTtsService {
    /// Create a new cloud TTS service
    pub fn new(backend_url: &str) -> Self {
        // Create an audio element for playback
        let audio_element = HtmlAudioElement::new().ok();

        Self {
            backend_url: backend_url.to_string(),
            audio_element,
        }
    }

    /// Synthesize and play speech using backend TTS
    pub async fn speak(
        &self,
        user_id: uuid::Uuid,
        text: &str,
        language_code: &str,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Err("Cannot speak empty text".to_string());
        }

        info!("Synthesizing speech with backend TTS: {}", text,);

        // Build request
        let request = TtsRequest {
            user_id,
            text: text.to_string(),
            language_code: language_code.to_string(),
            rate: Some(1.0),
        };

        // Call backend API
        let url = format!("{}/api/tts/synthesize", self.backend_url);
        let response = Request::post(&url)
            .json(&request)
            .map_err(|e| format!("Failed to build request: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Failed to call TTS API: {}", e))?;

        if !response.ok() {
            let status = response.status();
            if status == 429 {
                return Err("TTS_RATE_LIMIT_EXCEEDED".to_string());
            }
            return Err(format!("TTS API error: {}", status));
        }

        let tts_response: TtsSynthesizeResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse TTS response: {}", e))?;

        info!(
            "Received {} ms of audio from backend TTS",
            tts_response.duration_ms
        );

        // Play the audio
        self.play_audio_base64(&tts_response.audio_base64).await?;

        Ok(())
    }

    /// Play base64-encoded MP3 audio
    async fn play_audio_base64(&self, audio_base64: &str) -> Result<(), String> {
        if let Some(audio) = &self.audio_element {
            // Convert base64 to data URL
            let data_url = format!("data:audio/mp3;base64,{}", audio_base64);
            audio.set_src(&data_url);

            // Play the audio
            let play_promise = audio
                .play()
                .map_err(|e| format!("Failed to play audio: {:?}", e))?;

            // Wait for playback to complete (convert promise to future)
            wasm_bindgen_futures::JsFuture::from(play_promise)
                .await
                .map_err(|e| format!("Audio playback failed: {:?}", e))?;

            info!("Audio playback started successfully");
            Ok(())
        } else {
            Err("Audio element not available".to_string())
        }
    }

    /// Stop playback immediately
    pub fn stop(&self) {
        if let Some(audio) = &self.audio_element {
            audio.pause().ok();
            audio.set_current_time(0.0);
            info!("Stopped TTS playback");
        }
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        if let Some(audio) = &self.audio_element {
            !audio.paused()
        } else {
            false
        }
    }
}
