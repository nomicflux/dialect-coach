use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use super::dialect::dialect_features;
use super::{
    ConversationBranch, ConversationContext, Dialect, DialectWithFeatures, Formality,
    InitialUserSettings, Language, LanguageOption, LanguageOptions, LanguagePlan, LearningGoal,
    LearningItem, LearningItemType, Message, MessageMetadata, PastLearningItems, TeachingMode,
    UsageStats,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserGender {
    Male,
    Female,
    NonBinary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CefrLevel {
    A1,
    A2,
    #[default]
    B1,
    B2,
    C1,
    C2,
}

impl CefrLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::A1 => "A1 - Beginner",
            Self::A2 => "A2 - Elementary",
            Self::B1 => "B1 - Intermediate",
            Self::B2 => "B2 - Upper Intermediate",
            Self::C1 => "C1 - Advanced",
            Self::C2 => "C2 - Proficient",
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::A1 => "a1",
            Self::A2 => "a2",
            Self::B1 => "b1",
            Self::B2 => "b2",
            Self::C1 => "c1",
            Self::C2 => "c2",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "a1" => Some(Self::A1),
            "a2" => Some(Self::A2),
            "b1" => Some(Self::B1),
            "b2" => Some(Self::B2),
            "c1" => Some(Self::C1),
            "c2" => Some(Self::C2),
            _ => None,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::A1, Self::A2, Self::B1, Self::B2, Self::C1, Self::C2]
    }

    pub fn to_jlpt(&self) -> JlptLevel {
        match self {
            Self::A1 => JlptLevel::N5,
            Self::A2 => JlptLevel::N4,
            Self::B1 => JlptLevel::N3,
            Self::B2 => JlptLevel::N2,
            Self::C1 | Self::C2 => JlptLevel::N1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JlptLevel {
    N5,
    N4,
    #[default]
    N3,
    N2,
    N1,
}

impl JlptLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::N5 => "N5 - Beginner",
            Self::N4 => "N4 - Elementary",
            Self::N3 => "N3 - Intermediate",
            Self::N2 => "N2 - Upper Intermediate",
            Self::N1 => "N1 - Advanced",
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::N5 => "n5",
            Self::N4 => "n4",
            Self::N3 => "n3",
            Self::N2 => "n2",
            Self::N1 => "n1",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "n5" => Some(Self::N5),
            "n4" => Some(Self::N4),
            "n3" => Some(Self::N3),
            "n2" => Some(Self::N2),
            "n1" => Some(Self::N1),
            _ => None,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::N5, Self::N4, Self::N3, Self::N2, Self::N1]
    }

    pub fn to_cefr(&self) -> CefrLevel {
        match self {
            Self::N5 => CefrLevel::A1,
            Self::N4 => CefrLevel::A2,
            Self::N3 => CefrLevel::B1,
            Self::N2 => CefrLevel::B2,
            Self::N1 => CefrLevel::C1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageLevel {
    Cefr(CefrLevel),
    Jlpt(JlptLevel),
}

impl Default for LanguageLevel {
    fn default() -> Self {
        Self::Cefr(CefrLevel::B1)
    }
}

impl LanguageLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cefr(level) => level.name(),
            Self::Jlpt(level) => level.name(),
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::Cefr(level) => level.id(),
            Self::Jlpt(level) => level.id(),
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        if let Some(cefr) = CefrLevel::from_id(id) {
            return Some(Self::Cefr(cefr));
        }
        if let Some(jlpt) = JlptLevel::from_id(id) {
            return Some(Self::Jlpt(jlpt));
        }
        None
    }

    pub fn for_language(language: Language) -> Vec<Self> {
        match language {
            Language::Japanese => JlptLevel::all().into_iter().map(Self::Jlpt).collect(),
            _ => CefrLevel::all().into_iter().map(Self::Cefr).collect(),
        }
    }

    pub fn default_for_language(language: Language) -> Self {
        match language {
            Language::Japanese => Self::Jlpt(JlptLevel::default()),
            _ => Self::Cefr(CefrLevel::default()),
        }
    }

    pub fn convert_for_language(self, lang: Language) -> Self {
        match (self, lang) {
            (Self::Cefr(c), Language::Japanese) => Self::Jlpt(c.to_jlpt()),
            (Self::Jlpt(j), l) if l != Language::Japanese => Self::Cefr(j.to_cefr()),
            _ => self,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialectLevel {
    pub dialect: Dialect,
    pub level: LanguageLevel,
}

impl DialectLevel {
    pub fn new(dialect: Dialect, level: LanguageLevel) -> Self {
        Self { dialect, level }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserState {
    pub user_id: Uuid,
    pub learning_items: Arc<Vec<LearningItem>>,
    pub conversation_history: Arc<Vec<Message>>,
    pub tts_enabled: bool,
    pub selected_language: Language,
    pub selected_dialect: Dialect,
    pub formality: Formality,
    pub teaching_mode: TeachingMode,
    pub user_gender: UserGender,
    pub active_branch_id: Uuid,
    pub branches: Arc<Vec<ConversationBranch>>,
    pub learning_goals: Arc<Vec<LearningGoal>>,
    pub language_plans: Arc<Vec<LanguagePlan>>,
    pub active_plan_id: Option<Uuid>,
    pub usage_stats: UsageStats,
    pub language_options: LanguageOptions,
    pub show_experimental_dialects: bool,
    pub dialect_levels: Vec<DialectLevel>,
    #[serde(default)]
    pub is_admin: bool,
}

impl UserState {
    pub fn msg_by_id(&self, msg_id: Uuid) -> Option<Message> {
        self.conversation_history
            .iter()
            .find(|&msg| msg.id == msg_id)
            .cloned()
    }

    pub fn get_learning_items_for_dialect(&self, dialect: &Dialect) -> Vec<&LearningItem> {
        self.learning_items
            .iter()
            .filter(|item| &item.dialect == dialect)
            .collect()
    }

    pub fn get_learning_goals_for_dialect(&self, dialect: &Dialect) -> Vec<&LearningGoal> {
        self.learning_goals
            .iter()
            .filter(|goal| &goal.dialect == dialect)
            .collect()
    }

    pub fn active_plan(&self) -> Option<LanguagePlan> {
        self.active_plan_id.and_then(|id| {
            self.language_plans
                .iter()
                .find(|plan| plan.id == id)
                .cloned()
        })
    }
}

impl UserState {
    pub fn default_dialect_for_language(
        language: Language,
        show_experimental: bool,
    ) -> Option<Dialect> {
        let all_for_language = Dialect::for_language(language, false, false);

        // 1. Try to find one that matches the experimental filter
        let filtered = all_for_language
            .iter()
            .map(|&d| dialect_features(d))
            .filter(|d| show_experimental || !d.is_experimental)
            .map(|d| d.dialect)
            .next();

        if let Some(d) = filtered {
            return Some(d);
        }

        // 2. Fallback: If filter removed everything (e.g. Japanese only has experimental dialects),
        // return the first available dialect for THIS language.
        // This prevents leaking "SpanishArgentinian" into "Japanese" state.
        if let Some(&first) = all_for_language.first() {
            return Some(first);
        }

        // 3. No dialects found for language at all
        None
    }

    pub fn with_initial_settings(
        user_id: Uuid,
        initial_settings: Option<InitialUserSettings>,
    ) -> Self {
        let show_experimental_dialects = false;

        let (language, dialect, gender, dialect_levels) = match initial_settings {
            Some(s) => {
                let dl = DialectLevel::new(s.dialect, s.level);
                (s.language, s.dialect, s.gender, vec![dl])
            }
            None => {
                let lang = Language::Spanish;
                let dial = Self::default_dialect_for_language(lang, show_experimental_dialects)
                    .expect("Spanish language must have available dialects");
                (lang, dial, UserGender::NonBinary, Vec::new())
            }
        };

        let initial_branch = ConversationBranch::new(None, None, None, dialect, vec![], None);
        let initial_branch_id = initial_branch.id;

        Self {
            user_id,
            learning_items: Arc::new(Vec::new()),
            conversation_history: Arc::new(Vec::new()),
            tts_enabled: false,
            selected_language: language,
            selected_dialect: dialect,
            formality: Formality::Informal,
            teaching_mode: TeachingMode::Immersive,
            user_gender: gender,
            active_branch_id: initial_branch_id,
            branches: Arc::new(vec![initial_branch]),
            learning_goals: Arc::new(Vec::new()),
            language_plans: Arc::new(Vec::new()),
            active_plan_id: None,
            usage_stats: UsageStats::default(),
            language_options: LanguageOptions::default(),
            show_experimental_dialects,
            dialect_levels,
            is_admin: false,
        }
    }

    pub fn new(user_id: Uuid) -> Self {
        Self::with_initial_settings(user_id, None)
    }

    fn create_metadata(&self, session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            self.formality,
            self.teaching_mode,
            self.selected_language,
            self.current_dialect(),
            session_id,
        )
    }

    pub fn get_past_learning_items(&self, dialect: &Dialect) -> PastLearningItems {
        let mut items = PastLearningItems::default();
        for item in self.get_learning_items_for_dialect(dialect) {
            match &item.item {
                LearningItemType::Mistake(m) => items.mistakes.push(m.clone()),
                LearningItemType::Explanation(e) => items.explained.push(e.clone()),
                LearningItemType::Translation(t) => items.translated.push(t.clone()),
                LearningItemType::Exploration(e) => items.exploratory.push(e.clone()),
            }
        }
        items
    }

    pub fn build_action_context(&self) -> ConversationContext {
        let past_items = self.get_past_learning_items(&self.selected_dialect);
        ConversationContext {
            active_plan: self.active_plan(),
            learning_goals: self
                .get_learning_goals_for_dialect(&self.selected_dialect)
                .into_iter()
                .cloned()
                .collect(),
            past_mistakes: past_items.mistakes,
            past_explained: past_items.explained,
            past_translated: past_items.translated,
            past_exploratory: past_items.exploratory,
            user_gender: self.user_gender,
            language_option: self.current_language_option(),
            dialect: self.selected_dialect,
            formality: self.formality,
            teaching_mode: self.teaching_mode,
            language_level: self.current_language_level(),
        }
    }

    pub fn create_user_msg(&self, session_id: Uuid, content: &str) -> Message {
        Message::user_message(content.to_string(), self.create_metadata(session_id), None)
    }

    pub fn current_dialect(&self) -> Dialect {
        self.selected_dialect
    }

    pub fn current_language_option(&self) -> Option<LanguageOption> {
        self.language_options.for_language(self.selected_language)
    }

    pub fn get_level_for_dialect(&self, dialect: &Dialect) -> LanguageLevel {
        self.dialect_levels
            .iter()
            .find(|dl| &dl.dialect == dialect)
            .map(|dl| dl.level)
            .unwrap_or_default()
    }

    pub fn set_level_for_dialect(&mut self, dialect: Dialect, level: LanguageLevel) {
        if let Some(dl) = self
            .dialect_levels
            .iter_mut()
            .find(|dl| dl.dialect == dialect)
        {
            dl.level = level;
        } else {
            self.dialect_levels.push(DialectLevel::new(dialect, level));
        }
    }

    pub fn current_language_level(&self) -> LanguageLevel {
        self.get_level_for_dialect(&self.selected_dialect)
    }

    pub fn current_dialects(&self) -> Vec<DialectWithFeatures> {
        Dialect::for_language(self.selected_language, false, false)
            .into_iter()
            .map(dialect_features)
            .filter(|d| self.show_experimental_dialects || !d.is_experimental)
            .collect()
    }

    pub fn formality_display(&self) -> &'static str {
        match self.formality {
            Formality::Formal => "Formal",
            Formality::ProfessionalCasual => "Professional Casual",
            Formality::Informal => "Informal",
            Formality::Slang => "Slang",
        }
    }

    pub fn teaching_mode_display(&self) -> &'static str {
        match self.teaching_mode {
            TeachingMode::Immersive => "Immersive",
            TeachingMode::Corrective => "Corrective",
            TeachingMode::Explanatory => "Explanatory",
            TeachingMode::StoryTeller => "Storyteller",
            TeachingMode::ErrorFinding => "Find Agent Errors",
            TeachingMode::Debug => "Debug",
        }
    }

    pub fn get_active_branch_messages(&self) -> Vec<&Message> {
        if self.branches.is_empty() {
            return Vec::new();
        }

        let branch_id = if self
            .branches
            .iter()
            .any(|branch| branch.id == self.active_branch_id)
        {
            self.active_branch_id
        } else {
            self.select_active_branch(&self.branches)
        };

        let branch = match self.branches.iter().find(|b| b.id == branch_id) {
            Some(b) => b,
            None => return Vec::new(),
        };

        branch
            .message_ids
            .iter()
            .filter_map(|msg_id| self.conversation_history.iter().find(|m| m.id == *msg_id))
            .collect()
    }

    pub fn get_path_to_message(&self, leaf_id: Option<Uuid>) -> Vec<&Message> {
        let mut path = Vec::new();
        let mut current = leaf_id;

        while let Some(msg_id) = current {
            if let Some(msg) = self.conversation_history.iter().find(|m| m.id == msg_id) {
                path.push(msg);
                current = msg.parent_id;
            } else {
                break;
            }
        }

        path.reverse();
        path
    }

    pub fn get_child_branches(&self, message_id: Uuid) -> Vec<&ConversationBranch> {
        self.branches
            .iter()
            .filter(|branch| branch.parent_message_id == Some(message_id))
            .collect()
    }

    pub fn find_branch_root(&self, branch_id: Uuid) -> Option<Uuid> {
        self.branches
            .iter()
            .find(|branch| branch.id == branch_id)
            .and_then(|branch| branch.parent_message_id)
    }

    pub fn migrate_branch_message_ids(&mut self) -> bool {
        let migrations: Vec<(usize, Vec<Uuid>)> = self
            .branches
            .iter()
            .enumerate()
            .filter_map(|(idx, branch)| {
                if branch.message_ids.is_empty() && branch.leaf_message_id.is_some() {
                    let message_ids = self
                        .get_path_to_message(branch.leaf_message_id)
                        .into_iter()
                        .map(|m| m.id)
                        .collect();
                    Some((idx, message_ids))
                } else {
                    None
                }
            })
            .collect();

        let migrated = !migrations.is_empty();
        if migrated {
            let branches = Arc::make_mut(&mut self.branches);
            for (idx, message_ids) in migrations {
                branches[idx].message_ids = message_ids;
            }
        }
        migrated
    }

    pub fn rebuild_branches_from_history(&mut self) -> bool {
        if !self.branches.is_empty() {
            return false;
        }

        if self.conversation_history.is_empty() {
            self.reset_branches_to_root();
            return true;
        }

        let leaves = self.find_leaf_message_ids();
        if leaves.is_empty() {
            self.reset_branches_to_root();
            return true;
        }

        let branches = self.build_branches_from_leaves(leaves);
        self.apply_rebuilt_branches(branches);
        true
    }

    fn reset_branches_to_root(&mut self) {
        let dialect = self.selected_dialect;
        let branch = ConversationBranch::new(None, None, None, dialect, vec![], None);
        self.active_branch_id = branch.id;
        self.branches = Arc::new(vec![branch]);
    }

    fn find_leaf_message_ids(&self) -> Vec<Uuid> {
        let mut parents = HashSet::new();
        for msg in self.conversation_history.iter() {
            if let Some(parent) = msg.parent_id {
                parents.insert(parent);
            }
        }

        self.conversation_history
            .iter()
            .filter(|msg| !parents.contains(&msg.id))
            .map(|msg| msg.id)
            .collect()
    }

    fn build_branches_from_leaves(&self, leaves: Vec<Uuid>) -> Vec<ConversationBranch> {
        let mut branches = Vec::with_capacity(leaves.len());
        for leaf_id in leaves {
            let parent_id = self.find_message(leaf_id).and_then(|msg| msg.parent_id);
            let dialect = self.branch_dialect(leaf_id);
            let message_ids = self
                .get_path_to_message(Some(leaf_id))
                .into_iter()
                .map(|m| m.id)
                .collect();
            branches.push(ConversationBranch::new(
                parent_id,
                None,
                Some(leaf_id),
                dialect,
                message_ids,
                None,
            ));
        }
        branches
    }

    fn branch_dialect(&self, leaf_id: Uuid) -> Dialect {
        let path = self.get_path_to_message(Some(leaf_id));
        path.first()
            .map(|msg| msg.metadata.dialect)
            .unwrap_or(self.selected_dialect)
    }

    fn apply_rebuilt_branches(&mut self, branches: Vec<ConversationBranch>) {
        if branches.is_empty() {
            self.reset_branches_to_root();
            return;
        }

        let active_branch_id = self.select_active_branch(&branches);
        self.active_branch_id = active_branch_id;
        self.branches = Arc::new(branches);
    }

    fn select_active_branch(&self, branches: &[ConversationBranch]) -> Uuid {
        self.branch_with_latest_leaf(branches)
            .unwrap_or_else(|| branches[0].id)
    }

    fn branch_with_latest_leaf(&self, branches: &[ConversationBranch]) -> Option<Uuid> {
        branches
            .iter()
            .filter_map(|branch| {
                branch.leaf_message_id.and_then(|leaf_id| {
                    self.find_message(leaf_id)
                        .map(|msg| (branch.id, msg.metadata.timestamp))
                })
            })
            .max_by_key(|(_, timestamp)| *timestamp)
            .map(|(branch_id, _)| branch_id)
    }

    fn find_message(&self, id: Uuid) -> Option<&Message> {
        self.conversation_history.iter().find(|msg| msg.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConversationBranch, Dialect, Formality, InitialUserSettings, Language, LearningItem,
        Message, MessageMetadata, TeachingMode, UserGender, UserState,
    };
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::models::agent::Mistake;
    use crate::models::agent::MistakeCategory;
    use crate::models::learning_item::LearningItemType;

    fn test_metadata(session_id: Uuid) -> MessageMetadata {
        MessageMetadata::at_now(
            Formality::Informal,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishArgentinian,
            session_id,
        )
    }

    fn create_test_user_state() -> UserState {
        UserState::new(Uuid::new_v4())
    }

    #[test]
    fn test_user_state_with_initial_settings() {
        use super::{JlptLevel, LanguageLevel};
        let settings = InitialUserSettings {
            language: Language::Japanese,
            dialect: Dialect::JapaneseTokyo,
            level: LanguageLevel::Jlpt(JlptLevel::N4),
            gender: UserGender::Female,
        };
        let state = UserState::with_initial_settings(Uuid::new_v4(), Some(settings));
        assert_eq!(state.selected_language, Language::Japanese);
        assert_eq!(state.selected_dialect, Dialect::JapaneseTokyo);
        assert_eq!(state.user_gender, UserGender::Female);
        assert_eq!(state.dialect_levels.len(), 1);
        assert_eq!(
            state.dialect_levels[0].level,
            LanguageLevel::Jlpt(JlptLevel::N4)
        );
    }

    fn create_test_mistake() -> Mistake {
        Mistake::new(
            "hablar".to_string(),
            "habla".to_string(),
            MistakeCategory::SpellingError {
                context: "context".to_string(),
            },
        )
    }

    fn create_test_message() -> Message {
        Message::user_message("test".to_string(), test_metadata(Uuid::new_v4()), None)
    }

    #[test]
    fn test_user_state_serialization() {
        let state = create_test_user_state();
        let json = serde_json::to_string(&state).unwrap();

        assert!(json.contains("\"user_id\""));
        assert!(json.contains("\"learning_items\""));
        assert!(json.contains("\"conversation_history\""));
        assert!(json.contains("\"tts_enabled\""));
    }

    #[test]
    fn test_user_state_deserialization() {
        let state = create_test_user_state();
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: UserState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.user_id, deserialized.user_id);
        assert_eq!(state.learning_items, deserialized.learning_items);
        assert_eq!(state.tts_enabled, deserialized.tts_enabled);
    }

    #[test]
    fn test_conversation_history_in_user_state() {
        let mut state = create_test_user_state();
        let msg = create_test_message();
        Arc::make_mut(&mut state.conversation_history).push(msg.clone());

        assert_eq!(state.conversation_history.len(), 1);
        assert_eq!(state.conversation_history[0].id, msg.id);
    }

    #[test]
    fn test_learning_item_new() {
        let mistake = create_test_mistake();
        let item = LearningItem::new(
            LearningItemType::Mistake(mistake.clone()),
            Dialect::SpanishMexican,
        );

        assert_eq!(item.score, 0);
        assert_eq!(item.dialect, Dialect::SpanishMexican);
        match item.item {
            LearningItemType::Mistake(m) => assert_eq!(m.id, mistake.id),
            _ => panic!("Expected Mistake variant"),
        }
    }

    #[test]
    fn test_user_state_new() {
        let user_id = Uuid::new_v4();
        let state = UserState::new(user_id);

        assert_eq!(state.user_id, user_id);
        assert_eq!(state.learning_items.len(), 0);
        assert_eq!(state.conversation_history.len(), 0);
        assert!(!state.tts_enabled);
    }

    #[test]
    fn test_learning_item_serialization() {
        let mistake = create_test_mistake();
        let item = LearningItem::new(LearningItemType::Mistake(mistake), Dialect::SpanishMexican);
        let json = serde_json::to_string(&item).unwrap();

        assert!(json.contains("\"score\""));
        assert!(json.contains("\"item\""));
        assert!(json.contains("\"dialect\""));
    }

    #[test]
    fn test_get_active_branch_messages_empty() {
        let state = create_test_user_state();
        let messages = state.get_active_branch_messages();
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_get_active_branch_messages_single() {
        let mut state = create_test_user_state();
        let msg = Message::user_message("test".to_string(), test_metadata(Uuid::new_v4()), None);
        Arc::make_mut(&mut state.conversation_history).push(msg.clone());

        // Set the branch's leaf to this message
        if let Some(branch) = Arc::make_mut(&mut state.branches)
            .iter_mut()
            .find(|b| b.id == state.active_branch_id)
        {
            branch.message_ids = vec![msg.id];
            branch.leaf_message_id = Some(msg.id);
        }

        let messages = state.get_active_branch_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, msg.id);
    }

    #[test]
    fn test_get_active_branch_messages_chain() {
        let mut state = create_test_user_state();

        // Create a chain: A → B → C
        let msg_a = Message::user_message("A".to_string(), test_metadata(Uuid::new_v4()), None);
        Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());

        let msg_b = Message::user_message(
            "B".to_string(),
            test_metadata(Uuid::new_v4()),
            Some(msg_a.id),
        );
        Arc::make_mut(&mut state.conversation_history).push(msg_b.clone());

        let msg_c = Message::user_message(
            "C".to_string(),
            test_metadata(Uuid::new_v4()),
            Some(msg_b.id),
        );
        Arc::make_mut(&mut state.conversation_history).push(msg_c.clone());

        // Set the branch's leaf to msg_c
        if let Some(branch) = Arc::make_mut(&mut state.branches)
            .iter_mut()
            .find(|b| b.id == state.active_branch_id)
        {
            branch.message_ids = vec![msg_a.id, msg_b.id, msg_c.id];
            branch.leaf_message_id = Some(msg_c.id);
        }

        let messages = state.get_active_branch_messages();
        assert_eq!(messages.len(), 3);
        // Messages should be in order: A, B, C (root to leaf)
        assert_eq!(messages[0].id, msg_a.id);
        assert_eq!(messages[1].id, msg_b.id);
        assert_eq!(messages[2].id, msg_c.id);
        assert_eq!(messages[0].get_content(), "A");
        assert_eq!(messages[1].get_content(), "B");
        assert_eq!(messages[2].get_content(), "C");
    }

    #[test]
    fn test_get_active_branch_messages_excludes_other_branch() {
        let mut state = create_test_user_state();

        // Create main path: A → B
        let msg_a = Message::user_message("A".to_string(), test_metadata(Uuid::new_v4()), None);
        Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());

        let msg_b = Message::user_message(
            "B".to_string(),
            test_metadata(Uuid::new_v4()),
            Some(msg_a.id),
        );
        Arc::make_mut(&mut state.conversation_history).push(msg_b.clone());

        // Create alternative path from A: A → X
        let msg_x = Message::user_message(
            "X".to_string(),
            test_metadata(Uuid::new_v4()),
            Some(msg_a.id),
        );
        Arc::make_mut(&mut state.conversation_history).push(msg_x.clone());

        // Set active branch leaf to B (so path is A → B, not A → X)
        if let Some(branch) = Arc::make_mut(&mut state.branches)
            .iter_mut()
            .find(|b| b.id == state.active_branch_id)
        {
            branch.message_ids = vec![msg_a.id, msg_b.id];
            branch.leaf_message_id = Some(msg_b.id);
        }

        let messages = state.get_active_branch_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, msg_a.id);
        assert_eq!(messages[1].id, msg_b.id);
        // msg_x should NOT be in the result
        assert!(!messages.iter().any(|m| m.id == msg_x.id));
    }

    #[test]
    fn test_migrate_branch_message_ids() {
        let mut state = create_test_user_state();

        // Create a conversation chain A → B → C
        let msg_a = Message::user_message("A".to_string(), test_metadata(Uuid::new_v4()), None);
        let msg_b = Message::user_message(
            "B".to_string(),
            test_metadata(Uuid::new_v4()),
            Some(msg_a.id),
        );
        let msg_c = Message::user_message(
            "C".to_string(),
            test_metadata(Uuid::new_v4()),
            Some(msg_b.id),
        );

        Arc::make_mut(&mut state.conversation_history).push(msg_a.clone());
        Arc::make_mut(&mut state.conversation_history).push(msg_b.clone());
        Arc::make_mut(&mut state.conversation_history).push(msg_c.clone());

        // Simulate old data: branch has leaf_message_id but empty message_ids
        let branch = &mut Arc::make_mut(&mut state.branches)[0];
        branch.leaf_message_id = Some(msg_c.id);
        branch.message_ids = vec![]; // Simulate old data format

        // Run migration
        let migrated = state.migrate_branch_message_ids();
        assert!(migrated, "Migration should have occurred");

        // Verify message_ids was populated
        assert_eq!(
            state.branches[0].message_ids,
            vec![msg_a.id, msg_b.id, msg_c.id]
        );

        // Running migration again should return false (idempotent)
        let migrated_again = state.migrate_branch_message_ids();
        assert!(!migrated_again, "Migration should be idempotent");
    }

    #[test]
    fn test_get_child_branches_none() {
        let state = create_test_user_state();
        let message_id = Uuid::new_v4();
        let branches = state.get_child_branches(message_id);
        assert_eq!(branches.len(), 0);
    }

    #[test]
    fn test_get_child_branches_one() {
        let mut state = create_test_user_state();
        let message_id = Uuid::new_v4();
        let branch = ConversationBranch::new(
            Some(message_id),
            None,
            None,
            Dialect::SpanishMexican,
            vec![],
            None,
        );
        let branch_id = branch.id;
        Arc::make_mut(&mut state.branches).push(branch);

        let child_branches = state.get_child_branches(message_id);
        assert_eq!(child_branches.len(), 1);
        assert_eq!(child_branches[0].id, branch_id);
    }

    #[test]
    fn test_find_branch_root_exists() {
        let mut state = create_test_user_state();
        let parent_msg_id = Uuid::new_v4();
        let branch = ConversationBranch::new(
            Some(parent_msg_id),
            None,
            None,
            Dialect::SpanishMexican,
            vec![],
            None,
        );
        let branch_id = branch.id;
        Arc::make_mut(&mut state.branches).push(branch);

        let root = state.find_branch_root(branch_id);
        assert_eq!(root, Some(parent_msg_id));
    }

    #[test]
    fn test_find_branch_root_not_found() {
        let state = create_test_user_state();
        let fake_branch_id = Uuid::new_v4();
        let root = state.find_branch_root(fake_branch_id);
        assert_eq!(root, None);
    }

    #[test]
    fn test_rebuild_branches_from_history_creates_branch() {
        let mut state = create_test_user_state();
        let msg = create_test_message();
        Arc::make_mut(&mut state.conversation_history).push(msg.clone());
        Arc::make_mut(&mut state.branches).clear();
        state.active_branch_id = Uuid::new_v4();

        let rebuilt = state.rebuild_branches_from_history();

        assert!(rebuilt);
        assert_eq!(state.branches.len(), 1);
        let branch = &state.branches[0];
        assert_eq!(branch.leaf_message_id, Some(msg.id));
        assert_eq!(state.active_branch_id, branch.id);
    }

    #[test]
    fn test_get_active_branch_messages_fallbacks_to_existing_branch() {
        let mut state = create_test_user_state();
        let msg = create_test_message();
        Arc::make_mut(&mut state.conversation_history).push(msg.clone());

        if let Some(branch) = Arc::make_mut(&mut state.branches).first_mut() {
            branch.message_ids = vec![msg.id];
            branch.leaf_message_id = Some(msg.id);
        }
        state.active_branch_id = Uuid::new_v4();

        let messages = state.get_active_branch_messages();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, msg.id);
    }

    #[test]
    fn test_user_gender_serialization() {
        let genders = vec![UserGender::Male, UserGender::Female, UserGender::NonBinary];
        for gender in genders {
            let json = serde_json::to_string(&gender).unwrap();
            let deserialized: UserGender = serde_json::from_str(&json).unwrap();
            assert_eq!(gender, deserialized);
        }
    }

    #[test]
    fn test_current_language_option_with_arabic() {
        use crate::models::language_options::{ArabicScript, LanguageOption};

        let mut state = create_test_user_state();
        state.selected_language = Language::Arabic;
        state.language_options.arabic_script = ArabicScript::Ruqa;

        let option = state.current_language_option();
        assert_eq!(option, Some(LanguageOption::Arabic(ArabicScript::Ruqa)));
    }

    #[test]
    fn test_current_language_option_with_japanese() {
        use crate::models::language_options::{JapaneseScript, LanguageOption};

        let mut state = create_test_user_state();
        state.selected_language = Language::Japanese;
        state.language_options.japanese_script = JapaneseScript::Romaji;

        let option = state.current_language_option();
        assert_eq!(
            option,
            Some(LanguageOption::Japanese(JapaneseScript::Romaji))
        );
    }

    #[test]
    fn test_current_language_option_with_spanish() {
        let state = create_test_user_state();
        let option = state.current_language_option();
        assert_eq!(option, None);
    }

    #[test]
    fn test_language_level_default_is_cefr_b1() {
        use super::{CefrLevel, LanguageLevel};
        assert_eq!(LanguageLevel::default(), LanguageLevel::Cefr(CefrLevel::B1));
    }

    #[test]
    fn test_get_level_for_dialect_returns_default_when_not_set() {
        use super::{CefrLevel, LanguageLevel};
        let state = create_test_user_state();
        let level = state.get_level_for_dialect(&Dialect::SpanishMexican);
        assert_eq!(level, LanguageLevel::Cefr(CefrLevel::B1));
    }

    #[test]
    fn test_set_and_get_level_for_dialect() {
        use super::{CefrLevel, LanguageLevel};
        let mut state = create_test_user_state();
        state.set_level_for_dialect(Dialect::SpanishMexican, LanguageLevel::Cefr(CefrLevel::C1));
        assert_eq!(
            state.get_level_for_dialect(&Dialect::SpanishMexican),
            LanguageLevel::Cefr(CefrLevel::C1)
        );
    }

    #[test]
    fn test_set_level_updates_existing() {
        use super::{CefrLevel, LanguageLevel};
        let mut state = create_test_user_state();
        state.set_level_for_dialect(Dialect::SpanishMexican, LanguageLevel::Cefr(CefrLevel::A1));
        state.set_level_for_dialect(Dialect::SpanishMexican, LanguageLevel::Cefr(CefrLevel::C2));
        assert_eq!(
            state.get_level_for_dialect(&Dialect::SpanishMexican),
            LanguageLevel::Cefr(CefrLevel::C2)
        );
        assert_eq!(state.dialect_levels.len(), 1);
    }

    #[test]
    fn test_language_level_serialization() {
        use super::{CefrLevel, JlptLevel, LanguageLevel};
        let cefr_levels = vec![
            LanguageLevel::Cefr(CefrLevel::A1),
            LanguageLevel::Cefr(CefrLevel::A2),
            LanguageLevel::Cefr(CefrLevel::B1),
            LanguageLevel::Cefr(CefrLevel::B2),
            LanguageLevel::Cefr(CefrLevel::C1),
            LanguageLevel::Cefr(CefrLevel::C2),
        ];
        for level in cefr_levels {
            let json = serde_json::to_string(&level).unwrap();
            let deserialized: LanguageLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, deserialized);
        }

        let jlpt_levels = vec![
            LanguageLevel::Jlpt(JlptLevel::N5),
            LanguageLevel::Jlpt(JlptLevel::N4),
            LanguageLevel::Jlpt(JlptLevel::N3),
            LanguageLevel::Jlpt(JlptLevel::N2),
            LanguageLevel::Jlpt(JlptLevel::N1),
        ];
        for level in jlpt_levels {
            let json = serde_json::to_string(&level).unwrap();
            let deserialized: LanguageLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, deserialized);
        }
    }

    #[test]
    fn test_cefr_to_jlpt_conversion() {
        use super::{CefrLevel, JlptLevel};
        assert_eq!(CefrLevel::A1.to_jlpt(), JlptLevel::N5);
        assert_eq!(CefrLevel::A2.to_jlpt(), JlptLevel::N4);
        assert_eq!(CefrLevel::B1.to_jlpt(), JlptLevel::N3);
        assert_eq!(CefrLevel::B2.to_jlpt(), JlptLevel::N2);
        assert_eq!(CefrLevel::C1.to_jlpt(), JlptLevel::N1);
        assert_eq!(CefrLevel::C2.to_jlpt(), JlptLevel::N1);
    }

    #[test]
    fn test_jlpt_to_cefr_conversion() {
        use super::{CefrLevel, JlptLevel};
        assert_eq!(JlptLevel::N5.to_cefr(), CefrLevel::A1);
        assert_eq!(JlptLevel::N4.to_cefr(), CefrLevel::A2);
        assert_eq!(JlptLevel::N3.to_cefr(), CefrLevel::B1);
        assert_eq!(JlptLevel::N2.to_cefr(), CefrLevel::B2);
        assert_eq!(JlptLevel::N1.to_cefr(), CefrLevel::C1);
    }

    #[test]
    fn test_language_level_for_language() {
        use super::{CefrLevel, JlptLevel, LanguageLevel};
        let spanish_levels = LanguageLevel::for_language(Language::Spanish);
        assert_eq!(spanish_levels.len(), 6);
        assert_eq!(spanish_levels[0], LanguageLevel::Cefr(CefrLevel::A1));

        let japanese_levels = LanguageLevel::for_language(Language::Japanese);
        assert_eq!(japanese_levels.len(), 5);
        assert_eq!(japanese_levels[0], LanguageLevel::Jlpt(JlptLevel::N5));
    }

    #[test]
    fn test_language_level_default_for_language() {
        use super::{CefrLevel, JlptLevel, LanguageLevel};
        assert_eq!(
            LanguageLevel::default_for_language(Language::Spanish),
            LanguageLevel::Cefr(CefrLevel::B1)
        );
        assert_eq!(
            LanguageLevel::default_for_language(Language::Japanese),
            LanguageLevel::Jlpt(JlptLevel::N3)
        );
    }

    #[test]
    fn test_language_level_from_id() {
        use super::{CefrLevel, JlptLevel, LanguageLevel};
        assert_eq!(
            LanguageLevel::from_id("a1"),
            Some(LanguageLevel::Cefr(CefrLevel::A1))
        );
        assert_eq!(
            LanguageLevel::from_id("n3"),
            Some(LanguageLevel::Jlpt(JlptLevel::N3))
        );
        assert_eq!(LanguageLevel::from_id("invalid"), None);
    }

    #[test]
    fn test_convert_cefr_to_jlpt_for_japanese() {
        use super::{CefrLevel, JlptLevel, LanguageLevel};
        let cefr_b2 = LanguageLevel::Cefr(CefrLevel::B2);
        let converted = cefr_b2.convert_for_language(Language::Japanese);
        assert_eq!(converted, LanguageLevel::Jlpt(JlptLevel::N2));
    }

    #[test]
    fn test_convert_jlpt_to_cefr_for_spanish() {
        use super::{CefrLevel, JlptLevel, LanguageLevel};
        let jlpt_n3 = LanguageLevel::Jlpt(JlptLevel::N3);
        let converted = jlpt_n3.convert_for_language(Language::Spanish);
        assert_eq!(converted, LanguageLevel::Cefr(CefrLevel::B1));
    }

    #[test]
    fn test_convert_cefr_stays_cefr_for_non_japanese() {
        use super::{CefrLevel, LanguageLevel};
        let cefr_a1 = LanguageLevel::Cefr(CefrLevel::A1);
        let converted = cefr_a1.convert_for_language(Language::Spanish);
        assert_eq!(converted, LanguageLevel::Cefr(CefrLevel::A1));
    }

    #[test]
    fn test_convert_jlpt_stays_jlpt_for_japanese() {
        use super::{JlptLevel, LanguageLevel};
        let jlpt_n5 = LanguageLevel::Jlpt(JlptLevel::N5);
        let converted = jlpt_n5.convert_for_language(Language::Japanese);
        assert_eq!(converted, LanguageLevel::Jlpt(JlptLevel::N5));
    }
}

#[test]
fn test_default_dialect_leakage_reproduction() {
    // user reports show_experimental was TRUE
    let show_experimental = true;

    // When switching to Japanese
    let default_dialect =
        UserState::default_dialect_for_language(Language::Japanese, show_experimental)
            .expect("Should find a dialect for Japanese");

    // It SHOULD be a Japanese dialect
    assert_eq!(
        default_dialect.language(),
        Language::Japanese,
        "User reported leakage where Japanese selected SpanishArgentinian"
    );

    // Specifically, it should ideally be Tokyo if available
    assert_eq!(default_dialect, Dialect::JapaneseTokyo);
}

#[test]
fn test_default_dialect_fallback_safety() {
    // Even if show_experimental is FALSE
    let show_experimental = false;

    // And we ask for Japanese (which only has experimental dialects currently)
    let default_dialect =
        UserState::default_dialect_for_language(Language::Japanese, show_experimental)
            .expect("Should find a fallback dialect even if experimental is hidden");

    // It MUST still be Japanese to avoid leakage
    assert_eq!(
        default_dialect.language(),
        Language::Japanese,
        "Fallback logic leaked to different language!"
    );
}
