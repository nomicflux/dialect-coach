# web-sys Tutorial: Browser APIs in Rust

## Overview

**web-sys** provides Rust bindings to all Web APIs available in browsers. It's auto-generated from WebIDL definitions, giving you type-safe access to the DOM, WebSockets, Web Storage, Web Speech API, and more.

**Version used in dialect-coach:** `0.3` with extensive feature flags

**Why we use it:** Enables direct interaction with browser APIs from Rust/WASM without manual JavaScript interop. Type-safe and zero-cost.

## Dependencies

```toml
[dependencies]
web-sys = { version = "0.3", features = [
    # Core DOM
    "Window",
    "Document",
    "HtmlElement",
    "HtmlSelectElement",
    "HtmlInputElement",
    "HtmlTextAreaElement",

    # Events
    "Event",
    "InputEvent",
    "SubmitEvent",
    "EventTarget",
    "MessageEvent",
    "CloseEvent",
    "ErrorEvent",

    # WebSocket
    "WebSocket",

    # Web Speech API
    "SpeechSynthesis",
    "SpeechSynthesisUtterance",
    "SpeechSynthesisVoice",
    "SpeechSynthesisEvent",
    "SpeechSynthesisErrorEvent",
    "SpeechRecognition",
    "SpeechRecognitionEvent",
    "SpeechRecognitionResult",
    "SpeechRecognitionResultList",
    "SpeechRecognitionAlternative",
] }
wasm-bindgen = "0.2"
```

**Key Principle:** You only pay for what you enable via features. Each API type must be explicitly listed.

## Core Concepts

### 1. Getting the Window and Document

Everything starts with the `Window` object:

```rust
use web_sys::{Window, Document, HtmlElement};

fn get_window() -> Option<Window> {
    web_sys::window()
}

fn get_document() -> Option<Document> {
    web_sys::window()?.document()
}
```

### 2. Type Casting with `dyn_into`

web-sys returns generic `Element` types that need casting:

```rust
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, Event};

fn get_input_value(event: Event) -> Option<String> {
    let target = event.target()?;
    let input = target.dyn_into::<HtmlInputElement>().ok()?;
    Some(input.value())
}
```

### 3. Optional Return Values

Most web-sys methods return `Option<T>` because APIs might fail:

```rust
let window = web_sys::window()?;  // Returns Option<Window>
let document = window.document()?; // Returns Option<Document>
let body = document.body()?;       // Returns Option<HtmlElement>
```

### 4. Result Types for Fallible Operations

Some operations return `Result<T, JsValue>`:

```rust
use web_sys::WebSocket;

fn connect(url: &str) -> Result<WebSocket, JsValue> {
    WebSocket::new(url)  // Returns Result
}
```

### 5. Closures and Memory Management

**The `.forget()` Pattern:** When you pass Rust closures to JavaScript as event handlers, you must ensure they live long enough. The `.forget()` method intentionally "leaks" the closure so JavaScript can keep calling it:

```rust
use wasm_bindgen::closure::Closure;

// Create a closure
let callback = Closure::wrap(Box::new(move || {
    web_sys::console::log_1(&"Event triggered!".into());
}) as Box<dyn FnMut()>);

// Pass to JavaScript
element.set_onclick(Some(callback.as_ref().unchecked_ref()));

// IMPORTANT: Forget the closure so it stays alive
callback.forget();  // Without this, the closure would be dropped!
```

**Why this is necessary:**
1. JavaScript receives a reference to the closure
2. Rust normally drops the closure when it goes out of scope
3. JavaScript would then try to call a freed closure → crash!
4. `.forget()` tells Rust "don't drop this, ever"

**When to use `.forget()`:** Always use it for event handlers that should last the lifetime of the app (like `onclick`, WebSocket callbacks). For temporary handlers, store the `Closure` in a struct and let it drop naturally when no longer needed.

## Step-by-Step: Building a Voice Recorder

Let's build an app that demonstrates key web-sys APIs.

### Step 1: DOM Manipulation

```rust
use wasm_bindgen::prelude::*;
use web_sys::{Window, Document, HtmlElement};

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Get window and document
    let window = web_sys::window()
        .ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window.document()
        .ok_or_else(|| JsValue::from_str("No document"))?;

    // Get body
    let body = document.body()
        .ok_or_else(|| JsValue::from_str("No body"))?;

    // Create elements
    let title = document.create_element("h1")?;
    title.set_text_content(Some("Voice Recorder"));

    let button = document.create_element("button")?;
    button.set_text_content(Some("Start Recording"));
    button.set_id("record-btn");

    // Append to body
    body.append_child(&title)?;
    body.append_child(&button)?;

    Ok(())
}
```

### Step 2: WebSocket Communication

```rust
use web_sys::{WebSocket, MessageEvent, ErrorEvent, CloseEvent};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn create_websocket(url: &str) -> Result<WebSocket, JsValue> {
    let ws = WebSocket::new(url)?;

    // Set binary type
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // onopen callback
    let onopen = Closure::wrap(Box::new(move |_event: JsValue| {
        web_sys::console::log_1(&"WebSocket connected".into());
    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    // onmessage callback
    let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
        if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
            web_sys::console::log_1(&format!("Received: {}", text).into());
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // onerror callback
    let onerror = Closure::wrap(Box::new(move |event: ErrorEvent| {
        web_sys::console::error_1(&"WebSocket error".into());
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    // onclose callback
    let onclose = Closure::wrap(Box::new(move |event: CloseEvent| {
        let code = event.code();
        let reason = event.reason();
        web_sys::console::log_1(&format!("Closed: {} - {}", code, reason).into());
    }) as Box<dyn FnMut(CloseEvent)>);
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    Ok(ws)
}

fn send_message(ws: &WebSocket, message: &str) -> Result<(), JsValue> {
    ws.send_with_str(message)
}
```

### Step 3: Speech Synthesis (Text-to-Speech)

```rust
use web_sys::{SpeechSynthesis, SpeechSynthesisUtterance, SpeechSynthesisVoice};

struct TTS {
    synth: SpeechSynthesis,
}

impl TTS {
    fn new() -> Result<Self, String> {
        let window = web_sys::window()
            .ok_or("No window")?;

        let synth = window.speech_synthesis()
            .ok_or("SpeechSynthesis not supported")?;

        Ok(Self { synth })
    }

    fn get_voices(&self) -> Vec<SpeechSynthesisVoice> {
        let voices_array = self.synth.get_voices();
        let mut voices = Vec::new();

        for i in 0..voices_array.length() {
            if let Some(voice) = voices_array.get(i) {
                voices.push(voice);
            }
        }

        voices
    }

    fn find_voice(&self, lang: &str) -> Option<SpeechSynthesisVoice> {
        self.get_voices()
            .into_iter()
            .find(|v| v.lang() == lang)
    }

    fn speak(&self, text: &str, lang: &str) -> Result<(), JsValue> {
        let utterance = SpeechSynthesisUtterance::new_with_text(text)?;

        // Set language
        utterance.set_lang(lang);

        // Set voice if available
        if let Some(voice) = self.find_voice(lang) {
            utterance.set_voice(Some(&voice));
        }

        // Set parameters
        utterance.set_rate(1.0);
        utterance.set_pitch(1.0);
        utterance.set_volume(1.0);

        // Speak
        self.synth.speak(&utterance);

        Ok(())
    }

    fn cancel(&self) {
        self.synth.cancel();
    }

    fn is_speaking(&self) -> bool {
        self.synth.speaking()
    }
}
```

## Browser Compatibility Strategy

**The Problem:** Different browsers implement Web APIs with different names. For example, Safari uses `webkitSpeechRecognition` while Chrome uses `SpeechRecognition`.

**The Solution:** Runtime feature detection using `js_sys::Reflect`:

```rust
use js_sys::Reflect;
use wasm_bindgen::JsValue;

// Try to get API, falling back to vendor-prefixed version
fn get_speech_recognition(window: &web_sys::Window) -> Option<js_sys::Function> {
    // Try standard API first
    if let Ok(constructor) = Reflect::get(window, &JsValue::from_str("SpeechRecognition"))
        .and_then(|val| val.dyn_into::<js_sys::Function>())
    {
        return Some(constructor);
    }

    // Fall back to webkit-prefixed API (Safari)
    if let Ok(webkit_constructor) = Reflect::get(window, &JsValue::from_str("webkitSpeechRecognition"))
        .and_then(|val| val.dyn_into::<js_sys::Function>())
    {
        return Some(webkit_constructor);
    }

    None
}
```

**How it works:**
1. **`Reflect::get`** - Gets a property from a JavaScript object by name (like `window.SpeechRecognition`)
2. **`dyn_into`** - Tries to cast the value to a Function
3. **Fallback chain** - Try standard name, then vendor prefix, then fail gracefully

**When to use:** Any time you use a Web API that might have vendor prefixes (Speech APIs, fullscreen, notifications, etc.).

### Step 4: Speech Recognition (Speech-to-Text)

```rust
use web_sys::SpeechRecognition;
use wasm_bindgen::prelude::*;
use js_sys::Reflect;

fn create_speech_recognition(lang: &str) -> Result<SpeechRecognition, String> {
    let window = web_sys::window()
        .ok_or("No window")?;

    // Try standard API first
    let recognition = if let Ok(constructor) =
        Reflect::get(&window, &JsValue::from_str("SpeechRecognition"))
            .and_then(|val| val.dyn_into::<js_sys::Function>())
    {
        Reflect::construct(&constructor, &js_sys::Array::new())
            .ok()
            .and_then(|val| val.dyn_into::<SpeechRecognition>().ok())
            .ok_or("Failed to construct SpeechRecognition")?
    }
    // Try webkit prefixed API (Safari)
    else if let Ok(constructor) =
        Reflect::get(&window, &JsValue::from_str("webkitSpeechRecognition"))
            .and_then(|val| val.dyn_into::<js_sys::Function>())
    {
        Reflect::construct(&constructor, &js_sys::Array::new())
            .ok()
            .and_then(|val| val.dyn_into::<SpeechRecognition>().ok())
            .ok_or("Failed to construct webkitSpeechRecognition")?
    } else {
        return Err("SpeechRecognition not supported".to_string());
    };

    // Configure
    recognition.set_continuous(false);
    recognition.set_interim_results(true);
    recognition.set_lang(lang);
    recognition.set_max_alternatives(1);

    Ok(recognition)
}

fn start_listening(recognition: &SpeechRecognition) -> Result<(), JsValue> {
    // Set up result callback
    let onresult = Closure::wrap(Box::new(move |event: web_sys::SpeechRecognitionEvent| {
        if let Some(results) = event.results() {
            if let Some(result) = results.get(results.length() - 1) {
                if let Some(alternative) = result.get(0) {
                    let transcript = alternative.transcript();
                    let is_final = result.is_final();

                    web_sys::console::log_1(
                        &format!("{}: {}",
                            if is_final { "Final" } else { "Interim" },
                            transcript
                        ).into()
                    );
                }
            }
        }
    }) as Box<dyn FnMut(_)>);
    recognition.set_onresult(Some(onresult.as_ref().unchecked_ref()));
    onresult.forget();

    // Start
    recognition.start()?;

    Ok(())
}
```

### Step 5: Complete Voice Recorder App

Let's combine all the pieces into a working voice recorder:

```rust
use wasm_bindgen::prelude::*;
use web_sys::{
    Window, Document, HtmlElement, SpeechSynthesis, SpeechSynthesisUtterance,
    SpeechRecognition, SpeechRecognitionEvent,
};
use wasm_bindgen::JsCast;

struct VoiceRecorder {
    window: Window,
    document: Document,
    synth: SpeechSynthesis,
    recognition: Option<SpeechRecognition>,
    transcript: String,
}

impl VoiceRecorder {
    fn new() -> Result<Self, String> {
        let window = web_sys::window()
            .ok_or("No window")?;
        let document = window.document()
            .ok_or("No document")?;
        let synth = window.speech_synthesis()
            .ok_or("SpeechSynthesis not supported")?;

        // Try to create speech recognition
        let recognition = Self::create_speech_recognition(&window);

        Ok(Self {
            window,
            document,
            synth,
            recognition,
            transcript: String::new(),
        })
    }

    fn create_speech_recognition(window: &Window) -> Option<SpeechRecognition> {
        // Try standard API
        if let Ok(constructor) = js_sys::Reflect::get(window, &JsValue::from_str("SpeechRecognition"))
            .and_then(|val| val.dyn_into::<js_sys::Function>())
        {
            if let Ok(recognition) = js_sys::Reflect::construct(&constructor, &js_sys::Array::new())
                .and_then(|val| val.dyn_into::<SpeechRecognition>())
            {
                recognition.set_continuous(false);
                recognition.set_interim_results(true);
                return Some(recognition);
            }
        }

        // Try webkit prefix (Safari)
        if let Ok(webkit_constructor) = js_sys::Reflect::get(window, &JsValue::from_str("webkitSpeechRecognition"))
            .and_then(|val| val.dyn_into::<js_sys::Function>())
        {
            if let Ok(recognition) = js_sys::Reflect::construct(&webkit_constructor, &js_sys::Array::new())
                .and_then(|val| val.dyn_into::<SpeechRecognition>())
            {
                recognition.set_continuous(false);
                recognition.set_interim_results(true);
                return Some(recognition);
            }
        }

        None
    }

    fn setup_ui(&self) -> Result<(), String> {
        let body = self.document.body()
            .ok_or("No body element")?;

        // Title
        let title = self.document.create_element("h1")
            .map_err(|_| "Failed to create h1")?;
        title.set_text_content(Some("Voice Recorder"));
        body.append_child(&title)
            .map_err(|_| "Failed to append title")?;

        // Record button
        let record_btn = self.document.create_element("button")
            .map_err(|_| "Failed to create button")?;
        record_btn.set_id("record-btn");
        record_btn.set_text_content(Some("🎤 Start Recording"));
        body.append_child(&record_btn)
            .map_err(|_| "Failed to append button")?;

        // Speak button
        let speak_btn = self.document.create_element("button")
            .map_err(|_| "Failed to create button")?;
        speak_btn.set_id("speak-btn");
        speak_btn.set_text_content(Some("🔊 Speak Text"));
        body.append_child(&speak_btn)
            .map_err(|_| "Failed to append button")?;

        // Transcript display
        let transcript_div = self.document.create_element("div")
            .map_err(|_| "Failed to create div")?;
        transcript_div.set_id("transcript");
        transcript_div.set_inner_html("<p>Transcript will appear here...</p>");
        body.append_child(&transcript_div)
            .map_err(|_| "Failed to append div")?;

        Ok(())
    }

    fn attach_handlers(&mut self) -> Result<(), String> {
        // Record button handler
        if let Some(record_btn) = self.document.get_element_by_id("record-btn") {
            if let Some(recognition) = &self.recognition {
                let recognition_clone = recognition.clone();
                let document_clone = self.document.clone();

                // Set up result handler
                let onresult = Closure::wrap(Box::new(move |event: SpeechRecognitionEvent| {
                    if let Some(results) = event.results() {
                        if let Some(result) = results.get(results.length() - 1) {
                            if let Some(alternative) = result.get(0) {
                                let text = alternative.transcript();
                                if let Some(transcript_div) = document_clone.get_element_by_id("transcript") {
                                    transcript_div.set_inner_html(&format!("<p>{}</p>", text));
                                }
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);

                recognition.set_onresult(Some(onresult.as_ref().unchecked_ref()));
                onresult.forget();

                // Click handler to start recording
                let onclick = Closure::wrap(Box::new(move |_| {
                    let _ = recognition_clone.start();
                    web_sys::console::log_1(&"Recording started".into());
                }) as Box<dyn FnMut(_)>);

                record_btn.set_onclick(Some(onclick.as_ref().unchecked_ref()));
                onclick.forget();
            }
        }

        // Speak button handler
        if let Some(speak_btn) = self.document.get_element_by_id("speak-btn") {
            let synth_clone = self.synth.clone();
            let document_clone = self.document.clone();

            let onclick = Closure::wrap(Box::new(move |_| {
                if let Some(transcript_div) = document_clone.get_element_by_id("transcript") {
                    let text = transcript_div.text_content().unwrap_or_default();
                    if !text.is_empty() {
                        if let Ok(utterance) = SpeechSynthesisUtterance::new_with_text(&text) {
                            synth_clone.speak(&utterance);
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            speak_btn.set_onclick(Some(onclick.as_ref().unchecked_ref()));
            onclick.forget();
        }

        Ok(())
    }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let mut recorder = VoiceRecorder::new()
        .map_err(|e| JsValue::from_str(&e))?;

    recorder.setup_ui()
        .map_err(|e| JsValue::from_str(&e))?;

    recorder.attach_handlers()
        .map_err(|e| JsValue::from_str(&e))?;

    web_sys::console::log_1(&"Voice Recorder initialized!".into());

    Ok(())
}
```

**What this demonstrates:**
- **Complete application flow** - DOM creation, event handling, browser APIs
- **Error handling** - Proper `Result` types throughout
- **Browser compatibility** - Feature detection for Speech Recognition
- **Closure management** - `.forget()` for persistent event handlers
- **State encapsulation** - `VoiceRecorder` struct owns all resources

**To use:** Compile with `wasm-pack build --target web` and load in a browser with an HTML file that includes the generated JS.

## How dialect-coach Uses web-sys

### 1. Speech Synthesis Service (frontend/src/services/speech.rs:10-281)

Complete TTS implementation:

```rust
pub struct SpeechSynthesisService {
    synth: SpeechSynthesis,
    voices: Rc<RefCell<Vec<SpeechSynthesisVoice>>>,
    voices_loaded: Rc<RefCell<bool>>,
}

impl SpeechSynthesisService {
    pub fn new() -> Result<Self, String> {
        let window = web_sys::window()
            .ok_or("No window object available")?;

        let synth = window
            .speech_synthesis()
            .ok_or("SpeechSynthesis API not supported")?;

        // Initialize voices storage
        let voices = Rc::new(RefCell::new(Vec::new()));
        let voices_loaded = Rc::new(RefCell::new(false));

        // Try loading voices immediately
        let available_voices = synth.get_voices();
        if available_voices.length() > 0 {
            let mut voices_vec = Vec::new();
            for i in 0..available_voices.length() {
                if let Some(voice) = available_voices.get(i) {
                    voices_vec.push(voice);
                }
            }
            *voices.borrow_mut() = voices_vec;
            *voices_loaded.borrow_mut() = true;
        }

        Ok(Self { synth, voices, voices_loaded })
    }
}
```

**Pattern:** Wrap browser APIs in Rust structs with proper error handling.

### 2. Voice Selection by Language (speech.rs:88-120)

Three-tier fallback for finding voices:

```rust
pub fn get_voices_for_language(&self, language_code: &str) -> Vec<SpeechSynthesisVoice> {
    let voices = self.voices.borrow();
    let language_prefix = language_code.split('-').next().unwrap_or("");

    let mut exact_matches = Vec::new();
    let mut prefix_matches = Vec::new();

    for voice in voices.iter() {
        let voice_lang = voice.lang();
        if voice_lang == language_code {
            exact_matches.push(voice.clone());  // Exact: "es-MX"
        } else if voice_lang.starts_with(language_prefix) {
            prefix_matches.push(voice.clone());  // Prefix: "es"
        }
    }

    if !exact_matches.is_empty() {
        exact_matches
    } else if !prefix_matches.is_empty() {
        prefix_matches
    } else {
        Vec::new()
    }
}
```

**Pattern:** Graceful degradation when exact matches aren't available.

### 3. Speech with Event Callbacks (speech.rs:182-242)

Using closures for events:

```rust
pub fn speak_with_callbacks(
    &self,
    text: &str,
    language_code: &str,
    on_start: Option<Callback<()>>,
    on_end: Option<Callback<()>>,
    on_error: Option<Callback<String>>,
) -> Result<(), String> {
    let utterance = SpeechSynthesisUtterance::new_with_text(text)
        .map_err(|_| "Failed to create utterance")?;

    utterance.set_lang(language_code);

    // Set up start callback
    if let Some(callback) = on_start {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::SpeechSynthesisEvent| {
            callback.emit(());
        }) as Box<dyn FnMut(_)>);
        utterance.set_onstart(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    // Set up end callback
    if let Some(callback) = on_end {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::SpeechSynthesisEvent| {
            callback.emit(());
        }) as Box<dyn FnMut(_)>);
        utterance.set_onend(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    // Set up error callback
    if let Some(callback) = on_error {
        let closure = Closure::wrap(Box::new(move |event: SpeechSynthesisErrorEvent| {
            let error_msg = format!("Speech synthesis error: {:?}", event.type_());
            callback.emit(error_msg);
        }) as Box<dyn FnMut(_)>);
        utterance.set_onerror(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    self.synth.speak(&utterance);
    Ok(())
}
```

**Pattern:** Optional callbacks for different lifecycle events. Note the `.forget()` to keep closures alive.

### 4. Speech Recognition with Browser Compatibility (speech.rs:290-330)

Handling vendor prefixes:

```rust
pub fn new(language_code: &str) -> Result<Self, String> {
    let window = web_sys::window()
        .ok_or("No window object available")?;

    // Try standard API
    let recognition = if let Some(constructor) =
        js_sys::Reflect::get(&window, &JsValue::from_str("SpeechRecognition"))
            .ok()
            .and_then(|val| val.dyn_into::<js_sys::Function>().ok())
    {
        js_sys::Reflect::construct(&constructor, &js_sys::Array::new())
            .ok()
            .and_then(|val| val.dyn_into::<web_sys::SpeechRecognition>().ok())
            .ok_or("Failed to construct SpeechRecognition")?
    }
    // Try webkit prefixed API (Safari)
    else if let Some(webkit_constructor) =
        js_sys::Reflect::get(&window, &JsValue::from_str("webkitSpeechRecognition"))
            .ok()
            .and_then(|val| val.dyn_into::<js_sys::Function>().ok())
    {
        js_sys::Reflect::construct(&webkit_constructor, &js_sys::Array::new())
            .ok()
            .and_then(|val| val.dyn_into::<web_sys::SpeechRecognition>().ok())
            .ok_or("Failed to construct webkitSpeechRecognition")?
    } else {
        return Err("SpeechRecognition API not supported".to_string());
    };

    recognition.set_continuous(false);
    recognition.set_interim_results(true);
    recognition.set_lang(language_code);

    Ok(Self {
        recognition,
        is_listening: Rc::new(RefCell::new(false)),
    })
}
```

**Pattern:** Runtime feature detection and vendor prefix fallback.

### 5. Processing Recognition Results (speech.rs:361-378)

Extracting transcript from results:

```rust
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
```

**Pattern:** Navigate the results tree structure to extract the latest transcript.

### 6. Form Event Handling (app.rs:186-205)

Getting values from form elements:

```rust
let on_language_change = {
    let selected_language = selected_language.clone();
    let selected_dialect = selected_dialect.clone();

    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let lang = match value.as_str() {
                "spanish" => Language::Spanish,
                "arabic" => Language::Arabic,
                "french" => Language::French,
                _ => Language::Spanish,
            };
            selected_language.set(lang);
            // Update dialect to match...
        }
    })
};
```

**Pattern:** Use `target_dyn_into` to safely cast event targets to specific element types.

## Common Patterns

### Pattern 1: Safe Element Access

```rust
fn get_element_by_id(id: &str) -> Result<HtmlElement, String> {
    web_sys::window()
        .ok_or("No window")?
        .document()
        .ok_or("No document")?
        .get_element_by_id(id)
        .ok_or_else(|| format!("Element {} not found", id))?
        .dyn_into::<HtmlElement>()
        .map_err(|_| format!("Element {} is not an HtmlElement", id))
}
```

### Pattern 2: Event Listener with Closure

```rust
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

fn add_click_listener(element: &HtmlElement) -> Result<(), JsValue> {
    let callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        web_sys::console::log_1(&"Clicked!".into());
    }) as Box<dyn FnMut(_)>);

    element.add_event_listener_with_callback(
        "click",
        callback.as_ref().unchecked_ref()
    )?;

    callback.forget();  // Keep closure alive
    Ok(())
}
```

### Pattern 3: Feature Detection

```rust
fn is_feature_supported(feature: &str) -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };

    js_sys::Reflect::has(&window, &JsValue::from_str(feature))
        .unwrap_or(false)
}

// Usage
if is_feature_supported("SpeechRecognition") {
    // Use the API
}
```

### Pattern 4: Handling JsArray Iteration

```rust
use js_sys::Array;

fn iterate_js_array<T>(array: &Array, mut f: impl FnMut(T))
where
    T: JsCast,
{
    for i in 0..array.length() {
        if let Some(item) = array.get(i).dyn_into::<T>().ok() {
            f(item);
        }
    }
}

// Usage with voices
iterate_js_array(&synth.get_voices(), |voice: SpeechSynthesisVoice| {
    console::log_1(&format!("Voice: {}", voice.name()).into());
});
```

## Best Practices from dialect-coach

1. **Wrap browser APIs in structs** - Encapsulate complexity and provide Rust-friendly interfaces
2. **Use feature detection** - Check for API availability before using (speech.rs:79-85, 333-340)
3. **Provide fallbacks** - Support vendor prefixes and degraded functionality
4. **`.forget()` closures** - Event handlers must outlive their registration
5. **Error propagation** - Use `Result` types and proper error messages
6. **Rc<RefCell<>>` for mutable state** - Needed when closures need to mutate shared state
7. **Type casting safety** - Always handle `dyn_into` failures gracefully

## Troubleshooting

### Issue: "Cannot read property of undefined"

**Cause:** Browser API not available or feature not enabled in Cargo.toml

**Solution:** Add required features and check for API availability:

```rust
if let Some(synth) = window.speech_synthesis() {
    // Use API
} else {
    // Fallback or error message
}
```

### Issue: Closure dropped too early

**Cause:** Closure cleaned up before event fires

**Solution:** Call `.forget()` on closures used as event handlers:

```rust
let closure = Closure::wrap(/* ... */);
element.set_onclick(Some(closure.as_ref().unchecked_ref()));
closure.forget();  // Don't drop!
```

### Issue: Type mismatch on event target

**Cause:** Event target is generic `EventTarget`, not specific element type

**Solution:** Use `dyn_into` to cast:

```rust
if let Some(input) = event.target()
    .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
{
    let value = input.value();
}
```

### Issue: Method not found on web-sys type

**Cause:** Feature not enabled in Cargo.toml

**Solution:** Check docs.rs for required feature flag and add it:

```toml
web-sys = { version = "0.3", features = ["SpeechRecognitionEvent"] }
```

## Further Resources

- **Official Docs:** https://rustwasm.github.io/wasm-bindgen/web-sys/index.html
- **Crate:** https://crates.io/crates/web-sys
- **API Docs:** https://docs.rs/web-sys/latest/web_sys/
- **Feature List:** https://docs.rs/web-sys/latest/web_sys/#features
- **MDN Web Docs:** https://developer.mozilla.org/en-US/docs/Web/API (for understanding browser APIs)

## Summary

web-sys provides type-safe Rust bindings to browser APIs:

- **Feature flags** control which APIs are available (pay-for-what-you-use)
- **Optional returns** reflect JavaScript's nullable nature
- **Type casting** with `dyn_into` for specific element types
- **Closures with `.forget()`** for event handlers
- **Feature detection** for browser compatibility
- **Error handling** with Result types

The dialect-coach project demonstrates production patterns including speech APIs, WebSocket communication, form handling, and cross-browser compatibility with vendor prefixes.
