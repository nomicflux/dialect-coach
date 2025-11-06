# Rate Limiting

We need to implement rate limits for users for agent usage and for TTS usage, as these require access to a limited pool
of resources.

In order to have rate limits, we need to first implement tracking.

As usual, when turning this into a plan:
1. The plan must be output to the docs/current-plans folder.
2. Each phase must start with the code style checklist using CLAUDE.md code style guidelines.
3. Each phase must end with a reminder to update a status document with progress.
4. Research the code before coming up with the plan.
5. If after researching the code you have questions, ask for clarification.

## Tracking

1. Create a new struct, UsageStats. It should have:
    - Response agent usage (calls, number of tokens used in input, number of tokens used in output)
    - Analysis agent usage (calls, number of tokens used in input, number of tokens used in output)
    - TTS usage (calls, characters sent, research if we receive back any sort of token usage from calls or if any such
      endpoints exist; there are characters remaining, this is probably what we want)
2. Add UserStats to UserState
3. Make sure that UserStats are persisted (this may be automatic with Sled, but check. No backwards compatibility or
   migration path necessary; we can delete database and start fresh.)
4. Add collapsible footer to UI to show usage

## Endpoint checks

1. Add in endpoints for Anthropic and ElevenLabs to check usage (research online; we have started this with the admin
   page, but I don't know that we have useful data yet for these purposes. We specifically need to know how many tokens
   / money / usage we have remaining.)

## Rate Limiter

1. Add rate limiter service trait
    - Should have methods to take UserStats and determine whether user can make analysis / response calls, wether user
      can make TTS calls, and whether the analysis / response agents are out of credits (see endpoint checks) or TTS is
      out of credits (see endpoint checks)
2. Implement an instance of the rate limiter that is available at websocket.rs when making agent or TTS calls
3. Make the number of calls & tokens that the user can use for analysis agents, response agents, and TTS its own
   configurable struct
4. If user makes too many calls or uses too many tokens per service, OR services are out of credits, send back a response instead of making a call:
    - For analysis agent, this will apply for any teaching mode other than Immersive or Debug
    - For response agent, this will apply to any call
    - For TTS, this will apply to any call
    - Use a status code (429) to indicate. We'll need to indidicate analysis / response agents differenty

## Frontend Processing

1. Frontend needs to handle 429 responses and store them in state
    - 429 responses for analysis agent will disable teaching modes other than Immersive or Debug
    - 429 responses for response agent will disable chat
    - 429 responses for synthesis will disable autospeech and manual speech
