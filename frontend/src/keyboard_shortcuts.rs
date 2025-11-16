use std::collections::HashMap;

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

#[derive(Clone, PartialEq)]
pub struct KeyBinding {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl KeyBinding {
    pub fn new(key: &str, ctrl: bool, shift: bool) -> Self {
        Self {
            key: key.to_string(),
            ctrl,
            shift,
            alt: false,
            meta: false,
        }
    }

    pub fn with_alt(key: &str) -> Self {
        Self {
            key: key.to_string(),
            ctrl: false,
            shift: false,
            alt: true,
            meta: false,
        }
    }
}

pub fn default_shortcuts() -> HashMap<ShortcutAction, KeyBinding> {
    let mut map = HashMap::new();
    map.insert(ShortcutAction::ToggleSidebar, KeyBinding::with_alt("["));
    map.insert(ShortcutAction::ToggleLearningPanel, KeyBinding::with_alt("]"));
    map.insert(ShortcutAction::ToggleUsageFooter, KeyBinding::with_alt("u"));
    map.insert(ShortcutAction::TogglePracticeSettings, KeyBinding::with_alt("p"));
    map.insert(ShortcutAction::ToggleAutoSpeak, KeyBinding::with_alt("a"));
    map.insert(ShortcutAction::ReplayLastMessage, KeyBinding::with_alt("r"));
    map.insert(ShortcutAction::CycleDialect, KeyBinding::with_alt("d"));
    map.insert(ShortcutAction::CycleTeachingMode, KeyBinding::with_alt("m"));
    map.insert(ShortcutAction::CycleFormality, KeyBinding::with_alt("f"));
    map.insert(ShortcutAction::FocusGoalInput, KeyBinding::with_alt("g"));
    map.insert(ShortcutAction::FocusChatInput, KeyBinding::with_alt("i"));
    map
}

pub fn matches_binding(event: &web_sys::KeyboardEvent, binding: &KeyBinding) -> bool {
    let key_matches = event.key() == binding.key
        || event.key().to_lowercase() == binding.key.to_lowercase();
    let ctrl_matches = event.ctrl_key() == binding.ctrl || event.meta_key() == binding.ctrl;
    let shift_matches = event.shift_key() == binding.shift;
    let alt_matches = event.alt_key() == binding.alt;

    key_matches && ctrl_matches && shift_matches && alt_matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_binding_new_sets_defaults() {
        let binding = KeyBinding::new("s", true, false);
        assert_eq!(binding.key, "s");
        assert!(binding.ctrl);
        assert!(!binding.shift);
        assert!(!binding.alt);
        assert!(!binding.meta);
    }

    #[test]
    fn test_key_binding_with_shift() {
        let binding = KeyBinding::new("Enter", false, true);
        assert_eq!(binding.key, "Enter");
        assert!(!binding.ctrl);
        assert!(binding.shift);
    }

    #[test]
    fn test_default_shortcuts_contains_all_actions() {
        let shortcuts = default_shortcuts();
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
    fn test_default_shortcuts_sidebar_binding() {
        let shortcuts = default_shortcuts();
        let binding = shortcuts.get(&ShortcutAction::ToggleSidebar).unwrap();
        assert_eq!(binding.key, "[");
        assert!(binding.alt);
        assert!(!binding.ctrl);
    }

    #[test]
    fn test_default_shortcuts_auto_speak_uses_alt() {
        let shortcuts = default_shortcuts();
        let binding = shortcuts.get(&ShortcutAction::ToggleAutoSpeak).unwrap();
        assert_eq!(binding.key, "a");
        assert!(binding.alt);
        assert!(!binding.ctrl);
    }

    #[test]
    fn test_default_shortcuts_replay_uses_alt() {
        let shortcuts = default_shortcuts();
        let binding = shortcuts.get(&ShortcutAction::ReplayLastMessage).unwrap();
        assert_eq!(binding.key, "r");
        assert!(binding.alt);
        assert!(!binding.ctrl);
    }

    #[test]
    fn test_key_binding_with_alt() {
        let binding = KeyBinding::with_alt("d");
        assert_eq!(binding.key, "d");
        assert!(binding.alt);
        assert!(!binding.ctrl);
        assert!(!binding.shift);
        assert!(!binding.meta);
    }

    #[test]
    fn test_shortcut_action_equality() {
        assert_eq!(ShortcutAction::ToggleSidebar, ShortcutAction::ToggleSidebar);
        assert_ne!(ShortcutAction::ToggleSidebar, ShortcutAction::ToggleLearningPanel);
    }

    #[test]
    fn test_key_binding_clone() {
        let binding = KeyBinding::new("d", true, false);
        let cloned = binding.clone();
        assert_eq!(binding.key, cloned.key);
        assert_eq!(binding.ctrl, cloned.ctrl);
        assert_eq!(binding.shift, cloned.shift);
    }
}
