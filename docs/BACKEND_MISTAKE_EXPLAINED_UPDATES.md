# Backend: Agent-Driven Mistake and Explanation Updates

## Context

- Previous work included Mistake and Explained coming from the Agent
- UI is storing past Mistakes and Explaineds in memory with scores, but no updates
- So we get new mistakes and explanations and can display them, but they are static afterward

## Next Steps

### 1. Pass back to agent

- We need to pass back the current list of mistakes and explanations along with user input
- Filter out entries that have scores of 100.
- These need to be accepted on the backend
- Relevant data structures need to be updated to allow this

## 2. Data Structure Updates

- Mistake and Explained should have Ids
- AgentResponse needs to have AgentAnalysis added
- AgentAnalysis should be a struct with:
    - A hashmap with mistake ids for keys and LearningItemScore for values
    - A hashmap with explained ids for keys and LearningItemScore for values
- LearningItemScore is a struct with a score (from -10 to 10, int)

## 3. Agent Analysis

- We need to have a second agent perform analysis of the conversation history.
- Prompts need to clearly situate them within the language & dialect
- The will receive user messages from the conversation history and a list of mistakes and explainations from the user
  input (the ones provided previously from agents)
- The agent should be instructed to produce JSON output with AgentAnalysis for mistakes and explaineds
- Instructions should be that it should output:
    - For each mistake: a score from -10 to 10 on improvement (-10: still occurring regularly, 0: no usage OR different
      error being made for same base issue, 10: regularly fixed)
    - For each explained: a score from 0 to 10 on usage (0: feature not used, 5: feature used but not quite
      correctly, 10: feature used in a dialect-appropriate manner)
- Have main agent workflow ship off this request to the second agent in parallel and combine for full AgentResponse


## 4. Pass back to UI

- Make sure that AgentResponse has full information and is serializable / deserializable

## 5. Update UI in-memory scores

- Associate update with existing in-memory mistakes and explaineds
- Add AgentAnalysis scores to UI items (update to use IDs to match them). Cap out at 100.
- Once entries reach 100, move them to a different section (accomplishments). Per step one, they are not going to be
  sent to the agent any longer.

## 6. Other UI changes

- Add tooltips to UI for mistakes and explanations to show the information beyond the content (do not show scores, but
  do show explanations from the AIs)
