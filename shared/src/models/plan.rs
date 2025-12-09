use super::dialect::Dialect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::learning_item::LearningItem;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguagePlan {
    pub id: Uuid,
    pub title: String,
    pub dialect: Dialect,
    pub description: Option<String>,
    pub steps: Vec<PlanStep>,
    pub current_step_index: usize,
    pub status: PlanStatus,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: Uuid,
    pub step_number: usize,
    pub title: String,
    pub step_type: StepType,
    pub instructions: String,
    pub completion_criteria: CompletionCriteria,
    pub status: StepStatus,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub content: PlanContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PlanContent {
    pub items: Vec<LearningItem>,
    pub agent_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepType {
    Learning {
        focus: String,
    },
    Review {
        review_step_ids: Vec<Uuid>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompletionCriteria {
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    NotStarted,
    InProgress,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    NotStarted,
    InProgress,
    Completed,
}

impl LanguagePlan {
    pub fn new(title: String, dialect: Dialect, description: Option<String>, steps: Vec<PlanStep>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            dialect,
            description,
            steps,
            current_step_index: 0,
            status: PlanStatus::NotStarted,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn current_step(&self) -> Option<&PlanStep> {
        self.steps.get(self.current_step_index)
    }

    pub fn current_step_mut(&mut self) -> Option<&mut PlanStep> {
        self.steps.get_mut(self.current_step_index)
    }

    pub fn is_complete(&self) -> bool {
        self.status == PlanStatus::Completed
    }

    pub fn advance_step(&mut self) -> bool {
        if self.is_complete() {
            return false;
        }

        let total_steps = self.steps.len();
        
        // Mark current step as completed
        if let Some(step) = self.current_step_mut() {
            step.status = StepStatus::Completed;
            step.completed_at = Some(chrono::Utc::now().timestamp());
        }

        // Advance index
        if self.current_step_index + 1 < total_steps {
            self.current_step_index += 1;
            
            // Start next step
            if let Some(step) = self.current_step_mut() {
                step.status = StepStatus::InProgress;
                step.started_at = Some(chrono::Utc::now().timestamp());
            }
            true
        } else {
            // Plan completed
            self.status = PlanStatus::Completed;
            true
        }
    }

    pub fn start(&mut self) {
        if self.status == PlanStatus::NotStarted {
            self.status = PlanStatus::InProgress;
            if let Some(step) = self.current_step_mut() {
                step.status = StepStatus::InProgress;
                step.started_at = Some(chrono::Utc::now().timestamp());
            }
        }
    }
}

impl PlanStep {
    pub fn new(
        step_number: usize,
        title: String,
        step_type: StepType,
        instructions: String,
        content: PlanContent,
        completion_criteria: CompletionCriteria,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            step_number,
            title,
            step_type,
            instructions,
            completion_criteria,
            status: StepStatus::NotStarted,
            content,
            started_at: None,
            completed_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_lifecycle() {
        let step1 = PlanStep::new(
            1,
            "Step 1".to_string(),
            StepType::Learning { focus: "Basics".to_string() },
            "Instructions".to_string(),
            PlanContent::default(),
            CompletionCriteria::Manual,
        );
        let step2 = PlanStep::new(
            2,
            "Step 2".to_string(),
            StepType::Learning { focus: "Advanced".to_string() },
            "Instructions".to_string(),
            PlanContent::default(),
            CompletionCriteria::Manual,
        );

        let mut plan = LanguagePlan::new(
            "Test Plan".to_string(),
            Dialect::SpanishMexican,
            None,
            vec![step1, step2],
        );

        assert_eq!(plan.status, PlanStatus::NotStarted);
        assert_eq!(plan.current_step_index, 0);

        // Start plan
        plan.start();
        assert_eq!(plan.status, PlanStatus::InProgress);
        assert_eq!(plan.current_step().unwrap().status, StepStatus::InProgress);

        // Advance step 1 -> 2
        let advanced = plan.advance_step();
        assert!(advanced);
        assert_eq!(plan.current_step_index, 1);
        assert_eq!(plan.steps[0].status, StepStatus::Completed);
        assert_eq!(plan.current_step().unwrap().status, StepStatus::InProgress);

        // Advance step 2 -> Complete
        let advanced = plan.advance_step();
        assert!(advanced);
        assert_eq!(plan.status, PlanStatus::Completed);
        assert_eq!(plan.steps[1].status, StepStatus::Completed);

        // Try to advance again
        let advanced = plan.advance_step();
        assert!(!advanced);
    }
}
