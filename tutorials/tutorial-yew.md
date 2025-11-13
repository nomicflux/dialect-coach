# Yew Tutorial: Component-Based Web UIs in Rust

## Overview

**Yew** is a modern Rust framework for building multi-threaded front-end web applications using WebAssembly. It follows a component-based architecture similar to React, with hooks for state management and side effects.

**Version used in dialect-coach:** `0.21` with CSR (Client-Side Rendering) feature

**Why we use it:** Yew enables building type-safe, performant web UIs entirely in Rust, sharing code between frontend and backend without JavaScript.

## Dependencies

```toml
[dependencies]
yew = { version = "0.21", features = ["csr"] }
```

## Core Concepts

### 1. Function Components

Yew uses function components with the `#[function_component]` attribute:

```rust
use yew::prelude::*;

#[function_component(MyComponent)]
fn my_component() -> Html {
    html! {
        <div>{"Hello, Yew!"}</div>
    }
}
```

### 2. The `html!` Macro

Yew's JSX-like syntax for building UI:

```rust
html! {
    <div class="container">
        <h1>{"Title"}</h1>
        <button onclick={callback}>{"Click me"}</button>
    </div>
}
```

### 3. Hooks for State Management

- **`use_state`** - Simple state (like React's useState)
- **`use_reducer`** - Complex state with actions (like React's useReducer)
- **`use_effect_with`** - Side effects on mount/dependency changes

### 4. Props and Callbacks

Components communicate via props and callbacks:

```rust
#[derive(Properties, PartialEq)]
struct ChildProps {
    message: String,
    on_click: Callback<()>,
}

#[function_component(Child)]
fn child(props: &ChildProps) -> Html {
    let onclick = {
        let callback = props.on_click.clone();
        Callback::from(move |_| callback.emit(()))
    };

    html! {
        <button onclick={onclick}>{&props.message}</button>
    }
}
```

## Understanding State Management

### When to Use `use_state` vs `use_reducer`

**Use `use_state` when:**
- You have simple, independent pieces of state (a counter, a toggle, a single string)
- State updates are straightforward value replacements
- You don't need to coordinate multiple related state changes

**Use `use_reducer` when:**
- You have complex state with multiple related fields
- Multiple actions can modify the same state (Add, Update, Delete, etc.)
- You want centralized state update logic
- You need predictable state transitions (each action → new state)

**Example comparison:**

```rust
// Simple state: use_state is perfect
let count = use_state(|| 0);
count.set(*count + 1);

// Complex state: use_reducer is better
let todos = use_reducer(TodoState::default);
todos.dispatch(TodoAction::Add("Buy milk".to_string()));
todos.dispatch(TodoAction::Toggle(5));
todos.dispatch(TodoAction::Remove(3));
```

**Why `use_reducer` helps:** With a reducer, all state changes go through one function (`reduce`). This makes it easier to understand how state changes, easier to test, and prevents accidental inconsistent state.

### Rust Ownership in Closures

Yew callbacks use Rust closures, which must satisfy ownership rules. This is why you see the "clone before move" pattern everywhere:

```rust
// This DOESN'T work:
let count = use_state(|| 0);
Callback::from(move |_| {
    count.set(*count + 1);  // ERROR: count moved into closure
});
// Can't use count here anymore!

// This WORKS:
let count = use_state(|| 0);
let count_clone = count.clone();  // Clone the handle (cheap!)
Callback::from(move |_| {
    count_clone.set(*count_clone + 1);  // Moves the clone, not the original
});
// count is still usable here
```

**What's being cloned?** Not the actual state! `use_state` returns a handle (like `Rc` internally). Cloning the handle is cheap—it just increments a reference count. The actual state stays in one place.

**Why is this necessary?** Rust's ownership rules prevent the same value from being owned by multiple closures. By cloning the handle first, each closure gets its own handle to the same underlying state.

## Step-by-Step: Building a Todo List

Let's build an interactive todo list to learn Yew's key patterns.

### Step 1: Project Setup

```bash
cargo new yew-todo
cd yew-todo
```

Add to `Cargo.toml`:

```toml
[package]
name = "yew-todo"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
yew = { version = "0.21", features = ["csr"] }
wasm-bindgen = "0.2"
web-sys = "0.3"
```

### Step 2: Basic Counter (use_state)

Create `src/lib.rs`:

```rust
use yew::prelude::*;

#[function_component(Counter)]
fn counter() -> Html {
    // State hook - returns (value, setter)
    let count = use_state(|| 0);

    // Event handler with closure
    let increment = {
        let count = count.clone();
        Callback::from(move |_| {
            count.set(*count + 1);
        })
    };

    let decrement = {
        let count = count.clone();
        Callback::from(move |_| {
            count.set(*count - 1);
        })
    };

    html! {
        <div>
            <h1>{"Counter: "}{*count}</h1>
            <button onclick={increment}>{"+"}</button>
            <button onclick={decrement}>{"-"}</button>
        </div>
    }
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <Counter />
    }
}
```

**Key Pattern:** Clone the state handle before moving into callbacks.

**Why the Clone?** Look at line 119-120: `let count = count.clone();` This creates a new handle to the same state. The `move` keyword on line 121 transfers ownership of `count` (the clone) into the closure. Without the clone, we'd transfer ownership of the original `count`, making it unusable elsewhere. The clone is cheap—just incrementing a reference counter, not copying the actual number.

### Step 2.5: Form with Multiple Inputs

Before jumping to the complex todo list, let's build a form to practice handling multiple pieces of state:

```rust
use yew::prelude::*;
use web_sys::HtmlInputElement;

#[function_component(UserForm)]
fn user_form() -> Html {
    let name = use_state(String::new);
    let email = use_state(String::new);
    let age = use_state(|| 0u32);
    let message = use_state(|| Option::<String>::None);

    // Handle name input
    let on_name_change = {
        let name = name.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                name.set(input.value());
            }
        })
    };

    // Handle email input
    let on_email_change = {
        let email = email.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                email.set(input.value());
            }
        })
    };

    // Handle age input
    let on_age_change = {
        let age = age.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                if let Ok(parsed_age) = input.value().parse::<u32>() {
                    age.set(parsed_age);
                }
            }
        })
    };

    // Handle form submission
    let on_submit = {
        let name = name.clone();
        let email = email.clone();
        let age = age.clone();
        let message = message.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            // Create summary message
            let summary = format!(
                "Name: {}, Email: {}, Age: {}",
                *name, *email, *age
            );
            message.set(Some(summary));
        })
    };

    html! {
        <div class="user-form">
            <h2>{"User Registration"}</h2>

            <form onsubmit={on_submit}>
                <div>
                    <label>{"Name: "}</label>
                    <input
                        type="text"
                        value={(*name).clone()}
                        oninput={on_name_change}
                        placeholder="Enter your name"
                    />
                </div>

                <div>
                    <label>{"Email: "}</label>
                    <input
                        type="email"
                        value={(*email).clone()}
                        oninput={on_email_change}
                        placeholder="you@example.com"
                    />
                </div>

                <div>
                    <label>{"Age: "}</label>
                    <input
                        type="number"
                        value={age.to_string()}
                        oninput={on_age_change}
                    />
                </div>

                <button type="submit">{"Submit"}</button>
            </form>

            {if let Some(msg) = (*message).as_ref() {
                html! { <p class="message">{msg}</p> }
            } else {
                html! {}
            }}
        </div>
    }
}
```

**Key Patterns:**
- **Multiple independent states** - Each form field gets its own `use_state`. This is simpler than one big state object.
- **Input binding** - The `value` attribute binds input to state, `oninput` updates state.
- **Type conversion** - Parse string input to `u32` for the age field.
- **Conditional rendering** - Only show the message if `Some(msg)` exists.

This pattern works well for forms with independent fields. For more complex state (like a todo list where items relate to each other), use a reducer instead.

### Step 3: Todo List with Reducer

```rust
use yew::prelude::*;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
struct Todo {
    id: usize,
    text: String,
    completed: bool,
}

#[derive(Clone, PartialEq)]
struct TodoState {
    todos: Vec<Todo>,
    next_id: usize,
}

impl Default for TodoState {
    fn default() -> Self {
        Self {
            todos: Vec::new(),
            next_id: 1,
        }
    }
}

enum TodoAction {
    Add(String),
    Toggle(usize),
    Remove(usize),
}

impl Reducible for TodoState {
    type Action = TodoAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            TodoAction::Add(text) => {
                let mut todos = self.todos.clone();
                todos.push(Todo {
                    id: self.next_id,
                    text,
                    completed: false,
                });
                Rc::new(Self {
                    todos,
                    next_id: self.next_id + 1,
                })
            }
            TodoAction::Toggle(id) => {
                let mut todos = self.todos.clone();
                if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
                    todo.completed = !todo.completed;
                }
                Rc::new(Self {
                    todos,
                    next_id: self.next_id,
                })
            }
            TodoAction::Remove(id) => {
                let todos = self.todos.iter()
                    .filter(|t| t.id != id)
                    .cloned()
                    .collect();
                Rc::new(Self {
                    todos,
                    next_id: self.next_id,
                })
            }
        }
    }
}

#[function_component(TodoApp)]
fn todo_app() -> Html {
    let state = use_reducer(TodoState::default);
    let input_value = use_state(String::new);

    // Add todo handler
    let on_submit = {
        let state = state.clone();
        let input_value = input_value.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if !input_value.is_empty() {
                state.dispatch(TodoAction::Add((*input_value).clone()));
                input_value.set(String::new());
            }
        })
    };

    // Input change handler
    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                input_value.set(input.value());
            }
        })
    };

    html! {
        <div class="todo-app">
            <h1>{"Todo List"}</h1>

            <form onsubmit={on_submit}>
                <input
                    type="text"
                    value={(*input_value).clone()}
                    oninput={on_input}
                    placeholder="What needs to be done?"
                />
                <button type="submit">{"Add"}</button>
            </form>

            <ul>
                {for state.todos.iter().map(|todo| {
                    let toggle = {
                        let state = state.clone();
                        let id = todo.id;
                        Callback::from(move |_| {
                            state.dispatch(TodoAction::Toggle(id));
                        })
                    };

                    let remove = {
                        let state = state.clone();
                        let id = todo.id;
                        Callback::from(move |_| {
                            state.dispatch(TodoAction::Remove(id));
                        })
                    };

                    html! {
                        <li key={todo.id}>
                            <input
                                type="checkbox"
                                checked={todo.completed}
                                onclick={toggle}
                            />
                            <span style={if todo.completed {
                                "text-decoration: line-through"
                            } else {
                                ""
                            }}>
                                {&todo.text}
                            </span>
                            <button onclick={remove}>{"×"}</button>
                        </li>
                    }
                })}
            </ul>
        </div>
    }
}
```

**Key Patterns:**
- Use `use_reducer` for complex state with multiple actions
- Clone dispatcher before moving into callbacks
- Use `key` prop for list items
- `prevent_default()` on form submit

### Step 4: Side Effects with use_effect_with

```rust
use gloo::console::log;

#[function_component(EffectDemo)]
fn effect_demo() -> Html {
    let count = use_state(|| 0);

    // Run effect when count changes
    use_effect_with(*count, {
        let count = *count;
        move |_| {
            log!(&format!("Count changed to: {}", count));

            // Cleanup function (runs before next effect or unmount)
            || {
                log!("Cleaning up previous effect");
            }
        }
    });

    let increment = {
        let count = count.clone();
        Callback::from(move |_| count.set(*count + 1))
    };

    html! {
        <div>
            <p>{"Count: "}{*count}</p>
            <button onclick={increment}>{"Increment"}</button>
        </div>
    }
}
```

## How dialect-coach Uses Yew

### 1. Main App Component (frontend/src/app.rs:45-345)

The `App` component manages the entire application:

```rust
#[function_component(App)]
pub fn app() -> Html {
    // Multiple state hooks for different concerns
    let selected_language = use_state(|| Language::Spanish);
    let selected_dialect = use_state(|| Dialect::SpanishMexican);
    let formality = use_state(|| Formality::Casual);
    let teaching_mode = use_state(|| TeachingMode::Immersive);
    let session_id = use_state(|| Uuid::new_v4());
    let messages = use_reducer(MessagesState::default);
    let connection_state = use_state(|| ConnectionState::Disconnected);
    let is_loading = use_state(|| false);
    let error_message = use_state(|| Option::<String>::None);

    // WebSocket service wrapped in Rc<RefCell<>> for interior mutability
    let ws_service = use_state(|| Rc::new(RefCell::new(
        WebSocketService::new("ws://localhost:3000/ws")
    )));

    // ... component logic
}
```

**Pattern:** Multiple `use_state` hooks for independent UI concerns.

### 2. Reducer for Messages (frontend/src/app.rs:11-43)

Complex message state uses a reducer:

```rust
#[derive(Clone, PartialEq)]
struct MessagesState {
    messages: Vec<Message>,
}

impl Reducible for MessagesState {
    type Action = MessagesAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            MessagesAction::Add(msg) => {
                let mut messages = self.messages.clone();
                messages.push(msg);
                Rc::new(Self { messages })
            }
            MessagesAction::Clear => Rc::new(Self::default()),
        }
    }
}

enum MessagesAction {
    Add(Message),
    Clear,
}
```

**Why:** Ensures immutable updates and clean action-based state changes.

### 3. Effect for WebSocket Initialization (frontend/src/app.rs:66-135)

WebSocket connects on component mount:

```rust
use_effect_with((), move |_| {
    info!("Initializing WebSocket connection");

    {
        let mut ws = ws_service.borrow_mut();

        // Set up callbacks
        ws.set_on_open(Callback::from(move |_| {
            info!("WebSocket opened");
        }));

        let error_message_clone = error_message.clone();
        ws.set_on_close(Callback::from(move |_| {
            error_message_clone.set(Some("Connection closed".to_string()));
        }));

        // More callbacks...

        ws.connect();
    }

    // Cleanup on unmount
    let ws_service_clone = ws_service.clone();
    move || {
        info!("Disconnecting WebSocket");
        ws_service_clone.borrow_mut().disconnect();
    }
});
```

**Pattern:** Effect with empty dependencies `()` runs only on mount, cleanup runs on unmount.

### 4. Event Handlers with Callbacks (frontend/src/app.rs:138-179)

Message sending callback:

```rust
let on_send_message = {
    let ws_service = ws_service.clone();
    let session_id = session_id.clone();
    let selected_dialect = selected_dialect.clone();
    let formality = formality.clone();
    let teaching_mode = teaching_mode.clone();
    let messages = messages.clone();
    let is_loading = is_loading.clone();
    let error_message = error_message.clone();

    Callback::from(move |content: String| {
        // Create message
        let mut msg = Message::new(
            *session_id,
            "user".to_string(),
            content,
            (*selected_dialect).name().to_string(),
        );

        msg.metadata.formality = Some(*formality);
        msg.metadata.teaching_mode = Some(*teaching_mode);

        // Add to local state
        messages.dispatch(MessagesAction::Add(msg.clone()));

        // Send via WebSocket
        match ws_service.borrow().send_message(&msg) {
            Ok(_) => {
                is_loading.set(true);
                error_message.set(None);
            }
            Err(e) => {
                error_message.set(Some(format!("Failed to send: {}", e)));
            }
        }
    })
}
```

**Pattern:** Clone all needed state before creating callback, use `Rc<RefCell<>>` for interior mutability.

### 5. Dynamic Select Options (frontend/src/app.rs:294-304)

Rendering dialect options based on selected language:

```rust
<select onchange={on_dialect_change}>
    {for Dialect::for_language(*selected_language).iter().map(|dialect| {
        let is_selected = *dialect == *selected_dialect;
        html! {
            <option value={dialect.id()} selected={is_selected}>
                {dialect.name()}
            </option>
        }
    })}
</select>
```

**Pattern:** Use `{for iterator.map(...)}` to render dynamic lists.

### 6. Conditional Rendering (frontend/src/app.rs:329-337)

Error banner appears conditionally:

```rust
{if let Some(err) = (*error_message).as_ref() {
    html! {
        <div class="error-banner">
            {format!("Error: {}", err)}
        </div>
    }
} else {
    html! {}
}}
```

**Pattern:** Use if-else expressions to conditionally render components.

### 7. Component Composition (frontend/src/app.rs:340-341)

Passing state to child components:

```rust
<ChatWindow messages={messages.messages.clone()} is_loading={*is_loading} />
<InputBox on_send={on_send_message} disabled={!matches!(*connection_state, ConnectionState::Connected)} />
```

**Pattern:** Child components receive data via props and communicate back via callbacks.

## Common Patterns

### Pattern 1: Controlled Inputs

```rust
let input_value = use_state(String::new);

let oninput = {
    let input_value = input_value.clone();
    Callback::from(move |e: InputEvent| {
        if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
            input_value.set(input.value());
        }
    })
};

html! {
    <input type="text" value={(*input_value).clone()} oninput={oninput} />
}
```

### Pattern 2: Preventing Default Events

```rust
let onsubmit = Callback::from(|e: SubmitEvent| {
    e.prevent_default();
    // Handle form submission
});
```

### Pattern 3: Conditional Classes

```rust
html! {
    <div class={classes!(
        "base-class",
        is_active.then_some("active"),
        is_error.then_some("error"),
    )}>
        {"Content"}
    </div>
}
```

### Pattern 4: Async Operations in Effects

```rust
use_effect_with((), |_| {
    wasm_bindgen_futures::spawn_local(async move {
        // Async work here
        let data = fetch_data().await;
        // Update state
    });

    || () // Cleanup
});
```

## Best Practices from dialect-coach

1. **Separate concerns with multiple state hooks** rather than one giant state object
2. **Use `use_reducer` for complex state** with multiple related actions
3. **Clone state handles before callbacks** to satisfy Rust's ownership rules
4. **Use `Rc<RefCell<>>` for services** that need interior mutability
5. **Provide cleanup functions** in effects to prevent memory leaks
6. **Use `match` patterns** for state transitions (ConnectionState example)
7. **Keep components focused** - compose larger UIs from smaller components

## Troubleshooting

### Issue: "Value moved into closure"

**Solution:** Clone the value before moving into the closure:

```rust
let count = count.clone();
Callback::from(move |_| {
    count.set(*count + 1);
})
```

### Issue: Component doesn't re-render on state change

**Solution:** Ensure you're calling the setter, not mutating directly:

```rust
// Wrong
*my_state.modify().value = 10;

// Correct
my_state.set(10);
```

### Issue: Effect runs on every render

**Solution:** Specify dependencies with `use_effect_with`:

```rust
// Runs on every render
use_effect(|| { /* ... */ });

// Runs only when `count` changes
use_effect_with(*count, |_| { /* ... */ });

// Runs only on mount
use_effect_with((), |_| { /* ... */ });
```

## Further Resources

- **Official Docs:** https://yew.rs/docs/getting-started/introduction
- **Crate:** https://crates.io/crates/yew
- **GitHub:** https://github.com/yewstack/yew
- **Examples:** https://github.com/yewstack/yew/tree/master/examples
- **API Docs:** https://docs.rs/yew/latest/yew/

## Summary

Yew provides a React-like component model for building web UIs in Rust:

- **Function components** with `#[function_component]`
- **State management** via `use_state` and `use_reducer`
- **Side effects** with `use_effect_with`
- **Event handling** through callbacks
- **Type safety** across your entire frontend

The dialect-coach project demonstrates real-world patterns for complex applications including WebSocket integration, multiple state management approaches, and clean component composition.
