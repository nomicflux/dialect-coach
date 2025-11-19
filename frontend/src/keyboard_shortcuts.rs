use std::collections::HashMap;
use web_sys::KeyboardEvent;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShortcutAction {
    ToggleSidebar,
    ToggleLearningPanel,
    ToggleUsageFooter,
    TogglePracticeSettings,
    ToggleAutoSpeak,
    ReplayLastMessage,
    CycleDialect,
    CycleTeachingMode,
    CycleFormality,
    FocusGoalInput,
    FocusChatInput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyBinding {
    pub code: String,
    pub ctrl: bool,
    pub shift: bool,
}

impl KeyBinding {
    pub fn with_ctrl_shift(code: &str) -> Self {
        Self {
            code: code.to_string(),
            ctrl: true,
            shift: true,
        }
    }
}

pub fn default_shortcuts() -> HashMap<ShortcutAction, KeyBinding> {
    let mut map = HashMap::new();
    map.insert(
        ShortcutAction::ToggleSidebar,
        KeyBinding::with_ctrl_shift("BracketLeft"),
    );
    map.insert(
        ShortcutAction::ToggleLearningPanel,
        KeyBinding::with_ctrl_shift("BracketRight"),
    );
    map.insert(
        ShortcutAction::ToggleUsageFooter,
        KeyBinding::with_ctrl_shift("KeyU"),
    );
    map.insert(
        ShortcutAction::TogglePracticeSettings,
        KeyBinding::with_ctrl_shift("KeyP"),
    );
    map.insert(
        ShortcutAction::ToggleAutoSpeak,
        KeyBinding::with_ctrl_shift("KeyA"),
    );
    map.insert(
        ShortcutAction::ReplayLastMessage,
        KeyBinding::with_ctrl_shift("KeyR"),
    );
    map.insert(
        ShortcutAction::CycleDialect,
        KeyBinding::with_ctrl_shift("KeyD"),
    );
    map.insert(
        ShortcutAction::CycleTeachingMode,
        KeyBinding::with_ctrl_shift("KeyT"),
    );
    map.insert(
        ShortcutAction::CycleFormality,
        KeyBinding::with_ctrl_shift("KeyF"),
    );
    map.insert(
        ShortcutAction::FocusGoalInput,
        KeyBinding::with_ctrl_shift("KeyG"),
    );
    map.insert(
        ShortcutAction::FocusChatInput,
        KeyBinding::with_ctrl_shift("KeyC"),
    );
    map
}

pub fn matches_binding(event: &KeyboardEvent, binding: &KeyBinding) -> bool {
    event.ctrl_key() == binding.ctrl
        && event.shift_key() == binding.shift
        && event.code() == binding.code
}

pub fn accesskey_documentation() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("ToggleSidebar", "Ctrl+Shift+[", "Toggle branch sidebar"),
        (
            "ToggleLearningPanel",
            "Ctrl+Shift+]",
            "Toggle learning panel",
        ),
        ("ToggleUsageFooter", "Ctrl+Shift+U", "Toggle usage footer"),
        (
            "TogglePracticeSettings",
            "Ctrl+Shift+P",
            "Toggle practice settings",
        ),
        ("ToggleAutoSpeak", "Ctrl+Shift+A", "Toggle auto-speak/TTS"),
        ("ReplayLastMessage", "Ctrl+Shift+R", "Replay last message"),
        ("CycleDialect", "Ctrl+Shift+D", "Cycle dialect"),
        ("CycleTeachingMode", "Ctrl+Shift+T", "Cycle teaching mode"),
        ("CycleFormality", "Ctrl+Shift+F", "Cycle formality"),
        ("FocusGoalInput", "Ctrl+Shift+G", "Focus goal input"),
        ("FocusChatInput", "Ctrl+Shift+C", "Focus chat input"),
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
    fn test_key_binding_with_ctrl_shift() {
        let binding = KeyBinding::with_ctrl_shift("KeyA");
        assert_eq!(binding.code, "KeyA");
        assert!(binding.ctrl);
        assert!(binding.shift);
    }

    #[test]
    fn test_default_shortcuts_contains_all_actions() {
        let shortcuts = default_shortcuts();
        assert_eq!(shortcuts.len(), 11);
        assert!(shortcuts.contains_key(&ShortcutAction::ToggleSidebar));
        assert!(shortcuts.contains_key(&ShortcutAction::ToggleLearningPanel));
        assert!(shortcuts.contains_key(&ShortcutAction::ToggleUsageFooter));
        assert!(shortcuts.contains_key(&ShortcutAction::TogglePracticeSettings));
        assert!(shortcuts.contains_key(&ShortcutAction::ToggleAutoSpeak));
        assert!(shortcuts.contains_key(&ShortcutAction::ReplayLastMessage));
        assert!(shortcuts.contains_key(&ShortcutAction::CycleDialect));
        assert!(shortcuts.contains_key(&ShortcutAction::CycleTeachingMode));
        assert!(shortcuts.contains_key(&ShortcutAction::CycleFormality));
        assert!(shortcuts.contains_key(&ShortcutAction::FocusGoalInput));
        assert!(shortcuts.contains_key(&ShortcutAction::FocusChatInput));
    }

    #[test]
    fn test_default_shortcuts_uses_correct_keys() {
        let shortcuts = default_shortcuts();
        assert_eq!(
            shortcuts.get(&ShortcutAction::ToggleSidebar).unwrap().code,
            "BracketLeft"
        );
        assert_eq!(
            shortcuts
                .get(&ShortcutAction::ToggleLearningPanel)
                .unwrap()
                .code,
            "BracketRight"
        );
        assert_eq!(
            shortcuts
                .get(&ShortcutAction::ToggleUsageFooter)
                .unwrap()
                .code,
            "KeyU"
        );
        assert_eq!(
            shortcuts
                .get(&ShortcutAction::TogglePracticeSettings)
                .unwrap()
                .code,
            "KeyP"
        );
        assert_eq!(
            shortcuts
                .get(&ShortcutAction::ToggleAutoSpeak)
                .unwrap()
                .code,
            "KeyA"
        );
    }

    #[test]
    fn test_accesskey_documentation_contains_all_implemented() {
        let docs = accesskey_documentation();
        assert_eq!(docs.len(), 11);

        let keys: Vec<&str> = docs.iter().map(|(_, key, _)| *key).collect();
        assert!(keys.contains(&"Ctrl+Shift+["));
        assert!(keys.contains(&"Ctrl+Shift+]"));
        assert!(keys.contains(&"Ctrl+Shift+U"));
        assert!(keys.contains(&"Ctrl+Shift+P"));
        assert!(keys.contains(&"Ctrl+Shift+A"));
        assert!(keys.contains(&"Ctrl+Shift+R"));
        assert!(keys.contains(&"Ctrl+Shift+D"));
        assert!(keys.contains(&"Ctrl+Shift+T"));
        assert!(keys.contains(&"Ctrl+Shift+F"));
        assert!(keys.contains(&"Ctrl+Shift+G"));
        assert!(keys.contains(&"Ctrl+Shift+C"));
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
