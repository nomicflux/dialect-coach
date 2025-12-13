use super::super::{
    dialect::Dialect,
    learning_item::{LearningItem, LearningItemType},
    plan::{LanguagePlan, PlanContent, PlanStatus, PlanStep, StepStatus, StepType},
    *,
};
use crate::models::{
    Explained, Exploratory, Mistake, Translated, partial_learning_item::PartialLearningItem,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

// --- Import DTOs (match YAML) ---

#[derive(Debug, Clone, Deserialize)]
pub struct ImportLanguagePlan {
    pub title: String,
    pub dialect: Dialect,
    pub description: Option<String>,
    pub steps: Vec<ImportPlanStep>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportPlanStep {
    #[serde(default)]
    pub title: String,
    pub step_type: ImportStepType,
    #[serde(default)]
    pub instructions: String,
}

#[derive(Debug, Clone, Deserialize)]

pub enum ImportStepType {
    Learning {
        content: ImportPlanContent,
    },
    Review {
        #[serde(rename = "review_steps")]
        review_steps: Vec<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportPlanContent {
    pub items: Vec<ImportLearningItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportLearningItem {
    #[serde(rename = "item")]
    pub item: ImportPartialLearningItem,
    // Optional overrides
    pub score: Option<u8>,
    pub dialect: Option<Dialect>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ImportPartialLearningItem {
    Mistake(crate::models::PartialMistake),
    Explanation(crate::models::PartialExplained),
    Translation(crate::models::PartialTranslated),
    Exploration(crate::models::PartialExploratory),
}

impl From<ImportPartialLearningItem> for PartialLearningItem {
    fn from(item: ImportPartialLearningItem) -> Self {
        match item {
            ImportPartialLearningItem::Mistake(m) => PartialLearningItem::Mistake(m),
            ImportPartialLearningItem::Explanation(e) => PartialLearningItem::Explained(e),
            ImportPartialLearningItem::Translation(t) => PartialLearningItem::Translated(t),
            ImportPartialLearningItem::Exploration(e) => PartialLearningItem::Exploratory(e),
        }
    }
}

// --- Intermediate "Hydrated" Structures ---
// These match the structure of LanguagePlan but allow PartialLearningItems

#[derive(Debug, Clone)]
pub struct HydratedLanguagePlan {
    pub id: Uuid,
    pub title: String,
    pub dialect: Dialect,
    pub description: Option<String>,
    pub steps: Vec<HydratedPlanStep>,
    pub created_at: i64,
    pub current_step_index: usize,
    pub status: PlanStatus,
}

#[derive(Debug, Clone)]
pub struct HydratedPlanStep {
    pub id: Uuid,
    pub step_number: usize,
    pub title: String,
    pub step_type: HydratedStepType,
    pub instructions: String,
    pub status: StepStatus,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum HydratedStepType {
    Learning { content: HydratedPlanContent },
    Review { review_step_ids: Vec<Uuid> },
}

#[derive(Debug, Clone)]
pub struct HydratedPlanContent {
    pub items: Vec<HydratedLearningItem>,
}

#[derive(Debug, Clone)]
pub struct HydratedLearningItem {
    pub item: PartialLearningItem,
    pub score: u8,
    pub dialect: Dialect,
}

impl ImportLanguagePlan {
    /// Converts the raw import format into a "Hydrated" plan.
    /// - Generates UUIDs for the Plan and all Steps.
    /// - Resolves Review step title references to their new UUIDs.
    /// - Sets default values (Status::NotStarted, Score 0, timestamps).
    /// - Propagates the top-level dialect to items that don't specify one.
    pub fn hydrate(self) -> Result<HydratedLanguagePlan, String> {
        // 1. Validate uniquness of step titles (required for reliable linking)
        let mut title_to_id = HashMap::new();
        // Generate IDs first
        let step_ids: Vec<Uuid> = (0..self.steps.len()).map(|_| Uuid::new_v4()).collect();

        for (i, step) in self.steps.iter().enumerate() {
            if !step.title.is_empty() {
                if title_to_id.contains_key(&step.title) {
                    return Err(format!(
                        "Duplicate step title found: '{}'. Step titles must be unique for review referencing.",
                        step.title
                    ));
                }
                title_to_id.insert(step.title.clone(), step_ids[i]);
            }
        }

        // 2. Build steps
        let mut hydrated_steps = Vec::new();
        for (i, step) in self.steps.into_iter().enumerate() {
            let id = step_ids[i];
            let step_number = i + 1;

            let hydrated_type = match step.step_type {
                ImportStepType::Learning { content } => {
                    let mut items = Vec::new();
                    for item_wrapper in content.items {
                        items.push(HydratedLearningItem {
                            item: item_wrapper.item.into(),
                            score: item_wrapper.score.unwrap_or(0),
                            dialect: item_wrapper.dialect.unwrap_or(self.dialect),
                        });
                    }
                    HydratedStepType::Learning {
                        content: HydratedPlanContent { items },
                    }
                }
                ImportStepType::Review { review_steps } => {
                    let mut ids = Vec::new();
                    for title in review_steps {
                        match title_to_id.get(&title) {
                            Some(target_id) => ids.push(*target_id),
                            None => {
                                return Err(format!(
                                    "Review step references unknown title: '{}'",
                                    title
                                ));
                            }
                        }
                    }
                    HydratedStepType::Review {
                        review_step_ids: ids,
                    }
                }
            };

            hydrated_steps.push(HydratedPlanStep {
                id,
                step_number,
                title: step.title,
                step_type: hydrated_type,
                instructions: step.instructions,
                status: StepStatus::NotStarted,
                started_at: None,
                completed_at: None,
            });
        }

        Ok(HydratedLanguagePlan {
            id: Uuid::new_v4(),
            title: self.title,
            dialect: self.dialect,
            description: self.description,
            steps: hydrated_steps,
            created_at: chrono::Utc::now().timestamp(),
            current_step_index: 0,
            status: PlanStatus::NotStarted,
        })
    }
}

// Support converting Hydrated -> Full if all items are fully populated
// (This might happen if the import file provided everything)
impl HydratedLanguagePlan {
    pub fn try_into_plan(self) -> Result<LanguagePlan, String> {
        let mut steps = Vec::new();

        for step in self.steps {
            let step_type = match step.step_type {
                HydratedStepType::Review { review_step_ids } => {
                    StepType::Review { review_step_ids }
                }
                HydratedStepType::Learning { content } => {
                    let mut items = Vec::new();
                    for h_item in content.items {
                        // Check if PartialLearningItem is complete
                        let full_item = match h_item.item {
                            PartialLearningItem::Mistake(p) => {
                                if let (Some(sm), Some(c)) = (p.specific_mistake, p.correction) {
                                    let category = match p.mistake_category.as_deref() {
                                        Some("SpellingError") => MistakeCategory::SpellingError {
                                            context: "Imported".into(),
                                        },
                                        Some("VocabularyError") => {
                                            MistakeCategory::VocabularyError {
                                                context: "Imported".into(),
                                            }
                                        }
                                        Some("GrammarError") => MistakeCategory::GrammarError {
                                            context: "Imported".into(),
                                        },
                                        Some("DialectUsageError") => {
                                            MistakeCategory::DialectUsageError {
                                                context: "Imported".into(),
                                            }
                                        }
                                        Some(other) => MistakeCategory::Other {
                                            context: other.into(),
                                        },
                                        None => MistakeCategory::Other {
                                            context: "Imported".into(),
                                        },
                                    };
                                    LearningItemType::Mistake(Mistake::new(sm, c, category))
                                } else {
                                    return Err("Contains incomplete Mistake item".to_string());
                                }
                            }
                            PartialLearningItem::Explained(p) => {
                                if let (Some(np), Some(ex)) = (p.new_phrase, p.explanation) {
                                    LearningItemType::Explanation(Explained::new(np, ex))
                                } else {
                                    return Err("Contains incomplete Explained item".to_string());
                                }
                            }
                            PartialLearningItem::Translated(p) => {
                                if let (Some(w), Some(t)) = (p.translated_word, p.translated_to) {
                                    LearningItemType::Translation(Translated::new(w, t, p.context))
                                } else {
                                    return Err("Contains incomplete Translated item".to_string());
                                }
                            }
                            PartialLearningItem::Exploratory(p) => {
                                if let (Some(pt), Some(ins)) =
                                    (p.point_to_try, p.instructions_for_use)
                                {
                                    LearningItemType::Exploration(Exploratory::new(pt, ins))
                                } else {
                                    return Err("Contains incomplete Exploratory item".to_string());
                                }
                            }
                        };

                        items.push(LearningItem {
                            item: full_item,
                            score: h_item.score,
                            dialect: h_item.dialect,
                        });
                    }
                    StepType::Learning {
                        content: PlanContent { items },
                    }
                    // WAIT: StepType::Learning { content: PlanContent { items } }
                    // Need to check structure of PlanContent
                }
            };

            steps.push(PlanStep {
                id: step.id,
                step_number: step.step_number,
                title: step.title,
                step_type,
                instructions: step.instructions,
                status: step.status,
                started_at: step.started_at,
                completed_at: step.completed_at,
            });
        }

        Ok(LanguagePlan {
            id: self.id,
            title: self.title,
            dialect: self.dialect,
            description: self.description,
            steps,
            created_at: self.created_at,
            current_step_index: self.current_step_index,
            status: self.status,
        })
    }
}

// --- Simple Import DTOs (Simplified Human-Readable Format) ---

#[derive(Debug, Clone, Deserialize)]
pub struct SimpleImportLanguagePlan {
    pub title: String,
    pub dialect: Dialect,
    pub description: Option<String>,
    pub steps: Vec<SimpleImportStep>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SimpleImportStep {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub instructions: String,
    // User must provide exactly one of these (enforced in conversion)
    pub learning_content: Option<Vec<SimpleLearningItem>>,
    pub review_steps: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SimpleLearningItem {
    pub vocab: Option<String>,
    pub translation: Option<String>,
    pub grammar: Option<String>,
    pub explanation: Option<String>,
}

impl TryFrom<SimpleImportLanguagePlan> for ImportLanguagePlan {
    type Error = String;

    fn try_from(simple: SimpleImportLanguagePlan) -> Result<Self, Self::Error> {
        let mut steps = Vec::new();

        for (i, s) in simple.steps.into_iter().enumerate() {
            let step_type = match (s.learning_content, s.review_steps) {
                (Some(items), None) => {
                    let mut import_items = Vec::new();
                    for item in items {
                        let partial = match (item.vocab, item.grammar) {
                            (Some(v), None) => ImportPartialLearningItem::Translation(
                                crate::models::PartialTranslated {
                                    translated_word: item.translation,
                                    translated_to: Some(v),
                                    context: None,
                                },
                            ),
                            (None, Some(g)) => ImportPartialLearningItem::Explanation(
                                crate::models::PartialExplained {
                                    new_phrase: Some(g),
                                    explanation: item.explanation,
                                },
                            ),
                            (None, None) => {
                                return Err(format!(
                                    "Step {} item has neither 'vocab' nor 'grammar'",
                                    i + 1
                                ));
                            }
                            (Some(_), Some(_)) => {
                                return Err(format!(
                                    "Step {} item has both 'vocab' and 'grammar' (must be one)",
                                    i + 1
                                ));
                            }
                        };
                        import_items.push(ImportLearningItem {
                            item: partial,
                            score: None,
                            dialect: None, // Will inherit from plan during hydrate
                        });
                    }
                    ImportStepType::Learning {
                        content: ImportPlanContent {
                            items: import_items,
                        },
                    }
                }
                (None, Some(reviews)) => ImportStepType::Review {
                    review_steps: reviews,
                },
                (Some(_), Some(_)) => {
                    return Err(format!(
                        "Step {} has both 'learning_content' and 'review_steps'",
                        i + 1
                    ));
                }
                (None, None) => {
                    return Err(format!(
                        "Step {} has neither 'learning_content' nor 'review_steps'",
                        i + 1
                    ));
                }
            };

            steps.push(ImportPlanStep {
                title: s.title,
                step_type,
                instructions: s.instructions,
            });
        }

        Ok(ImportLanguagePlan {
            title: simple.title,
            dialect: simple.dialect,
            description: simple.description,
            steps,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{dialect::Dialect, partial_learning_item::PartialTranslated};

    #[test]
    fn test_hydrate_minimal() {
        let plan = ImportLanguagePlan {
            title: "My Plan".into(),
            dialect: Dialect::SpanishArgentinian,
            description: None,
            steps: vec![ImportPlanStep {
                title: "Step 1".into(),
                step_type: ImportStepType::Learning {
                    content: ImportPlanContent { items: vec![] },
                },
                instructions: "".into(),
            }],
        };

        let hydrated = plan.hydrate().expect("Hydrate failed");
        assert_eq!(hydrated.title, "My Plan");
        assert_eq!(hydrated.dialect, Dialect::SpanishArgentinian);
        assert_eq!(hydrated.steps.len(), 1);
    }

    #[test]
    fn test_hydrate_generates_ids_and_defaults() {
        let plan = ImportLanguagePlan {
            title: "Test".into(),
            dialect: Dialect::EnglishGeneralAmerican,
            description: None,
            steps: vec![ImportPlanStep {
                title: "First".into(),
                step_type: ImportStepType::Learning {
                    content: ImportPlanContent { items: vec![] },
                },
                instructions: "Do it".into(),
            }],
        };

        let hydrated = plan.hydrate().expect("Hydrate failed");
        assert_ne!(hydrated.id, Uuid::nil());
        assert_eq!(hydrated.steps.len(), 1);
        let s1 = &hydrated.steps[0];
        assert_ne!(s1.id, Uuid::nil());
        assert_eq!(s1.step_number, 1);
        assert_eq!(s1.status, StepStatus::NotStarted);
    }

    #[test]
    fn test_hydrate_resolves_review_steps() {
        let plan = ImportLanguagePlan {
            title: "Test".into(),
            dialect: Dialect::EnglishGeneralAmerican,
            description: None,
            steps: vec![
                ImportPlanStep {
                    title: "Learn A".into(),
                    step_type: ImportStepType::Learning {
                        content: ImportPlanContent { items: vec![] },
                    },
                    instructions: "".into(),
                },
                ImportPlanStep {
                    title: "Review Time".into(),
                    step_type: ImportStepType::Review {
                        review_steps: vec!["Learn A".into()],
                    },
                    instructions: "".into(),
                },
            ],
        };

        let hydrated = plan.hydrate().expect("Hydrate failed");
        let id_a = hydrated.steps[0].id;

        match &hydrated.steps[1].step_type {
            HydratedStepType::Review { review_step_ids } => {
                assert_eq!(review_step_ids.len(), 1);
                assert_eq!(review_step_ids[0], id_a);
            }
            _ => panic!("Wrong step type"),
        }
    }

    #[test]
    fn test_hydrate_fails_on_duplicate_title() {
        let plan = ImportLanguagePlan {
            title: "Test".into(),
            dialect: Dialect::EnglishGeneralAmerican,
            description: None,
            steps: vec![
                ImportPlanStep {
                    title: "Dup".into(),
                    step_type: ImportStepType::Learning {
                        content: ImportPlanContent { items: vec![] },
                    },
                    instructions: "".into(),
                },
                ImportPlanStep {
                    title: "Dup".into(),
                    step_type: ImportStepType::Learning {
                        content: ImportPlanContent { items: vec![] },
                    },
                    instructions: "".into(),
                },
            ],
        };
        assert!(plan.hydrate().is_err());
    }

    #[test]
    fn test_hydrate_propagates_dialect() {
        let plan = ImportLanguagePlan {
            title: "Test".into(),
            dialect: Dialect::FrenchParisian,
            description: None,
            steps: vec![ImportPlanStep {
                title: "S1".into(),
                step_type: ImportStepType::Learning {
                    content: ImportPlanContent {
                        items: vec![ImportLearningItem {
                            item: ImportPartialLearningItem::Translation(PartialTranslated {
                                translated_word: Some("W".into()),
                                translated_to: None,
                                context: None,
                            }),
                            score: None,
                            dialect: None, // Should inherit FrenchParisian
                        }],
                    },
                },
                instructions: "".into(),
            }],
        };

        let hydrated = plan.hydrate().expect("Hydrate failed");
        if let HydratedStepType::Learning { content } = &hydrated.steps[0].step_type {
            assert_eq!(content.items[0].dialect, Dialect::FrenchParisian);
        } else {
            panic!("Wrong step type");
        }
    }

    #[test]
    fn test_simple_import_conversion() {
        let simple = SimpleImportLanguagePlan {
            title: "Simple".into(),
            dialect: Dialect::SpanishChilean,
            description: None,
            steps: vec![
                SimpleImportStep {
                    title: "S1".into(),
                    instructions: "".into(),
                    learning_content: Some(vec![
                        SimpleLearningItem {
                            vocab: Some("hola".into()),
                            translation: Some("hello".into()),
                            grammar: None,
                            explanation: None,
                        },
                        SimpleLearningItem {
                            vocab: None,
                            translation: None,
                            grammar: Some("subjunctive".into()),
                            explanation: Some("express doubt".into()),
                        },
                    ]),
                    review_steps: None,
                },
                SimpleImportStep {
                    title: "Rev".into(),
                    instructions: "".into(),
                    learning_content: None,
                    review_steps: Some(vec!["S1".into()]),
                },
            ],
        };

        let import_plan: ImportLanguagePlan = simple.try_into().expect("Conversion failed");
        assert_eq!(import_plan.steps.len(), 2);

        // Check S1
        match &import_plan.steps[0].step_type {
            ImportStepType::Learning { content } => {
                assert_eq!(content.items.len(), 2);
                match &content.items[0].item {
                    ImportPartialLearningItem::Translation(t) => {
                        assert_eq!(t.translated_to.as_deref(), Some("hola"));
                        assert_eq!(t.translated_word.as_deref(), Some("hello"));
                    }
                    _ => panic!("Expected Translation"),
                }
                match &content.items[1].item {
                    ImportPartialLearningItem::Explanation(e) => {
                        assert_eq!(e.new_phrase.as_deref(), Some("subjunctive"));
                        assert_eq!(e.explanation.as_deref(), Some("express doubt"));
                    }
                    _ => panic!("Expected Explanation"),
                }
            }
            _ => panic!("Expected Learning"),
        }

        // Check Rev
        match &import_plan.steps[1].step_type {
            ImportStepType::Review { review_steps } => {
                assert_eq!(review_steps[0], "S1");
            }
            _ => panic!("Expected Review"),
        }
    }

    #[test]
    fn test_simple_import_errors() {
        // Error: Both content and review
        let s_both = SimpleImportLanguagePlan {
            title: "Bad".into(),
            dialect: Dialect::SpanishChilean,
            description: None,
            steps: vec![SimpleImportStep {
                title: "BadStep".into(),
                instructions: "".into(),
                learning_content: Some(vec![]),
                review_steps: Some(vec![]),
            }],
        };
        assert!(ImportLanguagePlan::try_from(s_both).is_err());

        // Error: Neither content nor review
        let s_neither = SimpleImportLanguagePlan {
            title: "Bad".into(),
            dialect: Dialect::SpanishChilean,
            description: None,
            steps: vec![SimpleImportStep {
                title: "BadStep".into(),
                instructions: "".into(),
                learning_content: None,
                review_steps: None,
            }],
        };
        assert!(ImportLanguagePlan::try_from(s_neither).is_err());

        // Error: Item has both vocab and grammar
        let s_item_both = SimpleImportLanguagePlan {
            title: "Bad".into(),
            dialect: Dialect::SpanishChilean,
            description: None,
            steps: vec![SimpleImportStep {
                title: "BadStep".into(),
                instructions: "".into(),
                learning_content: Some(vec![SimpleLearningItem {
                    vocab: Some("a".into()),
                    translation: None,
                    grammar: Some("b".into()),
                    explanation: None,
                }]),
                review_steps: None,
            }],
        };
        assert!(ImportLanguagePlan::try_from(s_item_both).is_err());
    }
}
