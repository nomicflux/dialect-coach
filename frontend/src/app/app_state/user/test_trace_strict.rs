#[cfg(test)]
mod tests {
    use crate::app::app_state::session::{SessionAction, SessionState};
    use crate::app::app_state::user::{SettingsAction, UserDomainAction};
    use dialect_coach_shared::models::{
        Dialect, DialectLevel, Language, LanguageLevel, UserState, CefrLevel, JlptLevel
    };
    use uuid::Uuid;
    use std::rc::Rc;
    use yew::prelude::Reducible;

    #[test]
    fn reproduce_level_leakage() {
        // Step 1: Initial State - Argentinian Spanish, Level B2.
        let mut user_state = UserState::new(Uuid::new_v4());
        user_state.selected_language = Language::Spanish;
        user_state.selected_dialect = Dialect::SpanishArgentinian;
        user_state.dialect_levels.push(DialectLevel::new(Dialect::SpanishArgentinian, LanguageLevel::Cefr(CefrLevel::B2)));
        
        // Ensure strictly clean state
        user_state.dialect_levels.retain(|dl| dl.dialect == Dialect::SpanishArgentinian);

        // Wrap in SessionState
        let session = Rc::new(SessionState {
            user: Some(user_state),
            needs_save: false,
        });

        println!("INIT: Dialect={:?}, Levels={:?}", session.user.as_ref().unwrap().selected_dialect, session.user.as_ref().unwrap().dialect_levels);

        // Step 2: Switch to Japanese
        // Corresponds to ChangeLanguage(Japanese)
        let action = SessionAction::Domain(UserDomainAction::Settings(SettingsAction::ChangeLanguage(Language::Japanese)));
        let session = session.reduce(action);

        let user = session.user.as_ref().unwrap();
        println!("STEP 2 (Japanese): Dialect={:?}, Levels={:?}", user.selected_dialect, user.dialect_levels);
        
        assert_eq!(user.selected_language, Language::Japanese);
        // Expect default Japanese dialect (Tokyo)
        assert_eq!(user.selected_dialect, Dialect::JapaneseTokyo);

        // Step 3: Change Level to N2
        // Corresponds to UpdateLevel(N2)
        // CRITICAL: This mimics the user selecting N2 from the dropdown
        let action = SessionAction::Domain(UserDomainAction::Settings(SettingsAction::UpdateLevel(LanguageLevel::Jlpt(JlptLevel::N2))));
        let session = session.reduce(action);

        let user = session.user.as_ref().unwrap();
        println!("STEP 3 (Set N2): Dialect={:?}, Levels={:?}", user.selected_dialect, user.dialect_levels);

        // Verify Tokyo is N2
        let tokyo_level = user.get_level_for_dialect(&Dialect::JapaneseTokyo);
        assert_eq!(tokyo_level, LanguageLevel::Jlpt(JlptLevel::N2));

        // VERIFY LEAKAGE: Did Spanish change?
        let spanish_level = user.get_level_for_dialect(&Dialect::SpanishArgentinian);
        println!("CHECK: Spanish Level = {:?}", spanish_level);
        
        if spanish_level == LanguageLevel::Jlpt(JlptLevel::N2) {
            panic!("BUG REPRODUCED: Spanish level was overwritten by Japanese N2 update!");
        }
        assert_eq!(spanish_level, LanguageLevel::Cefr(CefrLevel::B2), "Spanish level should remain B2");

        // Step 4: Switch back to Spanish
        let action = SessionAction::Domain(UserDomainAction::Settings(SettingsAction::ChangeLanguage(Language::Spanish)));
        let session = session.reduce(action);
        let user = session.user.as_ref().unwrap();
        println!("STEP 4 (Back to Spanish): Dialect={:?}, Levels={:?}", user.selected_dialect, user.dialect_levels);

        assert_eq!(user.selected_language, Language::Spanish);
        
        // Step 5: Check Levels Visualization
        // User says "Level already at N1" (in our case N2).
        let current_level = user.current_language_level();
        println!("Visualization: Current Level = {:?}", current_level);
        
        if current_level == LanguageLevel::Jlpt(JlptLevel::N2) {
             panic!("BUG REPRODUCED: Current level is N2 (Japanese) while on Spanish!");
        }
        assert_eq!(current_level, LanguageLevel::Cefr(CefrLevel::B2));

    }
}
