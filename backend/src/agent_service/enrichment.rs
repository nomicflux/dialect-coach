use anyhow::{Result, anyhow};
use dialect_coach_shared::{
    Dialect, EnrichRequest, EnrichResponse, PartialLearningItem, PartialMistake, PartialTranslated,
    PartialExplained, PartialExploratory, Mistake, Translated, Explained, Exploratory, MistakeCategory,
};
use super::retry::RetryContext;
use super::util::GenerationConfig;

fn build_mistake_prompt(partial: &PartialMistake, dialect: Dialect) -> String {
    let mistake = partial.specific_mistake.as_ref().unwrap();
    let mut prompt = format!("The user made this mistake in {}: \"{}\"\n\n", dialect, mistake);

    if partial.correction.is_some() && partial.mistake_category.is_some() {
        return prompt;
    }

    prompt.push_str("Provide the correct version and categorize the mistake type.\n\n");
    prompt.push_str("Response format:\n");
    prompt.push_str("CORRECTION: [correct version]\n");
    prompt.push_str("CATEGORY: [SpellingError|VocabularyError|GrammarError|DialectUsageError|Other]\n");
    prompt.push_str("CONTEXT: [brief explanation for category context field]");

    prompt
}

fn build_translated_prompt(partial: &PartialTranslated, dialect: Dialect) -> String {
    if let Some(word) = &partial.translated_to {
        format!(
            "Translate this {} word to English: \"{}\"\n\nResponse format:\nENGLISH: [translation]",
            dialect, word
        )
    } else if let Some(word) = &partial.translated_word {
        format!(
            "Translate this English word to {}: \"{}\"\n\nResponse format:\nTRANSLATION: [translation]",
            dialect, word
        )
    } else {
        String::new()
    }
}

fn build_explained_prompt(partial: &PartialExplained, dialect: Dialect) -> String {
    if let Some(phrase) = &partial.new_phrase {
        format!(
            "Explain this {} phrase: \"{}\"\n\nResponse format:\nEXPLANATION: [what it means and how it's used]",
            dialect, phrase
        )
    } else if let Some(explanation) = &partial.explanation {
        format!(
            "What {} phrase matches this description: \"{}\"\n\nResponse format:\nPHRASE: [the phrase]",
            dialect, explanation
        )
    } else {
        String::new()
    }
}

fn build_exploratory_prompt(partial: &PartialExploratory, dialect: Dialect) -> String {
    if let Some(point) = &partial.point_to_try {
        format!(
            "The user wants to practice: \"{}\"\n\nProvide specific instructions with examples for {}.\n\nResponse format:\nINSTRUCTIONS: [concrete examples and usage guidance]",
            point, dialect
        )
    } else if let Some(instructions) = &partial.instructions_for_use {
        format!(
            "What practice point matches these instructions: \"{}\"\n\nResponse format:\nPOINT: [the practice point]",
            instructions
        )
    } else {
        String::new()
    }
}

fn extract_field(text: &str, prefix: &str) -> Option<String> {
    text.lines()
        .find(|line| line.trim().starts_with(prefix))
        .and_then(|line| line.split_once(':'))
        .map(|(_, value)| value.trim().to_string())
}

fn parse_category(text: &str, context: &str) -> Result<MistakeCategory> {
    match text.trim() {
        "SpellingError" => Ok(MistakeCategory::SpellingError { context: context.to_string() }),
        "VocabularyError" => Ok(MistakeCategory::VocabularyError { context: context.to_string() }),
        "GrammarError" => Ok(MistakeCategory::GrammarError { context: context.to_string() }),
        "DialectUsageError" => Ok(MistakeCategory::DialectUsageError { context: context.to_string() }),
        "Other" => Ok(MistakeCategory::Other { context: context.to_string() }),
        _ => Err(anyhow!("Unknown mistake category: {}", text))
    }
}

fn parse_mistake_response(text: &str, partial: &PartialMistake) -> Result<Mistake> {
    let correction = partial.correction.clone()
        .or_else(|| extract_field(text, "CORRECTION"))
        .ok_or_else(|| anyhow!("Missing correction"))?;

    let context = extract_field(text, "CONTEXT")
        .unwrap_or_else(|| "No context".to_string());

    let category = if let Some(cat_str) = extract_field(text, "CATEGORY") {
        parse_category(&cat_str, &context)?
    } else if partial.mistake_category.is_some() {
        parse_category(partial.mistake_category.as_ref().unwrap(), &context)?
    } else {
        MistakeCategory::Other { context: context.clone() }
    };

    Ok(Mistake::new(
        partial.specific_mistake.clone().unwrap(),
        correction,
        category,
    ))
}

fn parse_translated_response(text: &str, partial: &PartialTranslated) -> Result<Translated> {
    let (word, to) = if partial.translated_to.is_some() {
        let english = extract_field(text, "ENGLISH")
            .ok_or_else(|| anyhow!("Missing English translation"))?;
        (english, partial.translated_to.clone().unwrap())
    } else {
        let translation = extract_field(text, "TRANSLATION")
            .ok_or_else(|| anyhow!("Missing translation"))?;
        (partial.translated_word.clone().unwrap(), translation)
    };

    Ok(Translated::new(word, to, partial.context.clone()))
}

fn parse_explained_response(text: &str, partial: &PartialExplained) -> Result<Explained> {
    let phrase = partial.new_phrase.clone()
        .or_else(|| extract_field(text, "PHRASE"))
        .ok_or_else(|| anyhow!("Missing phrase"))?;

    let explanation = partial.explanation.clone()
        .or_else(|| extract_field(text, "EXPLANATION"))
        .ok_or_else(|| anyhow!("Missing explanation"))?;

    Ok(Explained::new(phrase, explanation))
}

fn parse_exploratory_response(text: &str, partial: &PartialExploratory) -> Result<Exploratory> {
    let point = partial.point_to_try.clone()
        .or_else(|| extract_field(text, "POINT"))
        .ok_or_else(|| anyhow!("Missing point"))?;

    let instructions = partial.instructions_for_use.clone()
        .or_else(|| extract_field(text, "INSTRUCTIONS"))
        .ok_or_else(|| anyhow!("Missing instructions"))?;

    Ok(Exploratory::new(point, instructions))
}

pub async fn enrich_partial_mistake(
    retry_ctx: &RetryContext,
    partial: &PartialMistake,
    dialect: Dialect,
) -> Result<Mistake> {
    let prompt = build_mistake_prompt(partial, dialect);
    let config = GenerationConfig {
        max_tokens: 1024,
        temperature: 0.2,
    };

    let request = super::provider::CompletionRequest {
        preamble: "You are a dialect expert helping users learn languages.",
        prompt: &prompt,
        history: &[],
        max_tokens: config.max_tokens,
        temperature: config.temperature,
    };

    let (result, _) = super::retry::retry_completion_call(retry_ctx.agent.as_ref(), &request, 3).await;
    let response = result?;

    parse_mistake_response(&response, partial)
}

pub async fn enrich_partial_translated(
    retry_ctx: &RetryContext,
    partial: &PartialTranslated,
    dialect: Dialect,
) -> Result<Translated> {
    let prompt = build_translated_prompt(partial, dialect);
    let config = GenerationConfig {
        max_tokens: 1024,
        temperature: 0.2,
    };

    let request = super::provider::CompletionRequest {
        preamble: "You are a dialect expert helping users learn languages.",
        prompt: &prompt,
        history: &[],
        max_tokens: config.max_tokens,
        temperature: config.temperature,
    };

    let (result, _) = super::retry::retry_completion_call(retry_ctx.agent.as_ref(), &request, 3).await;
    let response = result?;

    parse_translated_response(&response, partial)
}

pub async fn enrich_partial_explained(
    retry_ctx: &RetryContext,
    partial: &PartialExplained,
    dialect: Dialect,
) -> Result<Explained> {
    let prompt = build_explained_prompt(partial, dialect);
    let config = GenerationConfig {
        max_tokens: 1024,
        temperature: 0.2,
    };

    let request = super::provider::CompletionRequest {
        preamble: "You are a dialect expert helping users learn languages.",
        prompt: &prompt,
        history: &[],
        max_tokens: config.max_tokens,
        temperature: config.temperature,
    };

    let (result, _) = super::retry::retry_completion_call(retry_ctx.agent.as_ref(), &request, 3).await;
    let response = result?;

    parse_explained_response(&response, partial)
}

pub async fn enrich_partial_exploratory(
    retry_ctx: &RetryContext,
    partial: &PartialExploratory,
    dialect: Dialect,
) -> Result<Exploratory> {
    let prompt = build_exploratory_prompt(partial, dialect);
    let config = GenerationConfig {
        max_tokens: 1024,
        temperature: 0.2,
    };

    let request = super::provider::CompletionRequest {
        preamble: "You are a dialect expert helping users learn languages.",
        prompt: &prompt,
        history: &[],
        max_tokens: config.max_tokens,
        temperature: config.temperature,
    };

    let (result, _) = super::retry::retry_completion_call(retry_ctx.agent.as_ref(), &request, 3).await;
    let response = result?;

    parse_exploratory_response(&response, partial)
}

pub async fn enrich_learning_item(
    retry_ctx: &RetryContext,
    req: &EnrichRequest,
) -> Result<EnrichResponse> {
    let enriched_value = match &req.partial_data {
        PartialLearningItem::Mistake(partial) => {
            let item = enrich_partial_mistake(retry_ctx, partial, req.dialect).await?;
            serde_json::to_value(&item)?
        }
        PartialLearningItem::Translated(partial) => {
            let item = enrich_partial_translated(retry_ctx, partial, req.dialect).await?;
            serde_json::to_value(&item)?
        }
        PartialLearningItem::Explained(partial) => {
            let item = enrich_partial_explained(retry_ctx, partial, req.dialect).await?;
            serde_json::to_value(&item)?
        }
        PartialLearningItem::Exploratory(partial) => {
            let item = enrich_partial_exploratory(retry_ctx, partial, req.dialect).await?;
            serde_json::to_value(&item)?
        }
    };

    Ok(EnrichResponse {
        enriched_item: enriched_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mistake_prompt() {
        let partial = PartialMistake {
            specific_mistake: Some("Cómo andes".to_string()),
            correction: None,
            mistake_category: None,
        };
        let prompt = build_mistake_prompt(&partial, Dialect::SpanishMexican);
        assert!(prompt.contains("Cómo andes"));
        assert!(prompt.contains("Mexican Spanish"));
        assert!(prompt.contains("CORRECTION"));
        assert!(prompt.contains("CATEGORY"));
    }

    #[test]
    fn test_build_translated_prompt_english_to_dialect() {
        let partial = PartialTranslated {
            translated_word: Some("hello".to_string()),
            translated_to: None,
            context: None,
        };
        let prompt = build_translated_prompt(&partial, Dialect::SpanishMexican);
        assert!(prompt.contains("hello"));
        assert!(prompt.contains("English"));
        assert!(prompt.contains("Mexican Spanish"));
    }

    #[test]
    fn test_build_translated_prompt_dialect_to_english() {
        let partial = PartialTranslated {
            translated_word: None,
            translated_to: Some("hola".to_string()),
            context: None,
        };
        let prompt = build_translated_prompt(&partial, Dialect::SpanishMexican);
        assert!(prompt.contains("hola"));
        assert!(prompt.contains("English"));
        assert!(prompt.contains("Mexican Spanish"));
    }

    #[test]
    fn test_build_explained_prompt_phrase_to_explanation() {
        let partial = PartialExplained {
            new_phrase: Some("órale".to_string()),
            explanation: None,
        };
        let prompt = build_explained_prompt(&partial, Dialect::SpanishMexican);
        assert!(prompt.contains("órale"));
        assert!(prompt.contains("EXPLANATION"));
    }

    #[test]
    fn test_build_exploratory_prompt_point_to_instructions() {
        let partial = PartialExploratory {
            point_to_try: Some("Use subjunctive".to_string()),
            instructions_for_use: None,
        };
        let prompt = build_exploratory_prompt(&partial, Dialect::SpanishMexican);
        assert!(prompt.contains("Use subjunctive"));
        assert!(prompt.contains("INSTRUCTIONS"));
    }

    #[test]
    fn test_extract_field() {
        let text = "CORRECTION: Cómo andás\nCATEGORY: GrammarError";
        assert_eq!(extract_field(text, "CORRECTION"), Some("Cómo andás".to_string()));
        assert_eq!(extract_field(text, "CATEGORY"), Some("GrammarError".to_string()));
        assert_eq!(extract_field(text, "MISSING"), None);
    }

    #[test]
    fn test_parse_category() {
        let result = parse_category("GrammarError", "test context");
        assert!(result.is_ok());
        if let MistakeCategory::GrammarError { context } = result.unwrap() {
            assert_eq!(context, "test context");
        } else {
            panic!("Wrong category type");
        }
    }

    #[test]
    fn test_parse_mistake_response() {
        let partial = PartialMistake {
            specific_mistake: Some("Cómo andes".to_string()),
            correction: None,
            mistake_category: None,
        };
        let text = "CORRECTION: Cómo andás\nCATEGORY: GrammarError\nCONTEXT: verb conjugation";
        let result = parse_mistake_response(text, &partial);
        assert!(result.is_ok());
        let mistake = result.unwrap();
        assert_eq!(mistake.specific_mistake, "Cómo andes");
        assert_eq!(mistake.correction, "Cómo andás");
    }

    #[test]
    fn test_parse_translated_response_english_provided() {
        let partial = PartialTranslated {
            translated_word: None,
            translated_to: Some("hola".to_string()),
            context: None,
        };
        let text = "ENGLISH: hello";
        let result = parse_translated_response(text, &partial);
        assert!(result.is_ok());
        let translated = result.unwrap();
        assert_eq!(translated.translated_word, "hello");
        assert_eq!(translated.translated_to, "hola");
    }

    #[test]
    fn test_parse_translated_response_translation_provided() {
        let partial = PartialTranslated {
            translated_word: Some("hello".to_string()),
            translated_to: None,
            context: None,
        };
        let text = "TRANSLATION: hola";
        let result = parse_translated_response(text, &partial);
        assert!(result.is_ok());
        let translated = result.unwrap();
        assert_eq!(translated.translated_word, "hello");
        assert_eq!(translated.translated_to, "hola");
    }

    #[test]
    fn test_parse_explained_response() {
        let partial = PartialExplained {
            new_phrase: Some("órale".to_string()),
            explanation: None,
        };
        let text = "EXPLANATION: Mexican slang for wow";
        let result = parse_explained_response(text, &partial);
        assert!(result.is_ok());
        let explained = result.unwrap();
        assert_eq!(explained.new_phrase, "órale");
        assert_eq!(explained.explanation, "Mexican slang for wow");
    }

    #[test]
    fn test_parse_exploratory_response() {
        let partial = PartialExploratory {
            point_to_try: Some("Use subjunctive".to_string()),
            instructions_for_use: None,
        };
        let text = "INSTRUCTIONS: Try saying Si fuera rico";
        let result = parse_exploratory_response(text, &partial);
        assert!(result.is_ok());
        let exploratory = result.unwrap();
        assert_eq!(exploratory.point_to_try, "Use subjunctive");
        assert_eq!(exploratory.instructions_for_use, "Try saying Si fuera rico");
    }
}
