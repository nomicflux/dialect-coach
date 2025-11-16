#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShortcutAction {
    ToggleSidebar,
    ToggleLearningPanel,
    ToggleUsageFooter,
    TogglePracticeSettings,
    ToggleAutoSpeak,
}

pub fn accesskey_documentation() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("ToggleSidebar", "[", "Toggle branch sidebar"),
        ("ToggleLearningPanel", "]", "Toggle learning panel"),
        ("ToggleUsageFooter", "u", "Toggle usage footer"),
        ("TogglePracticeSettings", "p", "Toggle practice settings"),
        ("ToggleAutoSpeak", "a", "Toggle auto-speak/TTS"),
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
    fn test_accesskey_documentation_contains_all_implemented() {
        let docs = accesskey_documentation();
        assert_eq!(docs.len(), 5);

        let keys: Vec<&str> = docs.iter().map(|(_, key, _)| *key).collect();
        assert!(keys.contains(&"["));
        assert!(keys.contains(&"]"));
        assert!(keys.contains(&"u"));
        assert!(keys.contains(&"p"));
        assert!(keys.contains(&"a"));
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
