use crate::components::TranslateSelectionButton;
use dialect_coach_shared::models::dialect::dialect_features;
use dialect_coach_shared::models::{Language, LanguageOption, Message};
use uuid::Uuid;
use web_sys::{window, MouseEvent};
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct TextSelection {
    text: String,
    position: (f64, f64),
}

#[derive(Properties, PartialEq)]
pub struct MessageBubbleProps {
    pub message: Message,
    pub is_own_message: bool,
    #[prop_or_default]
    pub on_replay: Option<Callback<Message>>,
    #[prop_or_default]
    pub on_delete: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub on_create_branch: Option<Callback<Uuid>>,
    #[prop_or(false)]
    pub has_child_branches: bool,
    #[prop_or_default]
    pub on_explain: Option<Callback<Uuid>>,
    #[prop_or_default]
    pub language_option: Option<LanguageOption>,
    #[prop_or(false)]
    pub is_explain_loading: bool,
}

fn render_delete_button(on_delete: &Option<Callback<Uuid>>, msg_id: Uuid) -> Html {
    if let Some(callback) = on_delete {
        let cb = callback.clone();
        let onclick = Callback::from(move |_| cb.emit(msg_id));
        html! {
            <button class="message-delete-button" {onclick} title="Delete message">{"×"}</button>
        }
    } else {
        html! {}
    }
}

fn render_replay_button(on_replay: &Option<Callback<Message>>, msg: &Message) -> Html {
    if let Some(callback) = on_replay {
        let dialect = msg.metadata.dialect;
        if !dialect_features(dialect).has_tts() {
            return html! {};
        }
        let cb = callback.clone();
        let m = msg.clone();
        let onclick = Callback::from(move |_| cb.emit(m.clone()));
        html! {
            <button class="replay-button" {onclick} title="Replay audio">{"🔊"}</button>
        }
    } else {
        html! {}
    }
}

fn get_css_classes(is_own: bool) -> (&'static str, &'static str, &'static str, &'static str) {
    if is_own {
        (
            "message message--user",
            "avatar avatar--user",
            "bubble bubble--user",
            "U",
        )
    } else {
        (
            "message message--bot",
            "avatar avatar--bot",
            "bubble bubble--bot",
            "🤖",
        )
    }
}

fn language_code(lang: Language) -> &'static str {
    match lang {
        Language::Spanish => "es",
        Language::Arabic => "ar",
        Language::French => "fr",
        Language::English => "en",
        Language::Japanese => "ja",
    }
}

fn font_class(lang_option: &Option<LanguageOption>) -> &'static str {
    match lang_option {
        Some(LanguageOption::Arabic(script)) => {
            use dialect_coach_shared::models::ArabicScript;
            match script {
                ArabicScript::Naskh => "arabic-naskh",
                ArabicScript::Ruqa => "arabic-ruqa",
                ArabicScript::Latin => "arabic-latin",
            }
        }
        _ => "",
    }
}

enum TextSegment {
    Plain(String),
    Ruby { base: String, reading: String },
}

fn parse_ruby_text(text: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();
    let mut current_pos = 0;

    while let Some(ruby_start) = text[current_pos..].find("<ruby>") {
        let abs_start = current_pos + ruby_start;
        if abs_start > current_pos {
            segments.push(TextSegment::Plain(text[current_pos..abs_start].to_string()));
        }

        if let Some(rt_start) = text[abs_start..].find("<rt>") {
            let abs_rt_start = abs_start + rt_start;
            let base = text[abs_start + 6..abs_rt_start].to_string();

            if let Some(rt_end) = text[abs_rt_start..].find("</rt>") {
                let abs_rt_end = abs_rt_start + rt_end;
                let reading = text[abs_rt_start + 4..abs_rt_end].to_string();

                if let Some(ruby_end) = text[abs_rt_end..].find("</ruby>") {
                    let abs_ruby_end = abs_rt_end + ruby_end;
                    segments.push(TextSegment::Ruby { base, reading });
                    current_pos = abs_ruby_end + 7; // Skip past </ruby>
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if current_pos < text.len() {
        segments.push(TextSegment::Plain(text[current_pos..].to_string()));
    }

    segments
}

fn render_text_with_ruby(text: &str) -> Html {
    let segments = parse_ruby_text(text);
    html! {
        <>
            { for segments.iter().map(|seg| match seg {
                TextSegment::Plain(s) => html! { <>{s}</> },
                TextSegment::Ruby { base, reading } => html! {
                    <ruby>{base}<rt>{reading}</rt></ruby>
                },
            })}
        </>
    }
}

fn get_text_selection() -> Option<TextSelection> {
    let window = window()?;
    let document = window.document()?;
    let selection = document.get_selection().ok()??;

    if selection.is_collapsed() {
        return None;
    }

    let text = selection.to_string().as_string()?;
    if text.trim().is_empty() {
        return None;
    }

    let range = selection.get_range_at(0).ok()?;
    let rect = range.get_bounding_client_rect();

    Some(TextSelection {
        text,
        position: (rect.right(), rect.bottom()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ruby_text_plain_only() {
        let segments = parse_ruby_text("Hello world");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::Plain(s) => assert_eq!(s, "Hello world"),
            _ => panic!("Expected plain text"),
        }
    }

    #[test]
    fn test_parse_ruby_text_single_ruby() {
        let segments = parse_ruby_text("<ruby>漢字<rt>かんじ</rt></ruby>");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::Ruby { base, reading } => {
                assert_eq!(base, "漢字");
                assert_eq!(reading, "かんじ");
            }
            _ => panic!("Expected ruby segment"),
        }
    }

    #[test]
    fn test_parse_ruby_text_mixed() {
        let text = "Hello <ruby>世界<rt>せかい</rt></ruby>!";
        let segments = parse_ruby_text(text);
        assert_eq!(segments.len(), 3);

        match &segments[0] {
            TextSegment::Plain(s) => assert_eq!(s, "Hello "),
            _ => panic!("Expected plain text"),
        }

        match &segments[1] {
            TextSegment::Ruby { base, reading } => {
                assert_eq!(base, "世界");
                assert_eq!(reading, "せかい");
            }
            _ => panic!("Expected ruby segment"),
        }

        match &segments[2] {
            TextSegment::Plain(s) => assert_eq!(s, "!"),
            _ => panic!("Expected plain text"),
        }
    }

    #[test]
    fn test_parse_ruby_text_multiple_ruby() {
        let text = "<ruby>今日<rt>きょう</rt></ruby>は<ruby>何<rt>なに</rt></ruby>";
        let segments = parse_ruby_text(text);
        assert_eq!(segments.len(), 3);

        match &segments[0] {
            TextSegment::Ruby { base, reading } => {
                assert_eq!(base, "今日");
                assert_eq!(reading, "きょう");
            }
            _ => panic!("Expected ruby segment"),
        }

        match &segments[1] {
            TextSegment::Plain(s) => assert_eq!(s, "は"),
            _ => panic!("Expected plain text"),
        }

        match &segments[2] {
            TextSegment::Ruby { base, reading } => {
                assert_eq!(base, "何");
                assert_eq!(reading, "なに");
            }
            _ => panic!("Expected ruby segment"),
        }
    }
}

fn render_branch_button(
    on_create_branch: &Option<Callback<Uuid>>,
    msg_id: Uuid,
    has_children: bool,
) -> Html {
    if let Some(callback) = on_create_branch {
        let cb = callback.clone();
        let onclick = Callback::from(move |_| cb.emit(msg_id));
        let title = if has_children {
            "Branch from here (has existing branches)"
        } else {
            "Branch from here"
        };
        let class = if has_children {
            "branch-button branch-button--has-children"
        } else {
            "branch-button"
        };
        html! {
            <button {class} {onclick} {title}>{"🌿"}</button>
        }
    } else {
        html! {}
    }
}

fn render_action_buttons(
    on_explain: &Option<Callback<Uuid>>,
    msg_id: Uuid,
    is_explain_loading: bool,
) -> Html {
    if on_explain.is_none() {
        return html! {};
    }

    html! {
        <div class="message-actions">
            {render_explain_button(on_explain, msg_id, is_explain_loading)}
        </div>
    }
}

fn render_explain_button(on_explain: &Option<Callback<Uuid>>, msg_id: Uuid, is_loading: bool) -> Html {
    if let Some(callback) = on_explain {
        let cb = callback.clone();
        let onclick = Callback::from(move |_| cb.emit(msg_id));
        let button_text = if is_loading { "Explaining..." } else { "Explain" };
        html! {
            <button class="action-button explain-button" {onclick} disabled={is_loading}>{button_text}</button>
        }
    } else {
        html! {}
    }
}

#[function_component(MessageBubble)]
pub fn message_bubble(props: &MessageBubbleProps) -> Html {
    let selection_state = use_state(|| None::<TextSelection>);

    let on_mouseup = {
        let selection_state = selection_state.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(selection) = get_text_selection() {
                selection_state.set(Some(selection));
            }
        })
    };

    let on_selection_translate = {
        let selection_state = selection_state.clone();
        Callback::from(move |(selected_text, context): (String, String)| {
            web_sys::console::log_1(&format!("Translate: {} (context: {})", selected_text, context).into());
            selection_state.set(None);
        })
    };

    let (msg_class, avatar_class, bubble_class, avatar_text) =
        get_css_classes(props.is_own_message);
    let lang = language_code(props.message.metadata.language);
    let font_class_name = font_class(&props.language_option);

    html! {
        <>
            <div class={msg_class}>
                <div class={avatar_class}>{avatar_text}</div>
                <div class={bubble_class}>
                    {render_delete_button(&props.on_delete, props.message.id)}
                    <div class="message-header">
                        <div class="message-author">{if props.message.is_agent() { "agent" } else { "user" }}</div>
                        {if !props.is_own_message { render_replay_button(&props.on_replay, &props.message) } else { html! {} }}
                    </div>
                    <div class={classes!("message-content", font_class_name)} {lang} onmouseup={on_mouseup}>
                        {render_text_with_ruby(&props.message.get_content())}
                    </div>
                    {if props.message.is_agent() {
                        render_action_buttons(&props.on_explain, props.message.id, props.is_explain_loading)
                    } else {
                        html! {}
                    }}
                    <div class="message-time">{props.message.metadata.timestamp.to_rfc3339()}</div>
                </div>
            </div>
            {render_branch_button(&props.on_create_branch, props.message.id, props.has_child_branches)}
            {
                if let Some(selection) = &*selection_state {
                    html! {
                        <TranslateSelectionButton
                            selected_text={selection.text.clone()}
                            message_context={props.message.get_content()}
                            position={selection.position}
                            on_translate={on_selection_translate}
                        />
                    }
                } else {
                    html! {}
                }
            }
        </>
    }
}
