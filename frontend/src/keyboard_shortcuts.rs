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
}

pub fn default_shortcuts() -> HashMap<ShortcutAction, KeyBinding> {
    let mut map = HashMap::new();
    map.insert(ShortcutAction::ToggleSidebar, KeyBinding::new("ArrowLeft", true, false));
    map.insert(ShortcutAction::ToggleLearningPanel, KeyBinding::new("ArrowRight", true, false));
    map.insert(ShortcutAction::ToggleUsageFooter, KeyBinding::new("u", true, false));
    map.insert(ShortcutAction::TogglePracticeSettings, KeyBinding::new("p", true, false));
    map.insert(ShortcutAction::ToggleAutoSpeak, KeyBinding::new("s", true, true)); // Ctrl+Shift+S
    map.insert(ShortcutAction::ReplayLastMessage, KeyBinding::new("s", true, false));
    map.insert(ShortcutAction::CycleDialect, KeyBinding::new("d", true, false));
    map.insert(ShortcutAction::CycleTeachingMode, KeyBinding::new("t", true, false));
    map.insert(ShortcutAction::CycleFormality, KeyBinding::new("r", true, false));
    map.insert(ShortcutAction::FocusGoalInput, KeyBinding::new("g", true, false));
    map.insert(ShortcutAction::FocusChatInput, KeyBinding::new("c", true, false));
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
        assert_eq!(binding.key, "ArrowLeft");
        assert!(binding.ctrl);
        assert!(!binding.shift);
    }

    #[test]
    fn test_default_shortcuts_auto_speak_has_shift() {
        let shortcuts = default_shortcuts();
        let binding = shortcuts.get(&ShortcutAction::ToggleAutoSpeak).unwrap();
        assert_eq!(binding.key, "s");
        assert!(binding.ctrl);
        assert!(binding.shift);
    }

    #[test]
    fn test_default_shortcuts_replay_no_shift() {
        let shortcuts = default_shortcuts();
        let binding = shortcuts.get(&ShortcutAction::ReplayLastMessage).unwrap();
        assert_eq!(binding.key, "s");
        assert!(binding.ctrl);
        assert!(!binding.shift);
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
