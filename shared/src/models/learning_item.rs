use super::dialect::Dialect;
use super::{Explained, Exploratory, Mistake, Translated};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningItem {
    pub item: LearningItemType,
    pub score: u8,
    pub dialect: Dialect,
}

impl LearningItem {
    /// Create a new learning item with score 0
    pub fn new(item: LearningItemType, dialect: Dialect) -> Self {
        Self {
            item,
            score: 0,
            dialect,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LearningItemType {
    Mistake(Mistake),
    Explanation(Explained),
    Translation(Translated),
    Exploration(Exploratory),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningGoal {
    pub goal: String,
    pub dialect: Dialect,
}
