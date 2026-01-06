use dialect_coach_shared::{DialectDocument, Formality};
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

pub(super) fn deduplicate_examples(
    examples: Vec<DialectDocument>,
    random_samples: Vec<DialectDocument>,
) -> Vec<DialectDocument> {
    let mut all_examples = Vec::new();
    all_examples.extend(examples);
    all_examples.extend(random_samples);

    let mut seen = std::collections::HashSet::new();
    all_examples
        .into_iter()
        .filter(|doc| seen.insert(doc.content.clone()))
        .take(50)
        .collect()
}

pub(super) fn group_examples_by_formality(
    examples: &[DialectDocument],
    formality: Formality,
    primary_limit: usize,
    secondary_limit: usize,
) -> (Vec<DialectDocument>, Vec<DialectDocument>) {
    let primary_examples: Vec<_> = examples
        .iter()
        .filter(|doc| doc.formality.is_none() || doc.formality == Some(formality))
        .take(primary_limit)
        .cloned()
        .collect();

    let secondary_examples: Vec<_> = examples
        .iter()
        .filter(|doc| {
            doc.formality.is_some() && doc.formality != Some(formality) && doc.formality.is_some()
        })
        .take(secondary_limit)
        .cloned()
        .collect();

    (primary_examples, secondary_examples)
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
    fn test_deduplicate_examples() {
        let doc1 = DialectDocument::new("Hello".to_string(), Dialect::SpanishMexican, None);
        let doc2 = DialectDocument::new("Hola".to_string(), Dialect::SpanishMexican, None);
        let doc3 = DialectDocument::new("Hello".to_string(), Dialect::SpanishMexican, None);

        let examples = vec![doc1.clone(), doc2.clone()];
        let random_samples = vec![doc3];

        let result = deduplicate_examples(examples, random_samples);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "Hello");
        assert_eq!(result[1].content, "Hola");
    }

    #[test]
    fn test_deduplicate_examples_respects_limit() {
        let mut examples = Vec::new();
        for i in 0..60 {
            examples.push(DialectDocument::new(
                format!("Example {}", i),
                Dialect::SpanishMexican,
                None,
            ));
        }

        let result = deduplicate_examples(examples, Vec::new());
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn test_group_examples_by_formality() {
        let dialect = Dialect::SpanishMexican;
        let doc1 = DialectDocument::new("Hello".to_string(), dialect, Some(Formality::Informal));
        let doc2 = DialectDocument::new("Hola".to_string(), dialect, Some(Formality::Formal));
        let doc3 = DialectDocument::new("Hey".to_string(), dialect, None);
        let doc4 = DialectDocument::new("Hi".to_string(), dialect, Some(Formality::Informal));

        let examples = vec![doc1, doc2, doc3, doc4];
        let (primary, secondary) =
            group_examples_by_formality(&examples, Formality::Informal, 10, 10);

        assert_eq!(primary.len(), 3);
        assert_eq!(secondary.len(), 1);
        assert_eq!(secondary[0].formality, Some(Formality::Formal));
    }

    #[test]
    fn test_group_examples_by_formality_respects_limits() {
        let dialect = Dialect::SpanishMexican;
        let mut examples = Vec::new();
        for i in 0..15 {
            examples.push(DialectDocument::new(
                format!("Example {}", i),
                dialect,
                Some(Formality::Informal),
            ));
        }

        let (primary, _secondary) =
            group_examples_by_formality(&examples, Formality::Informal, 5, 5);
        assert_eq!(primary.len(), 5);
    }

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
