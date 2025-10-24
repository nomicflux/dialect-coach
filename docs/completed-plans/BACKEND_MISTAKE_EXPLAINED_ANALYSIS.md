# Backend: Agent Analysis of Mistakes and Explanations - Initial Creation

- As usual, once the detailed plan is laid out, every phase should start with "Review CLAUDE.MD code style guidelines",
  and every phase should end with "Update the status doc with work. Include discoveries and corrections, as well as
  verbatim user conversations."

## Background

- We have updated Message to accept AgentResponse for content
- AgentResponse includes optional vectors of Mistakes and Explaineds
- These are set to None right now
- Agents are instructed to return JSON structures based on TeachingMode
- Right now, only string responses are sent

## Next Steps

### 1. Update Mistake

- Mistake should take in "specific_mistake" as a String and "mistake_category"
- MistakeCategory is an enum with SpellingError { correction: String }, GrammarError { type: String }, DialectUsageError { preferred_usage:
  String }, and Other { explanation: String }.
- MistakeCategory should have implement `Display` with a description of the error:
    - SpellingError(correction): "Correct spelling is {correction}"
    - GrammarError(type): "Example of a grammatical error {type]"
    - DialectUsageError(preferred_usage): "Better usage would be {preferred_usage}"
    - Other(explanation): "{explanation}"
- When in Corrective mode, the agent should be instructed to return response as before as well as a list of mistakes.
  Mistakes should be able to be serialized as Vec<Mistake>. The content of each mistake is the exact word or phrase 
  that is problematic (keep it as brief as possible; single words if possible) along with the rest of the information relevant to the
  category (Consult the above list and ask for clarification as soon as it is needed). Update the `output_format_spec`
  accordingly.

### 2. Update Explained

- Explained should take in "new_phrase" as a String and "explanation" as a String.
- When in Explanatory mode, the agent should be instructed to return response as before as well as a liste of
  Explaineds. These should each include both the specific word or phrase introduced, as well as their usage explanation
  (keep brief).

### 3. Testing

- These should be confirmed to be sent to the UI and shown there appropriately.
    - Add in unit tests to make sure individual pieces work as expected
    - User will check overall integration
