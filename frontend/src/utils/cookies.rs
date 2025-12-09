use wasm_bindgen::JsCast;
use web_sys::{HtmlDocument, window};

const SESSION_COOKIE_NAME: &str = "dialect_coach_session";
const MAX_AGE_SECONDS: i32 = 86400; // 24 hours

pub fn set_session_token(token: &str) {
    if let Some(document) = window().and_then(|w| w.document())
        && let Ok(html_doc) = document.dyn_into::<HtmlDocument>()
    {
        let cookie = format!(
            "{}={}; max-age={}; path=/; SameSite=Lax",
            SESSION_COOKIE_NAME, token, MAX_AGE_SECONDS
        );
        html_doc.set_cookie(&cookie).ok();
    }
}

pub fn get_session_token() -> Option<String> {
    let document = window()?.document()?;
    let html_doc = document.dyn_into::<HtmlDocument>().ok()?;
    let cookies = html_doc.cookie().ok()?;
    parse_cookie(&cookies, SESSION_COOKIE_NAME)
}

pub fn clear_session_token() {
    if let Some(document) = window().and_then(|w| w.document())
        && let Ok(html_doc) = document.dyn_into::<HtmlDocument>()
    {
        let cookie = format!("{}=; max-age=0; path=/", SESSION_COOKIE_NAME);
        html_doc.set_cookie(&cookie).ok();
    }
}

fn parse_cookie(cookies: &str, name: &str) -> Option<String> {
    cookies
        .split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with(&format!("{}=", name)))
        .and_then(|s| s.split('=').nth(1))
        .map(String::from)
}
