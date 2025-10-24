use serde::{Deserialize, Serialize};

/// A mistake identified in user input
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mistake {
    pub content: String,
}

impl From<&str> for Mistake {
    fn from(content: &str) -> Self {
        Mistake {
            content: String::from(content),
        }
    }
}

impl From<String> for Mistake {
    fn from(content: String) -> Self {
        Mistake { content }
    }
}

impl From<&String> for Mistake {
    fn from(content: &String) -> Self {
        Mistake {
            content: content.clone(),
        }
    }
}

/// An explanation provided to the user
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Explained {
    pub content: String,
}

impl From<&str> for Explained {
    fn from(content: &str) -> Self {
        Explained {
            content: String::from(content),
        }
    }
}

impl From<String> for Explained {
    fn from(content: String) -> Self {
        Explained { content }
    }
}

impl From<&String> for Explained {
    fn from(content: &String) -> Self {
        Explained {
            content: content.clone(),
        }
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
    fn test_mistake_from_str() {
        let mistake = Mistake::from("wrong word");
        assert_eq!(mistake.content, "wrong word");
    }

    #[test]
    fn test_mistake_from_string() {
        let mistake = Mistake::from("wrong word".to_string());
        assert_eq!(mistake.content, "wrong word");
    }

    #[test]
    fn test_mistake_from_string_ref() {
        let content = "wrong word".to_string();
        let mistake = Mistake::from(&content);
        assert_eq!(mistake.content, "wrong word");
    }

    #[test]
    fn test_explained_from_str() {
        let explained = Explained::from("grammar explanation");
        assert_eq!(explained.content, "grammar explanation");
    }

    #[test]
    fn test_explained_from_string() {
        let explained = Explained::from("grammar explanation".to_string());
        assert_eq!(explained.content, "grammar explanation");
    }

    #[test]
    fn test_explained_from_string_ref() {
        let content = "grammar explanation".to_string();
        let explained = Explained::from(&content);
        assert_eq!(explained.content, "grammar explanation");
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
        response.mistakes = Some(vec![
            Mistake::from("mistake 1"),
            Mistake::from("mistake 2"),
        ]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"mistakes\""));
        assert!(json.contains("mistake 1"));
        assert!(json.contains("mistake 2"));
    }

    #[test]
    fn test_agent_response_serialization_with_explained() {
        let mut response = AgentResponse::from("Hello");
        response.explained = Some(vec![
            Explained::from("explanation 1"),
            Explained::from("explanation 2"),
        ]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"explained\""));
        assert!(json.contains("explanation 1"));
        assert!(json.contains("explanation 2"));
    }

    #[test]
    fn test_agent_response_deserialization_backward_compatibility() {
        // Old JSON format without mistakes/explained fields
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
            "mistakes": [{"content": "error"}],
            "explained": [{"content": "explanation"}]
        }"#;

        let response: AgentResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.response, "Hello");
        assert_eq!(response.mistakes.as_ref().unwrap().len(), 1);
        assert_eq!(response.mistakes.unwrap()[0].content, "error");
        assert_eq!(response.explained.as_ref().unwrap().len(), 1);
        assert_eq!(response.explained.unwrap()[0].content, "explanation");
    }
}
