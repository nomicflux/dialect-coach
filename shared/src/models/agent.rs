use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;

pub type MistakeId = uuid::Uuid;
pub type ExplainedId = uuid::Uuid;
pub type TranslatedId = uuid::Uuid;
pub type ExploratoryId = uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MistakeCategory {
    SpellingError { context: String },
    VocabularyError { context: String },
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
            MistakeCategory::VocabularyError { context } => {
                let size = context.split_once(" ").iter().len();
                let unit_word = if size == 1 { "word" } else { "phrase" };
                write!(f, "Correct {} is {}", unit_word, context)
            }
            MistakeCategory::GrammarError { context } => {
                write!(f, "Example of a grammatical error {}", context)
            }
            MistakeCategory::DialectUsageError { context } => {
                write!(f, "More natural usage would be {}", context)
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
    correction: String,
    mistake_category: MistakeCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mistake {
    pub id: MistakeId,
    pub specific_mistake: String,
    pub correction: String,
    pub mistake_category: MistakeCategory,
}

impl<'de> Deserialize<'de> for Mistake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = MistakeHelper::deserialize(deserializer)?;
        let id = helper.id.unwrap_or_else(|| {
            let hash_input = format!(
                "{}|{}|{:?}",
                helper.specific_mistake, helper.correction, helper.mistake_category
            );
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes())
        });
        Ok(Mistake {
            id,
            specific_mistake: helper.specific_mistake,
            correction: helper.correction,
            mistake_category: helper.mistake_category,
        })
    }
}

impl Mistake {
    pub fn new(
        specific_mistake: String,
        correction: String,
        mistake_category: MistakeCategory,
    ) -> Self {
        let hash_input = format!("{}|{}|{:?}", specific_mistake, correction, mistake_category);
        let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes());
        Self {
            id,
            specific_mistake,
            correction,
            mistake_category,
        }
    }

    pub fn get_content(&self) -> String {
        format!("{} -> {}", self.specific_mistake, self.correction)
    }
}

#[derive(Deserialize)]
struct ExplainedHelper {
    #[serde(default)]
    id: Option<ExplainedId>,
    new_phrase: String,
    explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
            let hash_input = format!("{}|{}", helper.new_phrase, helper.explanation);
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes())
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
        let hash_input = format!("{}|{}", new_phrase, explanation);
        let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes());
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

#[derive(Deserialize)]
struct TranslatedHelper {
    #[serde(default)]
    id: Option<TranslatedId>,
    translated_word: String,
    translated_to: String,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Translated {
    pub id: TranslatedId,
    pub translated_word: String,
    pub translated_to: String,
    pub context: Option<String>,
}

impl<'de> Deserialize<'de> for Translated {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = TranslatedHelper::deserialize(deserializer)?;
        let id = helper.id.unwrap_or_else(|| {
            let hash_input = format!("{}|{}", helper.translated_word, helper.translated_to);
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes())
        });
        Ok(Translated {
            id,
            translated_word: helper.translated_word,
            translated_to: helper.translated_to,
            context: helper.context,
        })
    }
}

impl Translated {
    pub fn new(translated_word: String, translated_to: String, context: Option<String>) -> Self {
        let hash_input = format!("{}|{}", translated_word, translated_to);
        let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes());
        Self {
            id,
            translated_word,
            translated_to,
            context,
        }
    }

    pub fn get_content(&self) -> String {
        format!("{} -> {}", self.translated_word, self.translated_to)
    }
}

#[derive(Deserialize)]
struct ExploratoryHelper {
    #[serde(default)]
    id: Option<ExploratoryId>,
    point_to_try: String,
    instructions_for_use: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Exploratory {
    pub id: ExploratoryId,
    pub point_to_try: String,
    pub instructions_for_use: String,
}

impl<'de> Deserialize<'de> for Exploratory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = ExploratoryHelper::deserialize(deserializer)?;
        let id = helper.id.unwrap_or_else(|| {
            let hash_input = format!("{}|{}", helper.point_to_try, helper.instructions_for_use);
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes())
        });
        Ok(Exploratory {
            id,
            point_to_try: helper.point_to_try,
            instructions_for_use: helper.instructions_for_use,
        })
    }
}

impl Exploratory {
    pub fn new(point_to_try: String, instructions_for_use: String) -> Self {
        let hash_input = format!("{}|{}", point_to_try, instructions_for_use);
        let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes());
        Self {
            id,
            point_to_try,
            instructions_for_use,
        }
    }

    pub fn get_content(&self) -> String {
        self.point_to_try.clone()
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAnalysis {
    pub mistake_scores: HashMap<MistakeId, LearningItemScore>,
    pub explained_scores: HashMap<ExplainedId, LearningItemScore>,
    pub translated_scores: HashMap<TranslatedId, LearningItemScore>,
    pub exploratory_scores: HashMap<ExploratoryId, LearningItemScore>,
}

impl AgentAnalysis {
    pub fn new() -> Self {
        Self {
            mistake_scores: HashMap::new(),
            explained_scores: HashMap::new(),
            translated_scores: HashMap::new(),
            exploratory_scores: HashMap::new(),
        }
    }
}

impl Default for AgentAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from the AI agent service
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentResponse {
    pub response: String,
    pub mistakes: Option<Vec<(Mistake, u8)>>,
    pub explained: Option<Vec<(Explained, u8)>>,
    pub translated: Option<Vec<(Translated, u8)>>,
    pub exploratory: Option<Vec<(Exploratory, u8)>>,
    pub analysis: Option<AgentAnalysis>,
    #[serde(default)]
    pub pronunciation_text: Option<String>,
}

impl AgentResponse {
    fn new(response: String) -> Self {
        AgentResponse {
            response,
            mistakes: None,
            explained: None,
            translated: None,
            exploratory: None,
            analysis: None,
            pronunciation_text: None,
        }
    }

    pub fn as_str(&self) -> &str {
        self.response.as_str()
    }

    pub fn get_tts_text(&self) -> &str {
        self.pronunciation_text.as_deref().unwrap_or(&self.response)
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
        assert_eq!(
            format!("{}", cat),
            "Example of a grammatical error verb conjugation"
        );
    }

    #[test]
    fn test_mistake_category_display_dialect() {
        let cat = MistakeCategory::DialectUsageError {
            context: "órale".to_string(),
        };
        assert_eq!(format!("{}", cat), "More natural usage would be órale");
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
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        assert_eq!(mistake.get_content(), "hablar -> habla");
    }

    #[test]
    fn test_mistake_serialization() {
        let mistake = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        let json = serde_json::to_string(&mistake).unwrap();
        assert!(json.contains("hablar"));
        assert!(json.contains("habla"));
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"correction\""));
    }

    #[test]
    fn test_mistake_stable_id() {
        let mistake1 = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        let mistake2 = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "habla".to_string(),
            },
        );
        let mistake3 = Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::GrammarError {
                context: "different".to_string(),
            },
        );
        assert_eq!(mistake1.id, mistake2.id);
        assert_ne!(mistake1.id, mistake3.id);
    }

    #[test]
    fn test_explained_get_content() {
        let explained = Explained::new("órale".to_string(), "Mexican slang for wow".to_string());
        assert_eq!(explained.get_content(), "órale");
    }

    #[test]
    fn test_explained_serialization() {
        let explained = Explained::new("órale".to_string(), "Mexican slang for wow".to_string());
        let json = serde_json::to_string(&explained).unwrap();
        assert!(json.contains("órale"));
        assert!(json.contains("Mexican slang for wow"));
        assert!(json.contains("\"id\""));
    }

    #[test]
    fn test_explained_stable_id() {
        let explained1 = Explained::new("órale".to_string(), "Mexican slang for wow".to_string());
        let explained2 = Explained::new("órale".to_string(), "Mexican slang for wow".to_string());
        let explained3 = Explained::new("órale".to_string(), "Different explanation".to_string());
        assert_eq!(explained1.id, explained2.id);
        assert_ne!(explained1.id, explained3.id);
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
    fn test_agent_response_pronunciation_text_defaults_to_none() {
        let old_json = r#"{"response":"Hello"}"#;
        let response: AgentResponse = serde_json::from_str(old_json).unwrap();
        assert_eq!(response.pronunciation_text, None);
    }

    #[test]
    fn test_get_tts_text_returns_pronunciation_when_present() {
        let mut response = AgentResponse::from("Display");
        response.pronunciation_text = Some("Pronunciation".to_string());
        assert_eq!(response.get_tts_text(), "Pronunciation");
    }

    #[test]
    fn test_get_tts_text_returns_response_when_no_pronunciation() {
        let response = AgentResponse::from("Display");
        assert_eq!(response.get_tts_text(), "Display");
    }

    #[test]
    fn test_agent_response_serialization_with_mistakes() {
        let mut response = AgentResponse::from("Hello");
        response.mistakes = Some(vec![(
            Mistake::new(
                "hablar".to_string(),
                "habla".to_string(),
                MistakeCategory::SpellingError {
                    context: "habla".to_string(),
                },
            ),
            0,
        )]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"mistakes\""));
        assert!(json.contains("hablar"));
    }

    #[test]
    fn test_agent_response_serialization_with_explained() {
        let mut response = AgentResponse::from("Hello");
        response.explained = Some(vec![(
            Explained::new("órale".to_string(), "Mexican slang".to_string()),
            0,
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
        let test_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hablar".as_bytes());
        let test_id2 = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "órale".as_bytes());
        let json = format!(
            r#"{{
            "response": "Hello",
            "mistakes": [[{{
                "id": "{}",
                "specific_mistake": "hablar",
                "correction": "habla",
                "mistake_category": {{
                    "type": "spelling_error",
                    "context": "habla"
                }}
            }}, 0]],
            "explained": [[{{
                "id": "{}",
                "new_phrase": "órale",
                "explanation": "Mexican slang"
            }}, 0]]
        }}"#,
            test_id, test_id2
        );

        let response: AgentResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.response, "Hello");
        assert_eq!(response.mistakes.as_ref().unwrap().len(), 1);
        assert_eq!(
            response.mistakes.as_ref().unwrap()[0].0.specific_mistake,
            "hablar"
        );
        assert_eq!(response.mistakes.as_ref().unwrap()[0].0.id, test_id);
        assert_eq!(response.explained.as_ref().unwrap().len(), 1);
        assert_eq!(
            response.explained.as_ref().unwrap()[0].0.new_phrase,
            "órale"
        );
        assert_eq!(response.explained.as_ref().unwrap()[0].0.id, test_id2);
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
        assert_eq!(analysis.translated_scores.len(), 0);
        assert_eq!(analysis.exploratory_scores.len(), 0);

        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("\"mistake_scores\":{}"));
        assert!(json.contains("\"explained_scores\":{}"));
        assert!(json.contains("\"translated_scores\":{}"));
        assert!(json.contains("\"exploratory_scores\":{}"));
    }

    #[test]
    fn test_agent_analysis_with_mistakes() {
        let mut analysis = AgentAnalysis::new();
        let mistake_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hablar".as_bytes());
        analysis
            .mistake_scores
            .insert(mistake_id, LearningItemScore::new(-5));

        assert_eq!(analysis.mistake_scores.len(), 1);
        assert_eq!(analysis.mistake_scores.get(&mistake_id).unwrap().score, -5);

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: AgentAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.mistake_scores.get(&mistake_id).unwrap().score,
            -5
        );
    }

    #[test]
    fn test_agent_analysis_with_explanations() {
        let mut analysis = AgentAnalysis::new();
        let explained_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "órale".as_bytes());
        analysis
            .explained_scores
            .insert(explained_id, LearningItemScore::new(8));

        assert_eq!(analysis.explained_scores.len(), 1);
        assert_eq!(
            analysis.explained_scores.get(&explained_id).unwrap().score,
            8
        );

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: AgentAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized
                .explained_scores
                .get(&explained_id)
                .unwrap()
                .score,
            8
        );
    }

    #[test]
    fn test_agent_response_with_analysis_serialization() {
        let mut response = AgentResponse::from("Hello");
        let mut analysis = AgentAnalysis::new();
        let mistake_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hablar".as_bytes());
        analysis
            .mistake_scores
            .insert(mistake_id, LearningItemScore::new(-5));
        response.analysis = Some(analysis);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"analysis\""));
        assert!(json.contains("\"mistake_scores\""));
    }

    #[test]
    fn test_agent_response_with_analysis_deserialization() {
        let mistake_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hablar".as_bytes());
        let json = format!(
            r#"{{
            "response": "Hello",
            "mistakes": null,
            "explained": null,
            "translated": null,
            "exploratory": null,
            "analysis": {{
                "mistake_scores": {{
                    "{}": {{"score": -5}}
                }},
                "explained_scores": {{}},
                "translated_scores": {{}},
                "exploratory_scores": {{}}
            }}
        }}"#,
            mistake_id
        );

        let response: AgentResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.response, "Hello");
        assert!(response.analysis.is_some());
        let analysis = response.analysis.unwrap();
        assert_eq!(analysis.mistake_scores.get(&mistake_id).unwrap().score, -5);
    }

    #[test]
    fn test_translated_get_content() {
        let translated = Translated::new("hello".to_string(), "hola".to_string(), None);
        assert_eq!(translated.get_content(), "hello -> hola");
    }

    #[test]
    fn test_translated_serialization() {
        let translated = Translated::new("hello".to_string(), "hola".to_string(), None);
        let json = serde_json::to_string(&translated).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("hola"));
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"translated_word\""));
        assert!(json.contains("\"translated_to\""));
    }

    #[test]
    fn test_translated_stable_id() {
        let translated1 = Translated::new("hello".to_string(), "hola".to_string(), None);
        let translated2 = Translated::new("hello".to_string(), "hola".to_string(), None);
        let translated3 = Translated::new("hello".to_string(), "¡hola!".to_string(), None);
        assert_eq!(translated1.id, translated2.id);
        assert_ne!(translated1.id, translated3.id);
    }

    #[test]
    fn test_translated_deserialization_without_id() {
        let json = r#"{"translated_word": "hello", "translated_to": "hola"}"#;
        let translated: Translated = serde_json::from_str(json).unwrap();
        assert_eq!(translated.translated_word, "hello");
        assert_eq!(translated.translated_to, "hola");
        assert_eq!(translated.context, None);

        let hash_input = "hello|hola";
        let expected_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes());
        assert_eq!(translated.id, expected_id);
    }

    #[test]
    fn test_translated_with_context() {
        let translated = Translated::new(
            "hello".to_string(),
            "hola".to_string(),
            Some("I said hello to my friend".to_string()),
        );
        assert_eq!(translated.translated_word, "hello");
        assert_eq!(translated.translated_to, "hola");
        assert_eq!(
            translated.context,
            Some("I said hello to my friend".to_string())
        );
    }

    #[test]
    fn test_translated_serialization_with_context() {
        let translated = Translated::new(
            "hello".to_string(),
            "hola".to_string(),
            Some("I said hello to my friend".to_string()),
        );
        let json = serde_json::to_string(&translated).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("hola"));
        assert!(json.contains("I said hello to my friend"));
        assert!(json.contains("\"context\""));
    }

    #[test]
    fn test_translated_deserialization_with_context() {
        let json =
            r#"{"translated_word": "hello", "translated_to": "hola", "context": "I said hello"}"#;
        let translated: Translated = serde_json::from_str(json).unwrap();
        assert_eq!(translated.translated_word, "hello");
        assert_eq!(translated.translated_to, "hola");
        assert_eq!(translated.context, Some("I said hello".to_string()));
    }

    #[test]
    fn test_exploratory_get_content() {
        let exploratory = Exploratory::new(
            "Use subjunctive mood".to_string(),
            "Try saying 'Si fuera rico' instead of 'Si soy rico'".to_string(),
        );
        assert_eq!(exploratory.get_content(), "Use subjunctive mood");
    }

    #[test]
    fn test_exploratory_serialization() {
        let exploratory = Exploratory::new(
            "Use subjunctive mood".to_string(),
            "Try saying 'Si fuera rico'".to_string(),
        );
        let json = serde_json::to_string(&exploratory).unwrap();
        assert!(json.contains("Use subjunctive mood"));
        assert!(json.contains("Si fuera rico"));
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"point_to_try\""));
        assert!(json.contains("\"instructions_for_use\""));
    }

    #[test]
    fn test_exploratory_stable_id() {
        let exploratory1 = Exploratory::new(
            "Use subjunctive mood".to_string(),
            "Instructions A".to_string(),
        );
        let exploratory2 = Exploratory::new(
            "Use subjunctive mood".to_string(),
            "Instructions A".to_string(),
        );
        let exploratory3 = Exploratory::new(
            "Use subjunctive mood".to_string(),
            "Instructions B".to_string(),
        );
        assert_eq!(exploratory1.id, exploratory2.id);
        assert_ne!(exploratory1.id, exploratory3.id);
    }

    #[test]
    fn test_exploratory_deserialization_without_id() {
        let json = r#"{"point_to_try": "Use subjunctive", "instructions_for_use": "Try it"}"#;
        let exploratory: Exploratory = serde_json::from_str(json).unwrap();
        assert_eq!(exploratory.point_to_try, "Use subjunctive");
        assert_eq!(exploratory.instructions_for_use, "Try it");

        let hash_input = "Use subjunctive|Try it";
        let expected_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, hash_input.as_bytes());
        assert_eq!(exploratory.id, expected_id);
    }

    #[test]
    fn test_agent_analysis_with_translated() {
        let mut analysis = AgentAnalysis::new();
        let translated_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hello".as_bytes());
        analysis
            .translated_scores
            .insert(translated_id, LearningItemScore::new(5));

        assert_eq!(analysis.translated_scores.len(), 1);
        assert_eq!(
            analysis
                .translated_scores
                .get(&translated_id)
                .unwrap()
                .score,
            5
        );

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: AgentAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized
                .translated_scores
                .get(&translated_id)
                .unwrap()
                .score,
            5
        );
    }

    #[test]
    fn test_agent_analysis_with_exploratory() {
        let mut analysis = AgentAnalysis::new();
        let exploratory_id =
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "Use subjunctive".as_bytes());
        analysis
            .exploratory_scores
            .insert(exploratory_id, LearningItemScore::new(8));

        assert_eq!(analysis.exploratory_scores.len(), 1);
        assert_eq!(
            analysis
                .exploratory_scores
                .get(&exploratory_id)
                .unwrap()
                .score,
            8
        );

        let json = serde_json::to_string(&analysis).unwrap();
        let deserialized: AgentAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized
                .exploratory_scores
                .get(&exploratory_id)
                .unwrap()
                .score,
            8
        );
    }

    #[test]
    fn test_agent_response_with_translated() {
        let mut response = AgentResponse::from("Hello");
        response.translated = Some(vec![(
            Translated::new("hello".to_string(), "hola".to_string(), None),
            0,
        )]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"translated\""));
        assert!(json.contains("hello"));
        assert!(json.contains("hola"));
    }

    #[test]
    fn test_agent_response_with_exploratory() {
        let mut response = AgentResponse::from("Try this");
        response.exploratory = Some(vec![(
            Exploratory::new(
                "Use subjunctive mood".to_string(),
                "Try 'Si fuera rico'".to_string(),
            ),
            0,
        )]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"exploratory\""));
        assert!(json.contains("Use subjunctive mood"));
    }

    #[test]
    fn test_agent_response_deserialization_all_four_types() {
        let mistake_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hablar".as_bytes());
        let explained_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "órale".as_bytes());
        let translated_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "hello".as_bytes());
        let exploratory_id =
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "Use subjunctive".as_bytes());

        let json = format!(
            r#"{{
            "response": "Hello",
            "mistakes": [[{{
                "id": "{}",
                "specific_mistake": "hablar",
                "correction": "habla",
                "mistake_category": {{"type": "spelling_error", "context": "habla"}}
            }}, 0]],
            "explained": [[{{
                "id": "{}",
                "new_phrase": "órale",
                "explanation": "Mexican slang"
            }}, 0]],
            "translated": [[{{
                "id": "{}",
                "translated_word": "hello",
                "translated_to": "hola"
            }}, 0]],
            "exploratory": [[{{
                "id": "{}",
                "point_to_try": "Use subjunctive",
                "instructions_for_use": "Try it"
            }}, 0]]
        }}"#,
            mistake_id, explained_id, translated_id, exploratory_id
        );

        let response: AgentResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.response, "Hello");
        assert_eq!(response.mistakes.as_ref().unwrap().len(), 1);
        assert_eq!(response.explained.as_ref().unwrap().len(), 1);
        assert_eq!(response.translated.as_ref().unwrap().len(), 1);
        assert_eq!(response.exploratory.as_ref().unwrap().len(), 1);
        assert_eq!(response.translated.as_ref().unwrap()[0].0.id, translated_id);
        assert_eq!(
            response.exploratory.as_ref().unwrap()[0].0.id,
            exploratory_id
        );
    }
}
