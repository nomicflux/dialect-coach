use serde::{Deserialize, Serialize};

/// Response from the AI agent service
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentResponse {
    pub response: String,
}

impl AgentResponse {
    fn new(response: String) -> Self {
        AgentResponse {
            response,
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
