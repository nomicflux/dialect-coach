# Add gender to agent

- Agents will need to register whether they are male-presenting or female-presenting
- This depends on the TTS voice
- We need to create a TTS struct with:
  - Provider enum (ElevenLabs, Azure)
  - Voice name (String)
  - Gender (MalePresenting, FemalePresenting)
  - Set all to FemalePresenting for now, I will update with actual values after research
  - Gender is entirely determined by TTS for now
- Gender will need to be fed into the response preamble to guide the response agent to know its gender
  - This is especially important for languages that change declensions and conjugations based on gender
