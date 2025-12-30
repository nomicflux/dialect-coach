
    #[test]
    fn test_default_dialect_leakage_reproduction() {
        // user reports show_experimental was TRUE
        let show_experimental = true;
        
        // When switching to Japanese
        let default_dialect = UserState::default_dialect_for_language(Language::Japanese, show_experimental)
            .expect("Should find a dialect for Japanese");
        
        // It SHOULD be a Japanese dialect
        assert_eq!(default_dialect.language(), Language::Japanese, "User reported leakage where Japanese selected SpanishArgentinian");
        
        // Specifically, it should ideally be Tokyo if available
        assert_eq!(default_dialect, Dialect::JapaneseTokyo);
    }

    #[test]
    fn test_default_dialect_fallback_safety() {
        // Even if show_experimental is FALSE
        let show_experimental = false;
         
        // And we ask for Japanese (which only has experimental dialects currently)
        let default_dialect = UserState::default_dialect_for_language(Language::Japanese, show_experimental)
             .expect("Should find a fallback dialect even if experimental is hidden");
        
        // It MUST still be Japanese to avoid leakage
        assert_eq!(default_dialect.language(), Language::Japanese, "Fallback logic leaked to different language!");
    }
