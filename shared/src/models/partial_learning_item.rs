use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialMistake {
    pub specific_mistake: Option<String>,
    pub correction: Option<String>,
    pub mistake_category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialExplained {
    pub new_phrase: Option<String>,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialTranslated {
    pub translated_word: Option<String>,
    pub translated_to: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialExploratory {
    pub point_to_try: Option<String>,
    pub instructions_for_use: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PartialLearningItem {
    Mistake(PartialMistake),
    Explained(PartialExplained),
    Translated(PartialTranslated),
    Exploratory(PartialExploratory),
}

pub fn validate_partial_mistake(p: &PartialMistake) -> Result<(), String> {
    if p.specific_mistake.is_none() {
        return Err("specific_mistake is required".to_string());
    }
    Ok(())
}

pub fn validate_partial_translated(p: &PartialTranslated) -> Result<(), String> {
    let word_filled = p.translated_word.is_some();
    let to_filled = p.translated_to.is_some();

    if word_filled && to_filled {
        return Err("exactly one of translated_word or translated_to must be provided".to_string());
    }
    if !word_filled && !to_filled {
        return Err("at least one of translated_word or translated_to is required".to_string());
    }
    Ok(())
}

pub fn validate_partial_explained(p: &PartialExplained) -> Result<(), String> {
    if p.new_phrase.is_none() && p.explanation.is_none() {
        return Err("at least one of new_phrase or explanation is required".to_string());
    }
    Ok(())
}

pub fn validate_partial_exploratory(p: &PartialExploratory) -> Result<(), String> {
    if p.point_to_try.is_none() && p.instructions_for_use.is_none() {
        return Err("at least one of point_to_try or instructions_for_use is required".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_partial_mistake_valid() {
        let mistake = PartialMistake {
            specific_mistake: Some("Cómo andes".to_string()),
            correction: None,
            mistake_category: None,
        };
        assert!(validate_partial_mistake(&mistake).is_ok());
    }

    #[test]
    fn test_validate_partial_mistake_missing() {
        let mistake = PartialMistake {
            specific_mistake: None,
            correction: Some("Cómo andás".to_string()),
            mistake_category: None,
        };
        assert!(validate_partial_mistake(&mistake).is_err());
    }

    #[test]
    fn test_validate_partial_translated_word_only() {
        let translated = PartialTranslated {
            translated_word: Some("hello".to_string()),
            translated_to: None,
            context: None,
        };
        assert!(validate_partial_translated(&translated).is_ok());
    }

    #[test]
    fn test_validate_partial_translated_to_only() {
        let translated = PartialTranslated {
            translated_word: None,
            translated_to: Some("hola".to_string()),
            context: None,
        };
        assert!(validate_partial_translated(&translated).is_ok());
    }

    #[test]
    fn test_validate_partial_translated_both() {
        let translated = PartialTranslated {
            translated_word: Some("hello".to_string()),
            translated_to: Some("hola".to_string()),
            context: None,
        };
        assert!(validate_partial_translated(&translated).is_err());
    }

    #[test]
    fn test_validate_partial_translated_neither() {
        let translated = PartialTranslated {
            translated_word: None,
            translated_to: None,
            context: None,
        };
        assert!(validate_partial_translated(&translated).is_err());
    }

    #[test]
    fn test_validate_partial_explained_phrase_only() {
        let explained = PartialExplained {
            new_phrase: Some("órale".to_string()),
            explanation: None,
        };
        assert!(validate_partial_explained(&explained).is_ok());
    }

    #[test]
    fn test_validate_partial_explained_explanation_only() {
        let explained = PartialExplained {
            new_phrase: None,
            explanation: Some("Mexican slang for wow".to_string()),
        };
        assert!(validate_partial_explained(&explained).is_ok());
    }

    #[test]
    fn test_validate_partial_explained_both() {
        let explained = PartialExplained {
            new_phrase: Some("órale".to_string()),
            explanation: Some("Mexican slang".to_string()),
        };
        assert!(validate_partial_explained(&explained).is_ok());
    }

    #[test]
    fn test_validate_partial_explained_neither() {
        let explained = PartialExplained {
            new_phrase: None,
            explanation: None,
        };
        assert!(validate_partial_explained(&explained).is_err());
    }

    #[test]
    fn test_validate_partial_exploratory_point_only() {
        let exploratory = PartialExploratory {
            point_to_try: Some("Use subjunctive mood".to_string()),
            instructions_for_use: None,
        };
        assert!(validate_partial_exploratory(&exploratory).is_ok());
    }

    #[test]
    fn test_validate_partial_exploratory_instructions_only() {
        let exploratory = PartialExploratory {
            point_to_try: None,
            instructions_for_use: Some("Try 'Si fuera rico'".to_string()),
        };
        assert!(validate_partial_exploratory(&exploratory).is_ok());
    }

    #[test]
    fn test_validate_partial_exploratory_both() {
        let exploratory = PartialExploratory {
            point_to_try: Some("Use subjunctive".to_string()),
            instructions_for_use: Some("Try it".to_string()),
        };
        assert!(validate_partial_exploratory(&exploratory).is_ok());
    }

    #[test]
    fn test_validate_partial_exploratory_neither() {
        let exploratory = PartialExploratory {
            point_to_try: None,
            instructions_for_use: None,
        };
        assert!(validate_partial_exploratory(&exploratory).is_err());
    }

    #[test]
    fn test_partial_mistake_serialization() {
        let mistake = PartialMistake {
            specific_mistake: Some("Cómo andes".to_string()),
            correction: None,
            mistake_category: None,
        };
        let json = serde_json::to_string(&mistake).unwrap();
        assert!(json.contains("Cómo andes"));
        assert!(json.contains("\"specific_mistake\""));
    }

    #[test]
    fn test_partial_mistake_deserialization() {
        let json =
            r#"{"specific_mistake": "Cómo andes", "correction": null, "mistake_category": null}"#;
        let mistake: PartialMistake = serde_json::from_str(json).unwrap();
        assert_eq!(mistake.specific_mistake, Some("Cómo andes".to_string()));
    }

    #[test]
    fn test_partial_translated_serialization() {
        let translated = PartialTranslated {
            translated_word: Some("hello".to_string()),
            translated_to: None,
            context: None,
        };
        let json = serde_json::to_string(&translated).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("\"translated_word\""));
    }

    #[test]
    fn test_partial_translated_deserialization() {
        let json = r#"{"translated_word": "hello", "translated_to": null, "context": null}"#;
        let translated: PartialTranslated = serde_json::from_str(json).unwrap();
        assert_eq!(translated.translated_word, Some("hello".to_string()));
    }

    #[test]
    fn test_partial_explained_serialization() {
        let explained = PartialExplained {
            new_phrase: Some("órale".to_string()),
            explanation: None,
        };
        let json = serde_json::to_string(&explained).unwrap();
        assert!(json.contains("órale"));
    }

    #[test]
    fn test_partial_explained_deserialization() {
        let json = r#"{"new_phrase": "órale", "explanation": null}"#;
        let explained: PartialExplained = serde_json::from_str(json).unwrap();
        assert_eq!(explained.new_phrase, Some("órale".to_string()));
    }

    #[test]
    fn test_partial_exploratory_serialization() {
        let exploratory = PartialExploratory {
            point_to_try: Some("Use subjunctive".to_string()),
            instructions_for_use: None,
        };
        let json = serde_json::to_string(&exploratory).unwrap();
        assert!(json.contains("Use subjunctive"));
    }

    #[test]
    fn test_partial_exploratory_deserialization() {
        let json = r#"{"point_to_try": "Use subjunctive", "instructions_for_use": null}"#;
        let exploratory: PartialExploratory = serde_json::from_str(json).unwrap();
        assert_eq!(
            exploratory.point_to_try,
            Some("Use subjunctive".to_string())
        );
    }

    #[test]
    fn test_partial_learning_item_enum_mistake() {
        let mistake = PartialMistake {
            specific_mistake: Some("error".to_string()),
            correction: None,
            mistake_category: None,
        };
        let item = PartialLearningItem::Mistake(mistake);
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"mistake\""));
    }

    #[test]
    fn test_partial_learning_item_enum_translated() {
        let translated = PartialTranslated {
            translated_word: Some("hello".to_string()),
            translated_to: None,
            context: None,
        };
        let item = PartialLearningItem::Translated(translated);
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"translated\""));
    }

    #[test]
    fn test_partial_learning_item_enum_explained() {
        let explained = PartialExplained {
            new_phrase: Some("word".to_string()),
            explanation: None,
        };
        let item = PartialLearningItem::Explained(explained);
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"explained\""));
    }

    #[test]
    fn test_partial_learning_item_enum_exploratory() {
        let exploratory = PartialExploratory {
            point_to_try: Some("point".to_string()),
            instructions_for_use: None,
        };
        let item = PartialLearningItem::Exploratory(exploratory);
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"exploratory\""));
    }
}
