use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MistakeCategory {
    SpellingError { context: String },
    GrammarError { context: String },
    DialectUsageError { context: String },
    Other { context: String },
}

impl fmt::Display for MistakeCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MistakeCategory::SpellingError { context } => {
                write!(f, "Correct spelling is {}", context)
            }
            MistakeCategory::GrammarError { context } => {
                write!(f, "Example of a grammatical error {}", context)
            }
            MistakeCategory::DialectUsageError { context } => {
                write!(f, "Better usage would be {}", context)
            }
            MistakeCategory::Other { context } => {
                write!(f, "{}", context)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mistake {
    pub specific_mistake: String,
    pub mistake_category: MistakeCategory,
}

impl Mistake {
    pub fn get_content(&self) -> &str {
        &self.specific_mistake
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Explained {
    pub new_phrase: String,
    pub explanation: String,
}

impl Explained {
    pub fn get_content(&self) -> &str {
        &self.new_phrase
    }
}

/// Response from the AI agent service
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentResponse {
    pub response: String,
    pub mistakes: Option<Vec<Mistake>>,
    pub explained: Option<Vec<Explained>>,
}

impl AgentResponse {
    fn new(response: String) -> Self {
        AgentResponse {
            response,
            mistakes: None,
            explained: None,
        }
    }
}

impl From<&str> for AgentResponse {
    fn from(content: &str) -> Self {
        AgentResponse::new(String::from(content))
    }
}

impl From<String> for AgentResponse {
    fn from(content: String) -> Self {
        AgentResponse::new(content)
    }
}

impl From<&String> for AgentResponse {
    fn from(content: &String) -> Self {
        AgentResponse::new(content.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mistake_category_display_spelling() {
        let cat = MistakeCategory::SpellingError {
            context: "habla".to_string(),
        };
        assert_eq!(format!("{}", cat), "Correct spelling is habla");
    }

    #[test]
    fn test_mistake_category_display_grammar() {
        let cat = MistakeCategory::GrammarError {
            context: "verb conjugation".to_string(),
        };
        assert_eq!(format!("{}", cat), "Example of a grammatical error verb conjugation");
    }

    #[test]
    fn test_mistake_category_display_dialect() {
        let cat = MistakeCategory::DialectUsageError {
            context: "órale".to_string(),
        };
        assert_eq!(format!("{}", cat), "Better usage would be órale");
    }

    #[test]
    fn test_mistake_category_display_other() {
        let cat = MistakeCategory::Other {
            context: "informal context".to_string(),
        };
        assert_eq!(format!("{}", cat), "informal context");
    }

    #[test]
    fn test_mistake_get_content() {
        let mistake = Mistake {
            specific_mistake: "hablar".to_string(),
            mistake_category: MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        };
        assert_eq!(mistake.get_content(), "hablar");
    }

    #[test]
    fn test_mistake_serialization() {
        let mistake = Mistake {
            specific_mistake: "hablar".to_string(),
            mistake_category: MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        };
        let json = serde_json::to_string(&mistake).unwrap();
        assert!(json.contains("hablar"));
        assert!(json.contains("habla"));
    }

    #[test]
    fn test_explained_get_content() {
        let explained = Explained {
            new_phrase: "órale".to_string(),
            explanation: "Mexican slang for wow".to_string(),
        };
        assert_eq!(explained.get_content(), "órale");
    }

    #[test]
    fn test_explained_serialization() {
        let explained = Explained {
            new_phrase: "órale".to_string(),
            explanation: "Mexican slang for wow".to_string(),
        };
        let json = serde_json::to_string(&explained).unwrap();
        assert!(json.contains("órale"));
        assert!(json.contains("Mexican slang for wow"));
    }

    #[test]
    fn test_agent_response_serialization_none_fields() {
        let response = AgentResponse::from("Hello");
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"response\":\"Hello\""));
        assert!(json.contains("\"mistakes\":null"));
        assert!(json.contains("\"explained\":null"));
    }

    #[test]
    fn test_agent_response_serialization_with_mistakes() {
        let mut response = AgentResponse::from("Hello");
        response.mistakes = Some(vec![Mistake {
            specific_mistake: "hablar".to_string(),
            mistake_category: MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        }]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"mistakes\""));
        assert!(json.contains("hablar"));
    }

    #[test]
    fn test_agent_response_serialization_with_explained() {
        let mut response = AgentResponse::from("Hello");
        response.explained = Some(vec![Explained {
            new_phrase: "órale".to_string(),
            explanation: "Mexican slang".to_string(),
        }]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"explained\""));
        assert!(json.contains("órale"));
    }

    #[test]
    fn test_agent_response_deserialization_backward_compatibility() {
        let old_json = r#"{"response":"Hello"}"#;
        let response: AgentResponse = serde_json::from_str(old_json).unwrap();

        assert_eq!(response.response, "Hello");
        assert_eq!(response.mistakes, None);
        assert_eq!(response.explained, None);
    }

    #[test]
    fn test_agent_response_deserialization_with_all_fields() {
        let json = r#"{
            "response": "Hello",
            "mistakes": [{
                "specific_mistake": "hablar",
                "mistake_category": {
                    "type": "spelling_error",
                    "context": "habla"
                }
            }],
            "explained": [{
                "new_phrase": "órale",
                "explanation": "Mexican slang"
            }]
        }"#;

        let response: AgentResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.response, "Hello");
        assert_eq!(response.mistakes.as_ref().unwrap().len(), 1);
        assert_eq!(response.mistakes.unwrap()[0].specific_mistake, "hablar");
        assert_eq!(response.explained.as_ref().unwrap().len(), 1);
        assert_eq!(response.explained.unwrap()[0].new_phrase, "órale");
    }
}
