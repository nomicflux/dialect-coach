use dialect_coach_shared::Dialect;
use crate::agent_service::util;


pub fn build_planning_system_prompt_yaml(dialect: Dialect) -> String {
    format!(
        "# CURRICULUM DESIGNER INSTRUCTIONS
        You are an expert curriculum designer. Your task is to convert raw text into a structured language learning path for {}.

        # GOAL
        Create a YAML learning plan that breaks the text into digestible learning steps and review steps.

        # CRITICAL CONSTRAINTS
        1. **Step Size**: Each 'Learning Step' must contain AT MOST 10 new items (vocab or grammar). If a section has more, split it into multiple steps.
        2. **Review Cadence**: Reviews no more than 4 learning steps apart. You MUST insert a 'Review Step' to ensure this gap is never exceeded.
        3. **Content Extraction**: 
           - Extract key vocabulary (translated).
           - Extract key grammatical concepts (explained).
           - Ignore filler text, introductions, or irrelevant digressions.
        4. **Mutual Exclusivity**: A step MUST be EITHER a 'Learning Step' (containing `learning_content`) OR a 'Review Step' (containing `review_steps`). It CANNOT be both.

        # PROCESS (STEP-BY-STEP)
        1. **Analyze**: Read the text and identify the main topic and dialect nuances.
        2. **Extract**: Identify specific phrases taught (Vocab) and rules explained (Grammar).
        3. **Segment**: Group these items into logical 'Learning Steps' (max 10 items each).
        4. **Review**: Interleave 'Review Steps' according to the cadence rule (max gap 4).
        5. **Format**: Output the final YAML.

        # FIELD GUIDELINES & LANGUAGE RULES
        - **title**: Descriptive title for the step.
        - **instructions**: Instructions **FOR THE AI TEACHER** on how to conduct this step (e.g., \"Focus on gender agreement\", \"Roleplay a market scene\"). Do NOT write instructions for the user to read.
        - **learning_content**:
          - **vocab**: The **TARGET LANGUAGE** phrase (e.g. \"soñar con\").
          - **translation**: The **ENGLISH** equivalent (e.g. \"to dream about\").
          - **grammar**: The concept name (e.g. \"Subjunctive Mood\").
          - **explanation**: The rule explanation in **ENGLISH**.
        - **review_steps**: EXACT titles of previous steps to review.

        
        # OUTPUT SCHEMA
        {}
        title: \"Plan Title\" (e.g. \"Verbs with Con\")
        dialect: \"{}\"
        steps:
          - title: \"Part 1: Basic Usage\"
            instructions: \"Focus on mastering the verb structure.\"
            learning_content:
              - vocab: \"soñar con\"
                translation: \"to dream about\"
              - grammar: \"Use of 'con'\"
                explanation: \"Used to indicate accompaniment...\"
          - title: \"Review: Basics\"
            instructions: \"Review the concepts from Part 1.\"
            review_steps: 
              - \"Part 1: Basic Usage\"
        ",
        dialect,
        util::YAML_OUTPUT_INSTRUCTION,
        dialect.id()
    )
}

pub fn build_planning_user_prompt_yaml(text: &str) -> String {
    format!(
        "Create a learning plan from this text. Return raw YAML:\n\n{}",
        text
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::Dialect;




    #[test]
    fn test_build_prompts_yaml() {
        let sys = build_planning_system_prompt_yaml(Dialect::SpanishMexican);
        assert!(sys.contains("CURRICULUM DESIGNER"));
        assert!(sys.contains("AT MOST 10 new items"));
        assert!(sys.contains("Review Step"));
        assert!(sys.contains("YAML"));
        assert!(!sys.contains("JSON"));

        let user = build_planning_user_prompt_yaml("input text");
        assert!(user.contains("input text"));
        assert!(user.contains("YAML"));
        assert!(sys.contains("spanish_mexican")); // Verify ID is used in schema
    }
}
