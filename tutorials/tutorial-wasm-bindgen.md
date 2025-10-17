# wasm-bindgen Tutorial: JavaScript Interop for Rust/WASM

## Overview

**wasm-bindgen** facilitates high-level interactions between Rust-generated WebAssembly and JavaScript. It provides bindings to JavaScript APIs, enables calling Rust from JS and vice versa, and handles type conversions automatically.

**Version used in dialect-coach:** `0.2` with wasm-bindgen-futures `0.4`

**Why we use it:** Makes Rust/WASM development practical by eliminating manual FFI, providing automatic type conversions, and enabling seamless async integration.

## Dependencies

```toml
[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"  # For Promise integration
js-sys = "0.3"                # JavaScript standard library
web-sys = "0.3"               # Browser APIs

[lib]
crate-type = ["cdylib", "rlib"]
```

## Core Concepts

### 1. The `#[wasm_bindgen]` Attribute

Marks Rust items for JavaScript exposure:

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// JavaScript can now call: wasm.add(5, 3)
```

### 2. JsValue - The Universal Type

`JsValue` represents any JavaScript value:

```rust
use wasm_bindgen::JsValue;

fn log_to_console(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}
```

### 3. Closure - Rust Functions for JavaScript

Wrap Rust closures to pass as JavaScript callbacks:

```rust
use wasm_bindgen::closure::Closure;

let callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
    // Handle event
}) as Box<dyn FnMut(web_sys::Event)>);

// Use with JavaScript API
element.set_onclick(Some(callback.as_ref().unchecked_ref()));

// Keep closure alive
callback.forget();
```

### 4. Type Conversions

Automatic conversions between Rust and JavaScript types:

| Rust Type | JavaScript Type |
|-----------|----------------|
| `i32`, `u32`, `f64` | `Number` |
| `bool` | `Boolean` |
| `String`, `&str` | `String` |
| `Vec<T>` | `Array` |
| `Option<T>` | `undefined` or value |
| `Result<T, E>` | `Promise` (with futures) |

### 5. JsCast Trait

Safe type casting for JavaScript types:

```rust
use wasm_bindgen::JsCast;

// Downcast to specific type
if let Some(input) = element.dyn_into::<HtmlInputElement>().ok() {
    let value = input.value();
}

// Assert type (panics if wrong)
let input = element.unchecked_into::<HtmlInputElement>();
```

## Understanding WASM Closures

**The Problem:** JavaScript expects callback functions. Rust has closures, but they live in Rust's memory model. How do we bridge this gap?

**The Solution:** `wasm_bindgen::closure::Closure` wraps Rust closures for JavaScript:

```rust
use wasm_bindgen::closure::Closure;

// Step 1: Create the Rust closure
let callback = Closure::wrap(
    Box::new(move |event: web_sys::Event| {
        // Your Rust code here
        web_sys::console::log_1(&"Event fired!".into());
    }) as Box<dyn FnMut(web_sys::Event)>
);

// Step 2: Pass reference to JavaScript
element.set_onclick(Some(callback.as_ref().unchecked_ref()));

// Step 3: Prevent Rust from dropping the closure
callback.forget();
```

**Breaking it down:**

1. **`Box::new(move |event| { ... })`** - Heap-allocate the closure
   - `move` transfers ownership of captured variables into the closure
   - Heap allocation (`Box`) is necessary because JavaScript needs a stable pointer

2. **`as Box<dyn FnMut(web_sys::Event)>`** - Type annotation for trait object
   - `dyn FnMut` means "any type that implements FnMut"
   - The type annotation tells Rust what trait object to create
   - `FnMut` (not `Fn`) because we might mutate captured state

3. **`Closure::wrap(...)`** - Wraps the Rust closure for JavaScript
   - Creates a JavaScript-compatible function pointer
   - Handles calling convention conversion (Rust ABI ↔ JavaScript ABI)

4. **`.as_ref().unchecked_ref()`** - Gets the JavaScript reference
   - `.as_ref()` gets a reference to the `Closure`
   - `.unchecked_ref()` converts to `&js_sys::Function` without runtime checks

5. **`.forget()`** - Prevents dropping the closure
   - Without this, Rust drops the closure when it goes out of scope
   - JavaScript would then have a dangling pointer → crash
   - `.forget()` intentionally leaks memory, which is correct here

**Why this specific syntax?** Each piece is necessary:
- `Box` → stable heap pointer
- `move` → ownership transfer
- `as Box<dyn ...>` → type inference help
- `Closure::wrap` → ABI conversion
- `.forget()` → lifetime management

**Memory leak?** Yes, but intentional! The closure must live as long as JavaScript might call it. For temporary callbacks, store the `Closure` in a struct and drop it when done (don't call `.forget()`).

## Step-by-Step: Building a Calculator with JS Interop

Let's build a calculator that demonstrates wasm-bindgen patterns.

### Step 1: Basic Function Export

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn calculate(op: &str, a: f64, b: f64) -> f64 {
    match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => a / b,
        _ => 0.0,
    }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}
```

### Step 2: Importing JavaScript Functions

```rust
#[wasm_bindgen]
extern "C" {
    // Import console.log
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // Import Math.random
    #[wasm_bindgen(js_namespace = Math)]
    fn random() -> f64;

    // Import custom JavaScript function
    #[wasm_bindgen(js_name = customAlert)]
    fn alert_custom(msg: &str);
}

#[wasm_bindgen]
pub fn generate_random() -> f64 {
    log("Generating random number");
    random()
}
```

### Step 3: Working with JavaScript Objects

```rust
use js_sys::{Object, Reflect};

#[wasm_bindgen]
pub struct Calculator {
    history: Vec<String>,
}

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Calculator {
        Calculator {
            history: Vec::new(),
        }
    }

    pub fn compute(&mut self, expression: &str) -> Result<f64, JsValue> {
        // Simple eval (don't do this in production!)
        let result = js_eval(expression)?;
        let num = result.as_f64()
            .ok_or_else(|| JsValue::from_str("Not a number"))?;

        self.history.push(format!("{} = {}", expression, num));
        Ok(num)
    }

    pub fn get_history(&self) -> js_sys::Array {
        self.history
            .iter()
            .map(|s| JsValue::from_str(s))
            .collect()
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = eval)]
    fn js_eval(s: &str) -> Result<JsValue, JsValue>;
}
```

### Step 4: Closures and Event Handlers

```rust
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, MouseEvent};

#[wasm_bindgen]
pub fn setup_calculator() -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    // Get button
    let button = document
        .get_element_by_id("calc-button")
        .unwrap()
        .dyn_into::<HtmlElement>()?;

    // Create click handler
    let onclick = Closure::wrap(Box::new(move |event: MouseEvent| {
        web_sys::console::log_1(&"Button clicked!".into());

        // Get input value
        if let Some(input) = document
            .get_element_by_id("expression")
            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
        {
            let expression = input.value();
            web_sys::console::log_1(&format!("Expression: {}", expression).into());
        }
    }) as Box<dyn FnMut(MouseEvent)>);

    // Attach handler
    button.set_onclick(Some(onclick.as_ref().unchecked_ref()));

    // Keep closure alive
    onclick.forget();

    Ok(())
}
```

### Step 5: Async/Await with Promises

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

#[wasm_bindgen]
pub async fn fetch_data(url: &str) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");

    let request = Request::new_with_str_and_init(url, &opts)?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    let resp: Response = resp_value.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;

    Ok(json)
}

#[wasm_bindgen]
pub async fn calculate_async(a: f64, b: f64) -> Result<f64, JsValue> {
    // Simulate async work
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let result = a + b;
        resolve.call1(&JsValue::NULL, &JsValue::from_f64(result)).unwrap();
    });

    let result = JsFuture::from(promise).await?;
    Ok(result.as_f64().unwrap())
}
```

### Step 5: Complete Calculator UI

Let's combine everything into a working calculator:

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlElement};
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub struct Calculator {
    history: Vec<String>,
}

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Calculator {
        Calculator {
            history: Vec::new(),
        }
    }

    pub fn add(&self, a: f64, b: f64) -> f64 {
        a + b
    }

    pub fn subtract(&self, a: f64, b: f64) -> f64 {
        a - b
    }

    pub fn multiply(&self, a: f64, b: f64) -> f64 {
        a * b
    }

    pub fn divide(&self, a: f64, b: f64) -> Result<f64, String> {
        if b == 0.0 {
            Err("Division by zero".to_string())
        } else {
            Ok(a / b)
        }
    }

    pub fn add_to_history(&mut self, entry: String) {
        self.history.push(entry);
    }

    pub fn get_history(&self) -> js_sys::Array {
        self.history
            .iter()
            .map(|s| JsValue::from_str(s))
            .collect()
    }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    let body = document.body().ok_or("No body")?;

    // Create UI
    let container = document.create_element("div")?;
    container.set_id("calculator");

    // Title
    let title = document.create_element("h2")?;
    title.set_text_content(Some("WASM Calculator"));
    container.append_child(&title)?;

    // Input 1
    let input1 = document.create_element("input")?;
    input1.set_id("num1");
    input1.set_attribute("type", "number")?;
    input1.set_attribute("placeholder", "First number")?;
    container.append_child(&input1)?;

    // Operator select
    let select = document.create_element("select")?;
    select.set_id("operator");
    for op in &["+", "-", "*", "/"] {
        let option = document.create_element("option")?;
        option.set_text_content(Some(op));
        select.append_child(&option)?;
    }
    container.append_child(&select)?;

    // Input 2
    let input2 = document.create_element("input")?;
    input2.set_id("num2");
    input2.set_attribute("type", "number")?;
    input2.set_attribute("placeholder", "Second number")?;
    container.append_child(&input2)?;

    // Calculate button
    let calc_button = document.create_element("button")?;
    calc_button.set_text_content(Some("Calculate"));
    calc_button.set_id("calc-btn");
    container.append_child(&calc_button)?;

    // Result display
    let result = document.create_element("div")?;
    result.set_id("result");
    result.set_text_content(Some("Result: "));
    container.append_child(&result)?;

    // History display
    let history_title = document.create_element("h3")?;
    history_title.set_text_content(Some("History"));
    container.append_child(&history_title)?;

    let history = document.create_element("ul")?;
    history.set_id("history");
    container.append_child(&history)?;

    body.append_child(&container)?;

    // Set up calculator instance
    let calc = Calculator::new();
    let calc = std::rc::Rc::new(std::cell::RefCell::new(calc));

    // Attach event handler
    if let Some(button) = document.get_element_by_id("calc-btn") {
        let document_clone = document.clone();
        let calc_clone = calc.clone();

        let onclick = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            // Get inputs
            let num1_el = document_clone.get_element_by_id("num1")
                .and_then(|el| el.dyn_into::<HtmlInputElement>().ok());
            let num2_el = document_clone.get_element_by_id("num2")
                .and_then(|el| el.dyn_into::<HtmlInputElement>().ok());
            let operator_el = document_clone.get_element_by_id("operator")
                .and_then(|el| el.dyn_into::<web_sys::HtmlSelectElement>().ok());

            if let (Some(num1_input), Some(num2_input), Some(op_select)) = (num1_el, num2_el, operator_el) {
                // Parse numbers
                let num1 = num1_input.value().parse::<f64>().unwrap_or(0.0);
                let num2 = num2_input.value().parse::<f64>().unwrap_or(0.0);
                let operator = op_select.value();

                // Perform calculation
                let result_value = match operator.as_str() {
                    "+" => Ok(calc_clone.borrow().add(num1, num2)),
                    "-" => Ok(calc_clone.borrow().subtract(num1, num2)),
                    "*" => Ok(calc_clone.borrow().multiply(num1, num2)),
                    "/" => calc_clone.borrow().divide(num1, num2),
                    _ => Err("Unknown operator".to_string()),
                };

                // Display result
                if let Some(result_el) = document_clone.get_element_by_id("result") {
                    match result_value {
                        Ok(val) => {
                            result_el.set_text_content(Some(&format!("Result: {}", val)));

                            // Add to history
                            let history_entry = format!("{} {} {} = {}", num1, operator, num2, val);
                            calc_clone.borrow_mut().add_to_history(history_entry.clone());

                            // Update history display
                            if let Some(history_el) = document_clone.get_element_by_id("history") {
                                let li = document_clone.create_element("li").unwrap();
                                li.set_text_content(Some(&history_entry));
                                history_el.append_child(&li).unwrap();
                            }
                        },
                        Err(e) => {
                            result_el.set_text_content(Some(&format!("Error: {}", e)));
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        button.set_onclick(Some(onclick.as_ref().unchecked_ref()));
        onclick.forget();
    }

    web_sys::console::log_1(&"Calculator initialized!".into());

    Ok(())
}
```

**What this demonstrates:**
- **Complete WASM app** - From initialization to user interaction
- **State management** - `Rc<RefCell<Calculator>>` for shared mutable state
- **DOM manipulation** - Creating and manipulating elements from Rust
- **Event handling** - Closure with captured state
- **Error handling** - Division by zero, invalid inputs
- **Type conversions** - Rust ↔ JavaScript number/string conversions

**To run:**
1. Create a new project: `cargo new --lib wasm-calculator`
2. Add dependencies to `Cargo.toml`:
   ```toml
   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   wasm-bindgen = "0.2"
   console_error_panic_hook = "0.1"
   js-sys = "0.3"

   [dependencies.web-sys]
   version = "0.3"
   features = ["Window", "Document", "HtmlElement", "HtmlInputElement", "HtmlSelectElement", "MouseEvent"]
   ```
3. Build: `wasm-pack build --target web`
4. Create `index.html`:
   ```html
   <!DOCTYPE html>
   <html>
     <head>
       <title>WASM Calculator</title>
     </head>
     <body>
       <script type="module">
         import init from './pkg/wasm_calculator.js';
         await init();
       </script>
     </body>
   </html>
   ```
5. Serve with a local server: `python3 -m http.server`

**Type Conversion Flow:**

```
Rust Side          wasm-bindgen          JavaScript Side
─────────────────────────────────────────────────────────
f64 (num1)    ─→  [serialize]  ─→  Number (num1)
String (op)   ─→  [serialize]  ─→  String (op)
Calculator    ─→  [wrap]       ─→  Calculator object

User clicks   ←─  [event]     ←─  MouseEvent
HtmlElement   ←─  [bind]      ←─  DOM element
Result        ─→  [serialize]  ─→  Displays in DOM
```

**Key Pattern:** `Rc<RefCell<>>` allows multiple closures to share mutable state. The `Rc` provides shared ownership, `RefCell` provides interior mutability (runtime borrow checking).

## How dialect-coach Uses wasm-bindgen

### 1. Closure Memory Management (speech.rs:66-67, 213-217)

Pattern for keeping closures alive:

```rust
let closure = Closure::wrap(Box::new(move || {
    // Voice loading callback
    let available_voices = synth_clone2.get_voices();
    let mut voices_vec = Vec::new();
    for i in 0..available_voices.length() {
        if let Some(voice) = available_voices.get(i) {
            voices_vec.push(voice);
        }
    }
    *voices_clone2.borrow_mut() = voices_vec;
    *voices_loaded_clone2.borrow_mut() = true;
}) as Box<dyn FnMut()>);

synth.set_onvoiceschanged(Some(closure.as_ref().unchecked_ref()));
closure.forget();  // Keep alive for lifetime of synth
```

**Why `.forget()`:** Closures passed to JavaScript must not be dropped. `.forget()` leaks the memory intentionally because JavaScript holds the reference.

### 2. Type Casting with JsCast (speech.rs:303-318)

Safe runtime type checking for browser compatibility:

```rust
// Try to get SpeechRecognition (standard or webkit prefix)
let recognition = if let Some(constructor) =
    js_sys::Reflect::get(&window, &JsValue::from_str("SpeechRecognition"))
        .ok()
        .and_then(|val| val.dyn_into::<js_sys::Function>().ok())
{
    // Standard API exists
    js_sys::Reflect::construct(&constructor, &js_sys::Array::new())
        .ok()
        .and_then(|val| val.dyn_into::<web_sys::SpeechRecognition>().ok())
        .ok_or("Failed to construct SpeechRecognition")?
} else if let Some(webkit_constructor) =
    js_sys::Reflect::get(&window, &JsValue::from_str("webkitSpeechRecognition"))
        .ok()
        .and_then(|val| val.dyn_into::<js_sys::Function>().ok())
{
    // Fallback to webkit-prefixed API
    js_sys::Reflect::construct(&webkit_constructor, &js_sys::Array::new())
        .ok()
        .and_then(|val| val.dyn_into::<web_sys::SpeechRecognition>().ok())
        .ok_or("Failed to construct webkitSpeechRecognition")?
} else {
    return Err("SpeechRecognition API not supported".to_string());
};
```

**Pattern:** Use `dyn_into` for safe type casting with proper error handling.

### 3. Spawning Async Tasks (websocket.rs:6, 180-188)

Using `spawn_local` for async work in WASM:

```rust
use wasm_bindgen_futures::spawn_local;

// Spawn send task for WebSocket
spawn_local(async move {
    while let Some(text) = rx.next().await {
        if let Err(e) = write.send(WsMessage::Text(text)).await {
            error!("Failed to send message: {:?}", e);
            break;
        }
    }
    info!("Send task terminated");
});
```

**Pattern:** `spawn_local` runs async tasks without blocking, similar to `tokio::spawn` but for WASM.

### 4. Event Callback with Yew Integration (speech.rs:213-224)

Bridging web-sys events to Yew callbacks:

```rust
if let Some(callback) = on_start {
    let closure = Closure::wrap(Box::new(move |_event: web_sys::SpeechSynthesisEvent| {
        callback.emit(());  // Call Yew callback
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
```

**Pattern:** Wrap Yew callbacks inside wasm-bindgen closures to bridge browser events to application state.

### 5. Error Event Handling (speech.rs:226-234)

Extracting error information from events:

```rust
if let Some(callback) = on_error {
    let closure = Closure::wrap(Box::new(move |event: SpeechSynthesisErrorEvent| {
        // Extract error from event
        let error_msg = format!("Speech synthesis error: {:?}", event.type_());
        callback.emit(error_msg);
    }) as Box<dyn FnMut(_)>);
    utterance.set_onerror(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
}
```

**Pattern:** Convert JavaScript errors to Rust strings for application-level error handling.

### 6. Reflection for Runtime Inspection (speech.rs:335-337)

Checking for API availability:

```rust
pub fn is_supported() -> bool {
    if let Some(window) = web_sys::window() {
        js_sys::Reflect::has(&window, &JsValue::from_str("SpeechRecognition"))
            .unwrap_or(false) ||
        js_sys::Reflect::has(&window, &JsValue::from_str("webkitSpeechRecognition"))
            .unwrap_or(false)
    } else {
        false
    }
}
```

**Pattern:** Use `js_sys::Reflect` to inspect JavaScript objects at runtime for feature detection.

## Common Patterns

### Pattern 1: Closure Template

```rust
let closure = Closure::wrap(Box::new(move |arg: EventType| {
    // Handle event
}) as Box<dyn FnMut(EventType)>);

api.set_callback(Some(closure.as_ref().unchecked_ref()));
closure.forget();
```

**Components:**
1. `Closure::wrap` - Wraps Rust closure for JavaScript
2. `Box::new(move |..| {})` - Heap-allocated closure
3. `as Box<dyn FnMut(EventType)>` - Type annotation
4. `.as_ref().unchecked_ref()` - Get JavaScript reference
5. `.forget()` - Prevent Rust from dropping

### Pattern 2: Optional Callback

```rust
fn setup_callbacks(on_event: Option<Callback<String>>) {
    if let Some(callback) = on_event {
        let closure = Closure::wrap(Box::new(move |data: String| {
            callback.emit(data);
        }) as Box<dyn FnMut(String)>);

        // Attach closure...
        closure.forget();
    }
}
```

### Pattern 3: JS Object Creation

```rust
use js_sys::Object;

fn create_options() -> Object {
    let obj = Object::new();
    js_sys::Reflect::set(
        &obj,
        &"method".into(),
        &"GET".into(),
    ).unwrap();
    js_sys::Reflect::set(
        &obj,
        &"headers".into(),
        &create_headers(),
    ).unwrap();
    obj
}
```

### Pattern 4: Array Conversion

```rust
// Rust Vec to JS Array
fn vec_to_array(vec: Vec<String>) -> js_sys::Array {
    vec.into_iter()
        .map(|s| JsValue::from_str(&s))
        .collect()
}

// JS Array to Rust Vec
fn array_to_vec(array: &js_sys::Array) -> Vec<String> {
    (0..array.length())
        .filter_map(|i| {
            array.get(i).as_string()
        })
        .collect()
}
```

## Best Practices from dialect-coach

1. **Always `.forget()` event handler closures** - They must outlive their registration
2. **Use `dyn_into` for safe casting** - Prefer over `unchecked_into` except when type is guaranteed
3. **Feature detection over assumptions** - Check API availability before use
4. **Handle vendor prefixes** - Support webkit/moz prefixes for broader compatibility
5. **Extract error messages** - Convert JS errors to Rust types for consistent error handling
6. **Clone before move into closure** - Satisfy Rust ownership when capturing environment
7. **Use `spawn_local` for async** - Don't block the main thread in WASM

## Troubleshooting

### Issue: "Cannot call closure after being moved"

**Cause:** Trying to use closure after passing to JavaScript

**Solution:** Clone what you need before creating closure:

```rust
let data = data.clone();  // Clone before move
let closure = Closure::wrap(Box::new(move || {
    use_data(&data);  // Now we own a clone
}) as Box<dyn FnMut()>);
```

### Issue: "Memory leak detected"

**Cause:** Creating closures without proper cleanup

**Solution:** Only `.forget()` closures that must live forever. For temporary callbacks, store and drop:

```rust
struct Handler {
    closure: Closure<dyn FnMut()>,
}

impl Drop for Handler {
    fn drop(&mut self) {
        // Closure will be cleaned up
    }
}
```

### Issue: "Failed to downcast JsValue"

**Cause:** Incorrect type assumption

**Solution:** Check type before casting:

```rust
if result.is_string() {
    let s = result.as_string().unwrap();
} else if result.is_array() {
    let arr = js_sys::Array::from(&result);
}
```

### Issue: "panic in async function"

**Cause:** Unhandled panic in WASM

**Solution:** Use `console_error_panic_hook`:

```rust
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}
```

## Further Resources

- **Official Guide:** https://rustwasm.github.io/wasm-bindgen/
- **Crate:** https://crates.io/crates/wasm-bindgen
- **API Docs:** https://docs.rs/wasm-bindgen/latest/wasm_bindgen/
- **Examples:** https://rustwasm.github.io/wasm-bindgen/examples/index.html
- **Book:** https://rustwasm.github.io/book/

## Summary

wasm-bindgen enables seamless Rust/JavaScript interop:

- **`#[wasm_bindgen]`** exposes Rust to JavaScript
- **Closures** bridge Rust callbacks to JS events
- **`.forget()`** keeps event handlers alive
- **JsCast** enables safe type conversions
- **`spawn_local`** runs async tasks in WASM
- **js-sys** provides JavaScript standard library
- **Feature detection** ensures browser compatibility

The dialect-coach project demonstrates production patterns including event handling, async operations, browser API compatibility, and clean integration with Yew for reactive UIs.
