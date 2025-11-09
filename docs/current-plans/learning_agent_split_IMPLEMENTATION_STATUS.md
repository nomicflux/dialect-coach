# Learning Agent Split Implementation Status

## Agreements Made
- 2025-11-09: "We will introduce a third agent along response and analysis: learning."
- 2025-11-09: "The learning agent will receive the LATEST message from the user, the current response from the response agent (it runs AFTER the response agent to receive this), the current list of learning items, and learning goals"
- 2025-11-09: "Response agent will retain most of its preamble concerning what responses to give. This will include TeachingMode preambles that do not involve producing the specific JSON format for learning items and the items to include in that. It will also still receive learning goals and learning items to guide it."
- 2025-11-09: "Learning agent will receive the preamble concerning JSON output as well as the portions of the preambles for response agents regarding learning item output. The system preambles will need to be updated to reflect the need for agents to focus ONLY on learning items, and especially focus on learning items related to learning goals, and as appropriate, to work in what was featured in the response agent's response (for example, storytelling exploration items should come from the response agent's response)."
- 2025-11-09: "Learning agent usage must be tracked like response and analysis agents"
- 2025-11-09: "Learning agent will use the same retry logic in @retry.rs as response and analysis agents."
- 2025-11-09: "We need to record stats (token usage, calls) for learning agents just like response and analysis agents."
- 2025-11-09: "This also requires frontend changes to display the stats."

## Explicitly Rejected
- None noted as of 2025-11-09.

## Implementation Details
- Introduce a dedicated learning agent module that runs after the response agent, consuming the latest user message, the freshly generated assistant response, existing learning items, and learning goals.
- Refactor response-agent preambles to exclude learning-item formatting requirements while still surfacing goals and existing items for conversational guidance.
- Consolidate learning-item prompt rules into shared helpers for reuse by the new learning agent, ensuring retry logic and usage tracking mirror existing agents.
- Added `response_output_format_spec`, `learning_output_format_spec`, `response_teaching_desc`, and `learning_teaching_desc` in `backend/src/agent_service/util.rs` to separate conversational guidance from learning-item directives without yet disrupting existing call sites.
- Updated `backend/src/agent_service/response.rs` so the response agent preamble now references `response_output_format_spec`, surfaces existing learning items in-context, and defers learning-item generation to a new `LearningAgent` invoked after successful conversation parsing. `GenerateResponseParams` now carries prior learning items to both agents, and placeholder `backend/src/agent_service/learning.rs` provides the interface for the upcoming learning agent implementation.
- Refactored `format_learning_items_context` in `backend/src/agent_service/response.rs` to use a reusable `format_learning_section` helper, keeping each function short and compliant with the code style constraints.
- Moved learning-item formatting helpers into `backend/src/agent_service/util.rs` and implemented the full `LearningAgent` flow in `backend/src/agent_service/learning.rs`, including retry-aware prompting, JSON parsing, and usage tracking consistent with existing agents.
- Added unit coverage for learning-item formatting and learning-agent prompt/parse helpers to enforce the new behavior.
- Restored the `rig::completion::Completion` import after refactoring the learning agent so the Anthropic completion helper resolves correctly.
- Extended `UsageStats` (shared), backend usage tracking, and websocket plumbing so learning-agent usage events are recorded separately; updated the frontend footer and websocket logging to display learning calls/tokens alongside existing metrics.
- Restored response-agent teaching prompts to their original wording (minus JSON directives) so conversational guidance stays consistent with the pre-split behavior.

## Issues Encountered
- None encountered as of 2025-11-09.

