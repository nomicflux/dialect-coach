# New Learning Items for Interleaved and Storyteller Modes

## Context

- We added Mistake and Explained for Corrective and Explanatory modes, respectively
- These include new structs to contain the data, changes to the AgentAnalysis type to record them, new prompts to get
  the responding agent to provide the data, a second agent to analyze whether responses afterward apply the new learning
  items, and places in the UI to show the learning items. Please review all of these.
- When forming the implementation plan for this, add:
    - At the start of EVERY PHASE, a direction to review CLAUDE.MD for code style guidelines
    - At the end of EVERY PHASE, a direction to update the status document with completed work, verbatim user
      conversations, changes made, approaches rejected, and other relevant information

## Next Steps

- Interleaved should do the same thing with Translated items:
    - Include a Translated struct like Mistake. It should have id, translated_word, translated_to. 
    - It should implement a get_content with "{translated_word -> translated_to}"
    - AgentResponse should have a new translated: Option<Vec<Translated>> field
    - Add in a new prompt instructing the AI agent to produce JSON results in this format in output_format_spec.
    - Update agent_analysis_prompt to take in &[Translated].
        - AgentAnalysis should include the relevant HashMap for Translated
        - Include PAST TRANSLATIONS
        - Score: (-10 to 10): -10 for reverting to untranslated version, 0 for not using it, 10 for correctly using the
          translated form
        - Include score in returned JSON
    - Update UI to include new Translated item alongside Mistake and Explained
        - Should be added likewise to learning panel
        - Should be sent for analysis same as mistakes and explained
        - Coloring should be different than the teal of Explained and coral of Mistake
          - We will also include a fourth item, so keep that in mind for chosing a new color here; we want four
            relatively different colors. Make this one something yellowish, but fitting the palette.

- Storyteller should do the same thing with Exploratory items:
    - Include a Exploratory struct like Translated. It should have id, point_to_try, instructions_for_use.
    - It should implement a get_content with "{point_to_try}"
    - AgentResponse should have a new exploratory: Option<Vec<Exploratory>> field
    - Add in a new prompt instructing the AI agent to produce JSON results in this format in output_format_spec.
    - Update agent_analysis_prompt to take in &[Exploratory].
        - AgentAnalysis should include the relevant HashMap for Exploratory
        - Include PAST EXPLORATIONS
        - Score: (0 to 10): 0 for not using it, 5 for imperfect attempts, 10 for correctly using the point_to_explore
        - Include score in returned JSON
    - Update UI to include new Exploratory item alongside Mistake, Explained, and Translated
        - Should be added likewise to learning panel
        - Should be sent for analysis same as mistakes and explained
        - Coloring should be greenish (fitting pattern and distinguishable from the other three colors)
