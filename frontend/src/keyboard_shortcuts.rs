use std::collections::HashMap;
use web_sys::KeyboardEvent;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShortcutAction {
    ToggleSidebar,
    ToggleLearningPanel,
    ToggleUsageFooter,
    TogglePracticeSettings,
    ToggleAutoSpeak,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyBinding {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyBinding {
    pub fn with_ctrl_alt(key: &str) -> Self {
        Self {
            key: key.to_string(),
            ctrl: true,
            alt: true,
        }
    }
}

pub fn default_shortcuts() -> HashMap<ShortcutAction, KeyBinding> {
    let mut map = HashMap::new();
    map.insert(ShortcutAction::ToggleSidebar, KeyBinding::with_ctrl_alt("["));
    map.insert(
        ShortcutAction::ToggleLearningPanel,
        KeyBinding::with_ctrl_alt("]"),
    );
    map.insert(
        ShortcutAction::ToggleUsageFooter,
        KeyBinding::with_ctrl_alt("u"),
    );
    map.insert(
        ShortcutAction::TogglePracticeSettings,
        KeyBinding::with_ctrl_alt("p"),
    );
    map.insert(
        ShortcutAction::ToggleAutoSpeak,
        KeyBinding::with_ctrl_alt("a"),
    );
    map
}

pub fn matches_binding(event: &KeyboardEvent, binding: &KeyBinding) -> bool {
    event.ctrl_key() == binding.ctrl && event.alt_key() == binding.alt && event.key() == binding.key
}

pub fn accesskey_documentation() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("ToggleSidebar", "Ctrl+Option+[", "Toggle branch sidebar"),
        (
            "ToggleLearningPanel",
            "Ctrl+Option+]",
            "Toggle learning panel",
        ),
        ("ToggleUsageFooter", "Ctrl+Option+u", "Toggle usage footer"),
        (
            "TogglePracticeSettings",
            "Ctrl+Option+p",
            "Toggle practice settings",
        ),
        ("ToggleAutoSpeak", "Ctrl+Option+a", "Toggle auto-speak/TTS"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_action_equality() {
        assert_eq!(ShortcutAction::ToggleSidebar, ShortcutAction::ToggleSidebar);
        assert_ne!(
            ShortcutAction::ToggleSidebar,
            ShortcutAction::ToggleLearningPanel
        );
    }

    #[test]
    fn test_key_binding_with_ctrl_alt() {
        let binding = KeyBinding::with_ctrl_alt("a");
        assert_eq!(binding.key, "a");
        assert!(binding.ctrl);
        assert!(binding.alt);
    }

    #[test]
    fn test_default_shortcuts_contains_all_actions() {
        let shortcuts = default_shortcuts();
        assert_eq!(shortcuts.len(), 5);
        assert!(shortcuts.contains_key(&ShortcutAction::ToggleSidebar));
        assert!(shortcuts.contains_key(&ShortcutAction::ToggleLearningPanel));
        assert!(shortcuts.contains_key(&ShortcutAction::ToggleUsageFooter));
        assert!(shortcuts.contains_key(&ShortcutAction::TogglePracticeSettings));
        assert!(shortcuts.contains_key(&ShortcutAction::ToggleAutoSpeak));
    }

    #[test]
    fn test_default_shortcuts_uses_correct_keys() {
        let shortcuts = default_shortcuts();
        assert_eq!(
            shortcuts.get(&ShortcutAction::ToggleSidebar).unwrap().key,
            "["
        );
        assert_eq!(
            shortcuts
                .get(&ShortcutAction::ToggleLearningPanel)
                .unwrap()
                .key,
            "]"
        );
        assert_eq!(
            shortcuts
                .get(&ShortcutAction::ToggleUsageFooter)
                .unwrap()
                .key,
            "u"
        );
        assert_eq!(
            shortcuts
                .get(&ShortcutAction::TogglePracticeSettings)
                .unwrap()
                .key,
            "p"
        );
        assert_eq!(
            shortcuts.get(&ShortcutAction::ToggleAutoSpeak).unwrap().key,
            "a"
        );
    }

    #[test]
    fn test_accesskey_documentation_contains_all_implemented() {
        let docs = accesskey_documentation();
        assert_eq!(docs.len(), 5);

        let keys: Vec<&str> = docs.iter().map(|(_, key, _)| *key).collect();
        assert!(keys.contains(&"Ctrl+Option+["));
        assert!(keys.contains(&"Ctrl+Option+]"));
        assert!(keys.contains(&"Ctrl+Option+u"));
        assert!(keys.contains(&"Ctrl+Option+p"));
        assert!(keys.contains(&"Ctrl+Option+a"));
    }

    #[test]
    fn test_accesskey_documentation_has_descriptions() {
        let docs = accesskey_documentation();
        for (name, key, description) in docs {
            assert!(!name.is_empty());
            assert!(!key.is_empty());
            assert!(!description.is_empty());
        }
    }
}
