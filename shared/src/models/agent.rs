use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;

pub type MistakeId = uuid::Uuid;
pub type ExplainedId = uuid::Uuid;

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

#[derive(Deserialize)]
struct MistakeHelper {
    #[serde(default)]
    id: Option<MistakeId>,
    specific_mistake: String,
    mistake_category: MistakeCategory,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mistake {
    pub id: MistakeId,
    pub specific_mistake: String,
    pub mistake_category: MistakeCategory,
}

impl<'de> Deserialize<'de> for Mistake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = MistakeHelper::deserialize(deserializer)?;
        let id = helper.id.unwrap_or_else(|| {
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, helper.specific_mistake.as_bytes())
        });
        Ok(Mistake {
            id,
            specific_mistake: helper.specific_mistake,
            mistake_category: helper.mistake_category,
        })
    }
}

impl Mistake {
    pub fn new(specific_mistake: String, mistake_category: MistakeCategory) -> Self {
        let id = uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_OID,
            specific_mistake.as_bytes(),
        );
        Self {
            id,
            specific_mistake,
            mistake_category,
        }
    }

    pub fn get_content(&self) -> &str {
        &self.specific_mistake
    }
}

#[derive(Deserialize)]
struct ExplainedHelper {
    #[serde(default)]
    id: Option<ExplainedId>,
    new_phrase: String,
    explanation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Explained {
    pub id: ExplainedId,
    pub new_phrase: String,
    pub explanation: String,
}

impl<'de> Deserialize<'de> for Explained {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = ExplainedHelper::deserialize(deserializer)?;
        let id = helper.id.unwrap_or_else(|| {
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, helper.new_phrase.as_bytes())
        });
        Ok(Explained {
            id,
            new_phrase: helper.new_phrase,
            explanation: helper.explanation,
        })
    }
}

impl Explained {
    pub fn new(new_phrase: String, explanation: String) -> Self {
        let id = uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_OID,
            new_phrase.as_bytes(),
        );
        Self {
            id,
            new_phrase,
            explanation,
        }
    }

    pub fn get_content(&self) -> &str {
        &self.new_phrase
    }
}

/// Score for a learning item from agent analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LearningItemScore {
    pub score: i8,
}

impl LearningItemScore {
    pub fn new(score: i8) -> Self {
        Self { score }
    }
}

impl<'de> Deserialize<'de> for LearningItemScore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ScoreHelper {
            Score(i8),
            Struct { score: i8 },
        }

        match ScoreHelper::deserialize(deserializer)? {
            ScoreHelper::Score(s) => Ok(LearningItemScore { score: s }),
            ScoreHelper::Struct { score } => Ok(LearningItemScore { score }),
        }
    }
}

/// Analysis of user's progress on mistakes and explanations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentAnalysis {
    pub mistake_scores: HashMap<MistakeId, LearningItemScore>,
    pub explained_scores: HashMap<ExplainedId, LearningItemScore>,
}

impl AgentAnalysis {
    pub fn new() -> Self {
        Self {
            mistake_scores: HashMap::new(),
            explained_scores: HashMap::new(),
        }
    }
}

impl Default for AgentAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from the AI agent service
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentResponse {
    pub response: String,
    pub mistakes: Option<Vec<Mistake>>,
    pub explained: Option<Vec<Explained>>,
    pub analysis: Option<AgentAnalysis>,
}

impl AgentResponse {
    fn new(response: String) -> Self {
        AgentResponse {
            response,
            mistakes: None,
            explained: None,
            analysis: None,
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
        let mistake = Mistake::new(
            "hablar".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        assert_eq!(mistake.get_content(), "hablar");
    }

    #[test]
    fn test_mistake_serialization() {
        let mistake = Mistake::new(
            "hablar".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        let json = serde_json::to_string(&mistake).unwrap();
        assert!(json.contains("hablar"));
        assert!(json.contains("habla"));
        assert!(json.contains("\"id\""));
    }

    #[test]
    fn test_mistake_stable_id() {
        let mistake1 = Mistake::new(
            "hablar".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        let mistake2 = Mistake::new(
            "hablar".to_string(),
            MistakeCategory::GrammarError {
                context: "different".to_string(),
            },
        );
        assert_eq!(mistake1.id, mistake2.id);
    }

    #[test]
    fn test_explained_get_content() {
        let explained = Explained::new(
            "órale".to_string(),
            "Mexican slang for wow".to_string(),
        );
        assert_eq!(explained.get_content(), "órale");
    }

    #[test]
    fn test_explained_serialization() {
        let explained = Explained::new(
            "órale".to_string(),
            "Mexican slang for wow".to_string(),
        );
        let json = serde_json::to_string(&explained).unwrap();
        assert!(json.contains("órale"));
        assert!(json.contains("Mexican slang for wow"));
        assert!(json.contains("\"id\""));
    }

    #[test]
    fn test_explained_stable_id() {
        let explained1 = Explained::new(
            "órale".to_string(),
            "Mexican slang for wow".to_string(),
        );
        let explained2 = Explained::new(
            "órale".to_string(),
            "Different explanation".to_string(),
        );
        assert_eq!(explained1.id, explained2.id);
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
        response.mistakes = Some(vec![Mistake::new(
            "hablar".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        )]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"mistakes\""));
        assert!(json.contains("hablar"));
    }

    #[test]
    fn test_agent_response_serialization_with_explained() {
        let mut response = AgentResponse::from("Hello");
        response.explained = Some(vec![Explained::new(
            "órale".to_string(),
            "Mexican slang".to_string(),
        )]);

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
        let test_id = uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_OID,
            "hablar".as_bytes(),
        );
        let test_id2 = uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_OID,
            "órale".as_bytes(),
        );
        let json = format!(r#"{{
            "response": "Hello",
            "mistakes": [{{
                "id": "{}",
                "specific_mistake": "hablar",
                "mistake_category": {{
                    "type": "spelling_error",
                    "context": "habla"
                }}
            }}],
            "explained": [{{
                "id": "{}",
                "new_phrase": "órale",
                "explanation": "Mexican slang"
            }}]
        }}"#, test_id, test_id2);

        let response: AgentResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.response, "Hello");
        assert_eq!(response.mistakes.as_ref().unwrap().len(), 1);
        assert_eq!(response.mistakes.as_ref().unwrap()[0].specific_mistake, "hablar");
        assert_eq!(response.mistakes.as_ref().unwrap()[0].id, test_id);
        assert_eq!(response.explained.as_ref().unwrap().len(), 1);
        assert_eq!(response.explained.as_ref().unwrap()[0].new_phrase, "órale");
        assert_eq!(response.explained.as_ref().unwrap()[0].id, test_id2);
    }

    #[test]
    fn test_learning_item_score_serialization() {
        let score = LearningItemScore::new(5);
        let json = serde_json::to_string(&score).unwrap();
        assert!(json.contains("\"score\":5"));

        let deserialized: LearningItemScore = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.score, 5);
    }

    #[test]
    fn test_learning_item_score_valid_range() {
        let min_score = LearningItemScore::new(-10);
        assert_eq!(min_score.score, -10);

        let max_score = LearningItemScore::new(10);
        assert_eq!(max_score.score, 10);

        let zero_score = LearningItemScore::new(0);
        assert_eq!(zero_score.score, 0);
    }

    #[test]
    fn test_agent_analysis_empty() {
        let analysis = AgentAnalysis::new();
        assert_eq!(analysis.mistake_scores.len(), 0);
        assert_eq!(analysis.explained_scores.len(), 0);

        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("\"mistake_scores\":{}"));
        assert!(json.contains("\"explained_scores\":{}"));
    }

    #[test]
    fn test_agent_analysis_with_mistakes() {
        let mut analysis = AgentAnalysis::new();
        let mistake_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hablar".as_bytes());
        analysis.mistake_scores.insert(mistake_id, LearningItemScore::new(-5));

        assert_eq!(analysis.mistake_scores.len(), 1);
        assert_eq!(analysis.mistake_scores.get(&mistake_id).unwrap().score, -5);

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: AgentAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.mistake_scores.get(&mistake_id).unwrap().score, -5);
    }

    #[test]
    fn test_agent_analysis_with_explanations() {
        let mut analysis = AgentAnalysis::new();
        let explained_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "órale".as_bytes());
        analysis.explained_scores.insert(explained_id, LearningItemScore::new(8));

        assert_eq!(analysis.explained_scores.len(), 1);
        assert_eq!(analysis.explained_scores.get(&explained_id).unwrap().score, 8);

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: AgentAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.explained_scores.get(&explained_id).unwrap().score, 8);
    }

    #[test]
    fn test_agent_response_with_analysis_serialization() {
        let mut response = AgentResponse::from("Hello");
        let mut analysis = AgentAnalysis::new();
        let mistake_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hablar".as_bytes());
        analysis.mistake_scores.insert(mistake_id, LearningItemScore::new(-5));
        response.analysis = Some(analysis);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"analysis\""));
        assert!(json.contains("\"mistake_scores\""));
    }

    #[test]
    fn test_agent_response_with_analysis_deserialization() {
        let mistake_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hablar".as_bytes());
        let json = format!(r#"{{
            "response": "Hello",
            "mistakes": null,
            "explained": null,
            "analysis": {{
                "mistake_scores": {{
                    "{}": {{"score": -5}}
                }},
                "explained_scores": {{}}
            }}
        }}"#, mistake_id);

        let response: AgentResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.response, "Hello");
        assert!(response.analysis.is_some());
        let analysis = response.analysis.unwrap();
        assert_eq!(analysis.mistake_scores.get(&mistake_id).unwrap().score, -5);
    }
}
