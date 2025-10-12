use yew::prelude::*;

#[function_component(ChatWindow)]
pub fn chat_window() -> Html {
    html! {
        <div class="chat-window">
            <p>{"Chat window component"}</p>
        </div>
    }
}
