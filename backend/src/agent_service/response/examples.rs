use dialect_coach_shared::DialectDocument;
use rig::completion::{Message as RigMessage, message::Text, message::UserContent};
use rig::one_or_many::OneOrMany;

use crate::agent_service::provider::ANTHROPIC_PROVIDER;
use crate::agent_service::util::create_prefilled_assistant_message;

pub fn format_examples_as_user_message(
    primary_examples: &[&DialectDocument],
    secondary_examples: &[&DialectDocument],
) -> String {
    if primary_examples.is_empty() && secondary_examples.is_empty() {
        return String::new();
    }

    let mut examples_text =
        "*** DIALECT EXAMPLES (DISTINCT FROM USER CONVERSATION): ***\n".to_string();

    for doc in primary_examples {
        examples_text.push_str(&format!("\"{}\"\n", doc.content));
    }

    for doc in secondary_examples {
        examples_text.push_str(&format!("\"{}\"\n", doc.content));
    }

    examples_text.push_str("*** END OF DIALECT EXAMPLES ***");

    examples_text
}

fn create_examples_user_message(examples_text: &str) -> RigMessage {
    if examples_text.is_empty() {
        return RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: String::new(),
            })),
        };
    }

    RigMessage::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: examples_text.to_string(),
        })),
    }
}

pub fn build_examples_message(
    primary_examples: &[&DialectDocument],
    secondary_examples: &[&DialectDocument],
) -> Option<RigMessage> {
    if primary_examples.is_empty() && secondary_examples.is_empty() {
        return None;
    }

    let examples_text = format_examples_as_user_message(primary_examples, secondary_examples);
    Some(create_examples_user_message(&examples_text))
}

/// Merge results from multiple search sources, deduplicating by content
/// Each search source already has its own limit from RAGConfig
pub(super) fn merge_search_results(
    results: Vec<Vec<(DialectDocument, f32)>>,
) -> Vec<DialectDocument> {
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();

    for result_set in results {
        for (doc, _score) in result_set {
            if seen.insert(doc.content.clone()) {
                merged.push(doc);
            }
        }
    }

    merged
}

/// Deduplicate results, excluding content already seen in primary results
/// Each search source already has its own limit from RAGConfig
pub(super) fn deduplicate_excluding(
    results: Vec<Vec<(DialectDocument, f32)>>,
    exclude: &[DialectDocument],
) -> Vec<DialectDocument> {
    let excluded_content: std::collections::HashSet<_> =
        exclude.iter().map(|d| d.content.clone()).collect();

    let mut seen = excluded_content;
    let mut deduped = Vec::new();

    for result_set in results {
        for (doc, _score) in result_set {
            if seen.insert(doc.content.clone()) {
                deduped.push(doc);
            }
        }
    }

    deduped
}

pub(crate) fn build_conversation_history_with_examples(
    conversation_history: &[RigMessage],
    primary_examples: &[DialectDocument],
    secondary_examples: &[DialectDocument],
    provider: &str,
) -> Vec<RigMessage> {
    let primary_refs: Vec<_> = primary_examples.iter().collect();
    let secondary_refs: Vec<_> = secondary_examples.iter().collect();
    let examples_message = build_examples_message(&primary_refs, &secondary_refs);
    let mut history_with_prefill = conversation_history.to_vec();

    if let Some(examples) = examples_message {
        history_with_prefill.insert(0, examples);
    }

    // Only add prefilled assistant message for Anthropic
    // OpenAI uses response_format parameter instead
    if provider == ANTHROPIC_PROVIDER {
        history_with_prefill.push(create_prefilled_assistant_message());
    }

    history_with_prefill
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::{Dialect, Formality};

    #[test]
    fn test_build_conversation_history_with_examples() {
        use crate::agent_service::provider::ANTHROPIC_PROVIDER;
        use rig::completion::{Message as RigMessage, message::Text, message::UserContent};
        use rig::one_or_many::OneOrMany;

        let dialect = Dialect::SpanishMexican;
        let doc1 = DialectDocument::new("Hello".to_string(), dialect, Some(Formality::Informal));
        let doc2 = DialectDocument::new("Hola".to_string(), dialect, Some(Formality::Formal));

        let conversation_history = vec![RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "Test".to_string(),
            })),
        }];

        let primary_examples = vec![doc1];
        let secondary_examples = vec![doc2];

        let history = build_conversation_history_with_examples(
            &conversation_history,
            &primary_examples,
            &secondary_examples,
            ANTHROPIC_PROVIDER,
        );

        assert_eq!(history.len(), 3);
        match &history[0] {
            RigMessage::User { .. } => {}
            _ => panic!("First message should be examples user message"),
        }
        match &history[1] {
            RigMessage::User { .. } => {}
            _ => panic!("Second message should be original history"),
        }
        match &history[2] {
            RigMessage::Assistant { .. } => {}
            _ => panic!("Last message should be prefilled assistant message"),
        }
    }

    #[test]
    fn test_build_conversation_history_without_examples() {
        use crate::agent_service::provider::ANTHROPIC_PROVIDER;
        use rig::completion::{Message as RigMessage, message::Text, message::UserContent};
        use rig::one_or_many::OneOrMany;

        let conversation_history = vec![RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "Test".to_string(),
            })),
        }];

        let history = build_conversation_history_with_examples(
            &conversation_history,
            &[],
            &[],
            ANTHROPIC_PROVIDER,
        );

        assert_eq!(history.len(), 2);
        match &history[0] {
            RigMessage::User { .. } => {}
            _ => panic!("First message should be original history"),
        }
        match &history[1] {
            RigMessage::Assistant { .. } => {}
            _ => panic!("Last message should be prefilled assistant message"),
        }
    }
}
