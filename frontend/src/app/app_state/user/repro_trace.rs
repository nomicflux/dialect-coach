
#[cfg(test)]
mod tests {
    use crate::app::app_state::user::reducer::reduce_settings;
    use crate::app::app_state::SettingsAction;
    use dialect_coach_shared::models::{
        Dialect, DialectLevel, Language, LanguageLevel, UserState, InitialUserSettings, UserGender, CefrLevel, JlptLevel
    };
    use uuid::Uuid;
    use std::collections::HashMap;

    #[test]
    fn strict_logic_trace_repro() {
        // Step 1: Initial State - Argentinian Spanish, Level B2.
        // User implied they had saved state for Tokyo N5 as well.
        // We simulate this by creating initial state manually.
        let mut user_state = UserState::new(Uuid::new_v4());
        
        // Setup initial explicit state matching user description
        user_state.selected_language = Language::Spanish;
        user_state.selected_dialect = Dialect::SpanishArgentinian;
        user_state.dialect_levels.push(DialectLevel::new(Dialect::SpanishArgentinian, LanguageLevel::Cefr(CefrLevel::B2)));
        user_state.dialect_levels.push(DialectLevel::new(Dialect::JapaneseTokyo, LanguageLevel::Jlpt(JlptLevel::N5))); // "Saved state"
        
        // Initial Prefs (Assuming correct behavior at start)
        user_state.set_preferred_dialect(Dialect::SpanishArgentinian);
        
        println!("Step 1: Start. Dialect: {:?}, Level: {:?}", user_state.selected_dialect, user_state.current_language_level());
        assert_eq!(user_state.selected_dialect, Dialect::SpanishArgentinian);

        // Step 2: Select Japanese.
        reduce_settings(&mut user_state, SettingsAction::ChangeLanguage(Language::Japanese));
        println!("Step 2: Japanese. Dialect: {:?}, Level: {:?}", user_state.selected_dialect, user_state.current_language_level());
        assert_eq!(user_state.selected_dialect, Dialect::JapaneseTokyo);
        // User says "Already N5". (We pre-seeded N5).
        assert_eq!(user_state.current_language_level(), LanguageLevel::Jlpt(JlptLevel::N5));

        // Step 3: Change level to N2.
        reduce_settings(&mut user_state, SettingsAction::UpdateLevel(LanguageLevel::Jlpt(JlptLevel::N2)));
        println!("Step 3: Update N2. Level: {:?}", user_state.current_language_level());
        assert_eq!(user_state.current_language_level(), LanguageLevel::Jlpt(JlptLevel::N2));
        // Verify Memory
        let tokyo_level = user_state.get_level_for_dialect(&Dialect::JapaneseTokyo);
        assert_eq!(tokyo_level, LanguageLevel::Jlpt(JlptLevel::N2));

        // Step 4: Change language to Spanish.
        reduce_settings(&mut user_state, SettingsAction::ChangeLanguage(Language::Spanish));
        println!("Step 4: Spanish. Dialect: {:?}, Level: {:?}", user_state.selected_dialect, user_state.current_language_level());
        
        // BUG CHECK 1: User says Mexican. If valid logic, should be Argentinian (Prefs).
        if user_state.selected_dialect == Dialect::SpanishMexican {
            println!("BUG CONFIRMED: Defaulted to Mexican, ignoring Argentinian preference.");
        } else {
            println!("Logic Correct: Restored Argentinian.");
        }

        // Step 5: Switch to Argentinian (if not already).
        if user_state.selected_dialect != Dialect::SpanishArgentinian {
            reduce_settings(&mut user_state, SettingsAction::ChangeDialect(Dialect::SpanishArgentinian));
        }
        println!("Step 5: Argentinian. Level: {:?}", user_state.current_language_level());
        // User says "Level already at B2".
        assert_eq!(user_state.current_language_level(), LanguageLevel::Cefr(CefrLevel::B2));

        // Step 6: Change level to A1.
        reduce_settings(&mut user_state, SettingsAction::UpdateLevel(LanguageLevel::Cefr(CefrLevel::A1)));
        println!("Step 6: Update A1. Level: {:?}", user_state.current_language_level());
        assert_eq!(user_state.current_language_level(), LanguageLevel::Cefr(CefrLevel::A1));

        // Step 7: Switch to Japanese.
        reduce_settings(&mut user_state, SettingsAction::ChangeLanguage(Language::Japanese));
        println!("Step 7: Japanese. Dialect: {:?}, Level: {:?}", user_state.selected_dialect, user_state.current_language_level());
        
        // BUG CHECK 2: User says N5. Logic should say N2 (from Step 3).
        if user_state.current_language_level() == LanguageLevel::Jlpt(JlptLevel::N5) {
             println!("BUG CONFIRMED: Level reset to N5 (Contamination from A1?).");
        } else if user_state.current_language_level() == LanguageLevel::Jlpt(JlptLevel::N2) {
             println!("Logic Correct: Preserved N2.");
        } else {
             println!("Unknown State: {:?}", user_state.current_language_level());
        }
        
    }
}
