use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{SpeechSynthesis, SpeechSynthesisUtterance, SpeechSynthesisVoice};
use yew::Callback;
use log::{info, warn, error};
use std::rc::Rc;
use std::cell::RefCell;

/// Speech Synthesis Service for Text-to-Speech
pub struct SpeechSynthesisService {
    synth: SpeechSynthesis,
    voices: Rc<RefCell<Vec<SpeechSynthesisVoice>>>,
    voices_loaded: Rc<RefCell<bool>>,
}

impl SpeechSynthesisService {
    /// Create a new speech synthesis service
    pub fn new() -> Result<Self, String> {
        let window = web_sys::window()
            .ok_or("No window object available")?;

        let synth = window
            .speech_synthesis()
            .map_err(|_| "SpeechSynthesis API not supported in this browser")?;

        let voices = Rc::new(RefCell::new(Vec::new()));
        let voices_loaded = Rc::new(RefCell::new(false));

        // Load voices immediately if available
        let voices_clone = voices.clone();
        let voices_loaded_clone = voices_loaded.clone();
        let synth_clone = synth.clone();

        // Try to load voices now
        let available_voices = synth.get_voices();
        if available_voices.length() > 0 {
            let mut voices_vec = Vec::new();
            for i in 0..available_voices.length() {
                if let Ok(voice) = available_voices.get(i).dyn_into::<SpeechSynthesisVoice>() {
                    voices_vec.push(voice);
                }
            }
            *voices_clone.borrow_mut() = voices_vec;
            *voices_loaded_clone.borrow_mut() = true;
            info!("Loaded {} voices immediately", available_voices.length());
        } else {
            // Set up voiceschanged event listener for browsers that load voices async
            let voices_clone2 = voices.clone();
            let voices_loaded_clone2 = voices_loaded.clone();
            let synth_clone2 = synth.clone();

            let closure = Closure::wrap(Box::new(move || {
                let available_voices = synth_clone2.get_voices();
                let mut voices_vec = Vec::new();
                for i in 0..available_voices.length() {
                    if let Ok(voice) = available_voices.get(i).dyn_into::<SpeechSynthesisVoice>() {
                        voices_vec.push(voice);
                    }
                }
                *voices_clone2.borrow_mut() = voices_vec;
                *voices_loaded_clone2.borrow_mut() = true;
                info!("Loaded {} voices via voiceschanged event", available_voices.length());
            }) as Box<dyn FnMut()>);

            synth.set_onvoiceschanged(Some(closure.as_ref().unchecked_ref()));
            closure.forget(); // Keep the closure alive
        }

        Ok(Self {
            synth,
            voices,
            voices_loaded,
        })
    }

    /// Check if speech synthesis is supported
    pub fn is_supported() -> bool {
        web_sys::window().is_some()
    }

    /// Get all available voices for a specific language (BCP-47 code like "es-MX", "ar-EG")
    pub fn get_voices_for_language(&self, language_code: &str) -> Vec<SpeechSynthesisVoice> {
        let voices = self.voices.borrow();

        // Three-tier fallback:
        // 1. Exact match (e.g., "es-MX")
        // 2. Language prefix match (e.g., "es")
        // 3. Return empty if none found

        let language_prefix = language_code.split('-').next().unwrap_or("");

        let mut exact_matches = Vec::new();
        let mut prefix_matches = Vec::new();

        for voice in voices.iter() {
            let voice_lang = voice.lang();
            if voice_lang == language_code {
                exact_matches.push(voice.clone());
            } else if voice_lang.starts_with(language_prefix) {
                prefix_matches.push(voice.clone());
            }
        }

        if !exact_matches.is_empty() {
            info!("Found {} exact voice matches for {}", exact_matches.len(), language_code);
            exact_matches
        } else if !prefix_matches.is_empty() {
            info!("Found {} prefix voice matches for {}", prefix_matches.len(), language_prefix);
            prefix_matches
        } else {
            warn!("No voices found for language: {}", language_code);
            Vec::new()
        }
    }

    /// Select the best voice for a given language code
    fn select_voice(&self, language_code: &str) -> Option<SpeechSynthesisVoice> {
        let matching_voices = self.get_voices_for_language(language_code);

        if matching_voices.is_empty() {
            return None;
        }

        // Prefer voices that are marked as default for the language
        for voice in &matching_voices {
            if voice.default() {
                info!("Selected default voice: {}", voice.name());
                return Some(voice.clone());
            }
        }

        // Otherwise, return the first matching voice
        info!("Selected first available voice: {}", matching_voices[0].name());
        Some(matching_voices[0].clone())
    }

    /// Speak text in the specified language
    pub fn speak(&self, text: &str, language_code: &str) -> Result<(), String> {
        if text.is_empty() {
            return Err("Cannot speak empty text".to_string());
        }

        // Check if voices are loaded
        if !*self.voices_loaded.borrow() {
            warn!("Voices not yet loaded, speech may use default voice");
        }

        // Create utterance
        let utterance = SpeechSynthesisUtterance::new_with_text(text)
            .map_err(|_| "Failed to create speech utterance")?;

        // Set language
        utterance.set_lang(language_code);

        // Try to select a specific voice for this language
        if let Some(voice) = self.select_voice(language_code) {
            utterance.set_voice(Some(&voice));
            info!("Speaking with voice: {} ({})", voice.name(), voice.lang());
        } else {
            warn!("No specific voice found, using browser default for {}", language_code);
        }

        // Set speech parameters
        utterance.set_rate(0.9); // Slightly slower for language learning
        utterance.set_pitch(1.0);
        utterance.set_volume(1.0);

        // Speak
        self.synth.speak(&utterance);
        info!("Started speaking: {} chars in {}", text.len(), language_code);

        Ok(())
    }

    /// Speak text with callbacks for events
    pub fn speak_with_callbacks(
        &self,
        text: &str,
        language_code: &str,
        on_start: Option<Callback<()>>,
        on_end: Option<Callback<()>>,
        on_error: Option<Callback<String>>,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Err("Cannot speak empty text".to_string());
        }

        // Create utterance
        let utterance = SpeechSynthesisUtterance::new_with_text(text)
            .map_err(|_| "Failed to create speech utterance")?;

        // Set language
        utterance.set_lang(language_code);

        // Try to select a specific voice for this language
        if let Some(voice) = self.select_voice(language_code) {
            utterance.set_voice(Some(&voice));
        }

        // Set speech parameters
        utterance.set_rate(0.9);
        utterance.set_pitch(1.0);
        utterance.set_volume(1.0);

        // Set up event handlers
        if let Some(callback) = on_start {
            let closure = Closure::wrap(Box::new(move |_event: web_sys::SpeechSynthesisEvent| {
                callback.emit(());
            }) as Box<dyn FnMut(_)>);
            utterance.set_onstart(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        if let Some(callback) = on_end {
            let closure = Closure::wrap(Box::new(move |_event: web_sys::SpeechSynthesisEvent| {
                callback.emit(());
            }) as Box<dyn FnMut(_)>);
            utterance.set_onend(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        if let Some(callback) = on_error {
            let closure = Closure::wrap(Box::new(move |event: web_sys::SpeechSynthesisErrorEvent| {
                // SpeechSynthesisErrorEvent.error() may not be available, use type instead
                let error_msg = format!("Speech synthesis error: {:?}", event.type_());
                callback.emit(error_msg);
            }) as Box<dyn FnMut(_)>);
            utterance.set_onerror(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        // Speak
        self.synth.speak(&utterance);
        info!("Started speaking with callbacks: {} chars in {}", text.len(), language_code);

        Ok(())
    }

    /// Stop speaking immediately
    pub fn stop(&self) {
        self.synth.cancel();
        info!("Stopped speech synthesis");
    }

    /// Pause speaking
    pub fn pause(&self) {
        self.synth.pause();
        info!("Paused speech synthesis");
    }

    /// Resume speaking
    pub fn resume(&self) {
        self.synth.resume();
        info!("Resumed speech synthesis");
    }

    /// Check if currently speaking
    pub fn is_speaking(&self) -> bool {
        self.synth.speaking()
    }

    /// Check if currently paused
    pub fn is_paused(&self) -> bool {
        self.synth.paused()
    }

    /// Check if voices are loaded
    pub fn are_voices_loaded(&self) -> bool {
        *self.voices_loaded.borrow()
    }

    /// Get all available voices
    pub fn get_all_voices(&self) -> Vec<SpeechSynthesisVoice> {
        self.voices.borrow().clone()
    }
}

/// Speech Recognition Service for Speech-to-Text
pub struct SpeechRecognitionService {
    recognition: web_sys::SpeechRecognition,
    is_listening: Rc<RefCell<bool>>,
}

impl SpeechRecognitionService {
    /// Create a new speech recognition service
    pub fn new(language_code: &str) -> Result<Self, String> {
        let window = web_sys::window()
            .ok_or("No window object available")?;

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
            js_sys::Reflect::construct(&webkit_speech_recognition_constructor, &js_sys::Array::new())
                .ok()
                .and_then(|val| val.dyn_into::<web_sys::SpeechRecognition>().ok())
                .ok_or("Failed to construct webkitSpeechRecognition")?
        } else {
            return Err("SpeechRecognition API not supported in this browser".to_string());
        };

        // Configure recognition
        recognition.set_continuous(false); // Stop after user pauses
        recognition.set_interim_results(true); // Get partial results while speaking
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
            js_sys::Reflect::has(&window, &JsValue::from_str("SpeechRecognition")).unwrap_or(false) ||
            js_sys::Reflect::has(&window, &JsValue::from_str("webkitSpeechRecognition")).unwrap_or(false)
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
        let result_closure = Closure::wrap(Box::new(move |event: web_sys::SpeechRecognitionEvent| {
            if let Some(results) = event.results() {
                // Get the latest result
                if let Some(result) = results.get(results.length() - 1) {
                    if let Some(alternative) = result.get(0) {
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
            }
        }) as Box<dyn FnMut(_)>);
        self.recognition.set_onresult(Some(result_closure.as_ref().unchecked_ref()));
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
            self.recognition.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
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
        self.recognition.set_onend(Some(end_closure.as_ref().unchecked_ref()));
        end_closure.forget();

        // Start recognition
        self.recognition.start()
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
