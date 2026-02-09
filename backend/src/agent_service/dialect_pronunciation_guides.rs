use dialect_coach_shared::Dialect;

const MEXICAN_SPANISH_GUIDE: &str = r#"
- /s/ fully retained in all positions (key distinguishing feature)
- /x/ as strong velar [x] (not glottal [h]): "mejor" = [meˈxoɾ]
- Trill /r/ → assibilated [ʐ], especially word-initially
- /tl/ cluster as single onset (Nahuatl substrate)
- Unstressed vowel reduction/devoicing near /s/: "trastes" → [ˈtɾasts]
- Word-final /n/ stays alveolar [n]
"#;

const ARGENTINIAN_SPANISH_GUIDE: &str = r#"
- Sheísmo: /ʝ/ → [ʃ]: "calle" = [ˈkaʃe], "yo" = [ʃo]
- /s/ → [h] before consonants: "esto" → [ˈehto]
- Word-final /ɾ/ frequently deleted: "comer" → [koˈme]
- Intervocalic /d/ deletion in -ado: "cansado" → [kanˈsao]
- Italian-substrate intonation: rising-falling pitch contours
- Voseo verb forms shift stress: "tenés" [teˈneh]
"#;

const CUBAN_SPANISH_GUIDE: &str = r#"
- /s/ → [h] or ∅ everywhere: "estos" → [ˈehtoh] or [ˈetoh]
- Lambdacism: /ɾ/ → [l]: "puerto" → [ˈpwelto], "amor" → [aˈmol]
- Liquid gemination: "pulpo" → [ˈpuppo], "caldo" → [ˈkaddo]
- /x/ → [h]: "gente" = [ˈhente]
- Final /n/ → [ŋ]: "pan" → [paŋ]
- Intervocalic /d/ deletion: "nada" → [ˈnaa]
"#;

const COLOMBIAN_SPANISH_GUIDE: &str = r#"
- Conservative: consonants fully articulated
- /s/ fully retained in all positions (highland conservative)
- /x/ → [h] (glottal, lighter than Mexican velar [x])
- /b d ɡ/ after ANY consonant → full plosives (distinctive)
- Intervocalic /b d ɡ/ may elide: "Bogota" → [bo.oˈta]
- Clean, crisp vowel articulation; no reduction
- Word-final /n/ stays alveolar [n]
"#;

const QUEBEC_FRENCH_GUIDE: &str = r#"
- Affrication: /t/ → [t͡s] before /i y/: "tu" → [t͡sy], "petit" → [pət͡si]
- Affrication: /d/ → [d͡z] before /i y/: "dis" → [d͡zi], "du" → [d͡zy]
- High vowel laxing in closed syllables: /i/ → [ɪ], /y/ → [ʏ], /u/ → [ʊ]
- Diphthongization of long vowels: /ɛː/ → [aɪ̯], /oː/ → [oʊ̯]
- Nasal vowels differ from Parisian: diphthongized
- Final cluster simplification: "table" → [tab], "quatre" → [kat]
- Schwa contractions: "je suis" → "chu" [ʃy]
"#;

const AFRICAN_FRENCH_GUIDE: &str = r#"
- /ʁ/ → [r] alveolar trill or [ɾ] tap (not uvular)
- Front rounded vowel derounding: /y/ → [i], /ø/ → [e], /œ/ → [ɛ]
- Nasal vowel simplification: /ɑ̃/ → [a]
- Mid-vowel mergers: /e/~/ɛ/ and /o/~/ɔ/ may merge
- Fewer vowel contrasts than Metropolitan French
- Syllable-timed, even stress distribution
"#;

pub fn build_dialect_pronunciation_guide(dialect: Dialect) -> &'static str {
    match dialect {
        Dialect::SpanishMexican => MEXICAN_SPANISH_GUIDE,
        Dialect::SpanishArgentinian => ARGENTINIAN_SPANISH_GUIDE,
        Dialect::SpanishCuban => CUBAN_SPANISH_GUIDE,
        Dialect::SpanishColombian => COLOMBIAN_SPANISH_GUIDE,
        Dialect::FrenchQuebecois => QUEBEC_FRENCH_GUIDE,
        Dialect::FrenchAfrican => AFRICAN_FRENCH_GUIDE,
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guide_mexican() {
        let guide = build_dialect_pronunciation_guide(Dialect::SpanishMexican);
        assert!(!guide.is_empty());
        assert!(guide.contains("/s/ fully retained"));
    }

    #[test]
    fn test_guide_argentinian() {
        let guide = build_dialect_pronunciation_guide(Dialect::SpanishArgentinian);
        assert!(!guide.is_empty());
        assert!(guide.contains("ʃ"));
    }

    #[test]
    fn test_guide_cuban() {
        let guide = build_dialect_pronunciation_guide(Dialect::SpanishCuban);
        assert!(!guide.is_empty());
        assert!(guide.contains("Lambdacism"));
    }

    #[test]
    fn test_guide_colombian() {
        let guide = build_dialect_pronunciation_guide(Dialect::SpanishColombian);
        assert!(!guide.is_empty());
        assert!(guide.contains("conservative"));
    }

    #[test]
    fn test_guide_quebec() {
        let guide = build_dialect_pronunciation_guide(Dialect::FrenchQuebecois);
        assert!(!guide.is_empty());
        assert!(guide.contains("Affrication"));
    }

    #[test]
    fn test_guide_african() {
        let guide = build_dialect_pronunciation_guide(Dialect::FrenchAfrican);
        assert!(!guide.is_empty());
        assert!(guide.contains("alveolar"));
    }

    #[test]
    fn test_guide_no_guide_dialect() {
        let guide = build_dialect_pronunciation_guide(Dialect::SpanishCastilian);
        assert!(guide.is_empty());
    }
}
