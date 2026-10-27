use dialect_coach_shared::models::{
    ArabicScript, Dialect, Formality, JapaneseScript, Language, LanguageLevel, LanguagePlan,
    Message, TeachingMode, UserGender,
};
use dialect_coach_shared::{
    AgentAnalysis, Explained, Exploratory, LearningItem, Mistake, Translated, UsageStats, UserState,
};
use uuid::Uuid;

#[derive(Clone, PartialEq, Debug)]
pub struct DeletedMessage {
    pub message: Message,
    pub history_index: usize,
    pub branches: Vec<DeletedMessageLocation>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DeletedMessageLocation {
    pub branch_id: Uuid,
    pub message_index: usize,
    pub leaf_message_id: Option<Uuid>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum MessageAction {
    Reply {
        message: Message,
        origin: ReplyOrigin,
    },
    Add(Message),
    Delete(Uuid),
    UndoDelete(DeletedMessage),
}

#[derive(Clone, PartialEq, Debug)]
pub struct ReplyOrigin {
    pub branch_id: Uuid,
    pub dialect: Dialect,
    pub plan_id: Option<Uuid>,
    pub activity: Option<Box<dialect_coach_shared::ActivityContext>>,
    pub checked_head: Option<Uuid>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum GoalTarget {
    Dialect,
    Activity { branch_id: Uuid, activity_id: Uuid },
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
    AddGoal(GoalTarget, String),
    DeleteGoal(GoalTarget, usize),
    UpdateGoal(GoalTarget, usize, String),
}

#[derive(Clone, PartialEq, Debug)]
pub enum BranchAction {
    ActivityGenerated {
        branch_id: Uuid,
        expected_head: Option<Uuid>,
        definition: dialect_coach_shared::ActivityDefinition,
    },
    ActivityChecked {
        origin: ReplyOrigin,
        analysis: AgentAnalysis,
    },
    EndActivity {
        branch_id: Uuid,
        activity_id: Uuid,
    },
    Create(Uuid),
    CreateNew,
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
    UpdateLevel(LanguageLevel),
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
}

impl From<UserDomainAction> for UserStateAction {
    fn from(action: UserDomainAction) -> Self {
        match action {
            UserDomainAction::Message(a) => UserStateAction::Message(a),
            UserDomainAction::Learning(a) => UserStateAction::Learning(a),
            UserDomainAction::Branch(a) => UserStateAction::Branch(a),
            UserDomainAction::Plan(a) => UserStateAction::Plan(a),
            UserDomainAction::Settings(a) => UserStateAction::Settings(a),
        }
    }
}
