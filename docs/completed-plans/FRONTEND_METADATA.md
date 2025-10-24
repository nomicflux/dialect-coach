# Specification for Frontend AgentResponse Views

- Message contains an AgentResponse, which contains mistakes and explanations as optional vectors
- If they exist, their content should be shown in a side panel (collapsible, but keeping track of currently marked
  mistakes and explanations over time)
- Helper methods should be added to shared to extract this information - we will be updating the structure of Mistake
  and Explained, so we want to change that in _one_ place
- I need help on something - we will eventually want to record scores for each mistake and explanation based on agent
  feedback in the backend. (FUTURE STEP. _DO NOT IMPLEMENT_, this is just to give you context.) For example, start at a 0 for a new mistake, increase it
  every time the user corrects that mistake, and decrease it every time the user repeats the mistake.). For now, we can
  persist in-memory in the UI.
- Side panel should show each mistake and explanation clearly - these should be short (single words / phrases), kept in
  a list, with something to match their score (between 0 and 100, 0 being newly introduced, 100 being well-practiced).
  Score should not be numerically displayed, but given in some other way - for example, with 0 being a dull grey (but
  readable!) word/phrase, and 100 being a bright, colorful word (with the color being determined by the full data - 
  you don't have the fields yet, but prepare for the AgentResponse's Mistake and Explained being passed in fully 
  to determine this), and also maybe a little vertical bar to the side of the word/phrase that starts empty and is full
  at 100.
