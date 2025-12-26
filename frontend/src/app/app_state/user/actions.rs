use dialect_coach_shared::models::{
    ArabicScript, Dialect, Formality, JapaneseScript, Language, LanguageLevel, LanguagePlan,
    Message, TeachingMode, UserGender,
};
use dialect_coach_shared::{
    AgentAnalysis, Explained, Exploratory, LearningGoal, LearningItem, Mistake, Translated,
    UsageStats, UserState,
};
use uuid::Uuid;

#[derive(Clone, PartialEq, Debug)]
pub enum MessageAction {
    Add(Message),
    Delete(Uuid),
    UndoDelete(Message),
}

#[derive(Clone, PartialEq, Debug)]
pub enum LearningAction {
    AddItems(
        Vec<(Mistake, u8)>,
        Vec<(Explained, u8)>,
        Vec<(Translated, u8)>,
        Vec<(Exploratory, u8)>,
    ),
    UpdateScores(AgentAnalysis),
    DeleteItem(Uuid),
    UndoDeleteItem(LearningItem),
    AddGoal(LearningGoal),
    DeleteGoal(usize),
}

#[derive(Clone, PartialEq, Debug)]
pub enum BranchAction {
    Create(Uuid),
    Switch(Uuid),
    Delete(Uuid),
    Rename(Uuid, String),
}

#[derive(Clone, PartialEq, Debug)]
pub enum PlanAction {
    Add(LanguagePlan),
    Delete(Uuid),
    SetActive(Option<Uuid>),
    AdvanceStep(Uuid),
    ActivateStep(Uuid),
    Update(LanguagePlan),
}

#[derive(Clone, PartialEq, Debug)]
pub enum SettingsAction {
    ChangeDialect(Dialect),
    ChangeLanguage(Language),
    ChangeFormality(Formality),
    ChangeTeachingMode(TeachingMode),
    UpdateGender(UserGender),
    UpdateLevel(Dialect, LanguageLevel),
    SetArabicScript(ArabicScript),
    SetJapaneseScript(JapaneseScript),
    ToggleTTS,
    ToggleExperimentalDialects,
    CycleDialect,
    CycleFormality,
    CycleTeachingMode(bool),
}

#[derive(Clone, PartialEq, Debug)]
pub enum UserStateAction {
    // Domains
    Message(MessageAction),
    Learning(LearningAction),
    Branch(BranchAction),
    Plan(PlanAction),
    Settings(SettingsAction),

    // Lifecycle / Meta
    ReplaceUserState(UserState),
    UpdateUsageStats(UsageStats),
    ClearUserState,
}

#[derive(Clone, PartialEq, Debug)]
pub enum UserDomainAction {
    Message(MessageAction),
    Learning(LearningAction),
    Branch(BranchAction),
    Plan(PlanAction),
    Settings(SettingsAction),
    UsageStats(UsageStats),
}

impl From<UserDomainAction> for UserStateAction {
    fn from(action: UserDomainAction) -> Self {
        match action {
            UserDomainAction::Message(a) => UserStateAction::Message(a),
            UserDomainAction::Learning(a) => UserStateAction::Learning(a),
            UserDomainAction::Branch(a) => UserStateAction::Branch(a),
            UserDomainAction::Plan(a) => UserStateAction::Plan(a),
            UserDomainAction::Settings(a) => UserStateAction::Settings(a),
            UserDomainAction::UsageStats(a) => UserStateAction::UpdateUsageStats(a),
        }
    }
}
