use yew::prelude::*;

pub fn render_message_undo_notification(deleted_count: usize, on_undo: Callback<()>) -> Html {
    if deleted_count > 0 {
        let onclick = Callback::from(move |_| on_undo.emit(()));
        html! {
            <div class="message-undo-notification">
                <span>{"Message deleted."}</span>
                <button class="undo-button" {onclick}>{"Undo"}</button>
            </div>
        }
    } else {
        html! {}
    }
}
