use dialect_coach_shared::models::agent::MistakeCategory;
use serde::Deserialize;

/// How the user handled an error discovered by the learning agent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorHandling {
    Corrected,
    SameError,
    DifferentError,
}

/// An error discovered by the learning agent in the assistant's message,
/// along with how the user handled it.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveredError {
    pub error_form: String,
    pub correct_form: String,
    pub handling: ErrorHandling,
    pub error_category: MistakeCategory,
}

/// Learning agent output for ErrorFinding mode.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorFindingLearningOutput {
    #[serde(default)]
    pub handled_errors: Vec<DiscoveredError>,
}

/// Maps error handling to initial score.
pub fn score_for_handling(handling: &ErrorHandling) -> u8 {
    match handling {
        ErrorHandling::Corrected => 10,
        ErrorHandling::SameError => 0,
        ErrorHandling::DifferentError => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_for_handling() {
        assert_eq!(score_for_handling(&ErrorHandling::Corrected), 10);
        assert_eq!(score_for_handling(&ErrorHandling::SameError), 0);
        assert_eq!(score_for_handling(&ErrorHandling::DifferentError), 5);
    }

    #[test]
    fn test_discovered_error_deserialization() {
        let json = r#"{
            "error_form": "esta",
            "correct_form": "está",
            "handling": "corrected",
            "error_category": {"type": "spelling_error", "context": "Missing accent"}
        }"#;
        let error: DiscoveredError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error_form, "esta");
        assert_eq!(error.correct_form, "está");
        assert_eq!(error.handling, ErrorHandling::Corrected);
    }

    #[test]
    fn test_learning_output_deserialization() {
        let json = r#"{"handled_errors": []}"#;
        let output: ErrorFindingLearningOutput = serde_json::from_str(json).unwrap();
        assert!(output.handled_errors.is_empty());
    }
}
