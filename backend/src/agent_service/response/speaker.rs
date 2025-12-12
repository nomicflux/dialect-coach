use dialect_coach_shared::{Dialect, DialectWithFeatures, Formality, Gender, TTSProviderType};

fn formality_description(base: &str, formality: &Formality) -> String {
    match formality {
        Formality::Formal => format!(
            "{} communicating in a professional, polite manner in a formal setting.",
            base
        ),
        Formality::ProfessionalCasual => format!(
            "{} communicating in a professional manner amongst colleagues, using more standard forms than usual but not being rigid in speech.",
            base
        ),
        Formality::Informal => format!(
            "{} speaking conversationally, using dialectal forms when appropriate and natural, and more standard forms when those become difficult to understand.",
            base
        ),
        Formality::Slang => format!("{} using informal slang and colloquialisms.", base),
    }
}

pub(crate) fn speaker_desc(dialect: &Dialect, formality: &Formality, gender: &Gender) -> String {
    let gender_str = match gender {
        Gender::MalePresenting => "male-presenting",
        Gender::FemalePresenting => "female-presenting",
    };
    let base = format!("You are a {} native {} speaker", gender_str, dialect.name());
    formality_description(&base, formality)
}

pub(crate) fn mimic_instruction(has_corpus: bool) -> &'static str {
    if has_corpus {
        "1. MIMIC THE PATTERNS: Study the dialect examples in the conversation history below and copy their vocabulary, grammar, style, and characteristic dialect constructions"
    } else {
        "1. MIMIC THE DIALECT: Use the vocabulary, grammar, style, and characteristic constructions for your dialect."
    }
}

pub(crate) fn extract_gender_from_dialect(dialect_with_features: &DialectWithFeatures) -> Gender {
    dialect_with_features
        .tts_voices
        .get(&TTSProviderType::ElevenLabs)
        .and_then(|voice| voice.as_ref().map(|v| v.gender))
        .unwrap_or(Gender::FemalePresenting)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::dialect_features;

    #[test]
    fn test_speaker_desc_male_presenting_formal() {
        let desc = speaker_desc(
            &Dialect::SpanishMexican,
            &Formality::Formal,
            &Gender::MalePresenting,
        );
        assert!(desc.contains("male-presenting"));
        assert!(desc.contains("Mexican Spanish"));
        assert!(desc.contains("professional, polite manner"));
    }

    #[test]
    fn test_speaker_desc_female_presenting_informal() {
        let desc = speaker_desc(
            &Dialect::SpanishArgentinian,
            &Formality::Informal,
            &Gender::FemalePresenting,
        );
        assert!(desc.contains("female-presenting"));
        assert!(desc.contains("Argentinian Spanish"));
        assert!(desc.contains("conversationally"));
    }

    #[test]
    fn test_speaker_desc_includes_gender_in_all_formalities() {
        let formalities = vec![
            Formality::Formal,
            Formality::ProfessionalCasual,
            Formality::Informal,
            Formality::Slang,
        ];
        for formality in formalities {
            let desc_male = speaker_desc(
                &Dialect::ArabicEgyptian,
                &formality,
                &Gender::MalePresenting,
            );
            let desc_female = speaker_desc(
                &Dialect::ArabicEgyptian,
                &formality,
                &Gender::FemalePresenting,
            );
            assert!(desc_male.contains("male-presenting"));
            assert!(desc_female.contains("female-presenting"));
        }
    }

    #[test]
    fn test_extract_gender_from_dialect_uses_elevenlabs() {
        let features = dialect_features(Dialect::SpanishArgentinian);
        let gender = extract_gender_from_dialect(&features);
        assert_eq!(
            gender,
            Gender::MalePresenting,
            "Should extract Male from ElevenLabs voice, not Female from Azure"
        );
    }
}
