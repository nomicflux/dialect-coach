# Add gender for user

- User will need to have a gender flag
  - Enum with Male, Female, Non-binary
- This should be changeable, so the user can change between or within sessions
  - Create new drop-down in UI, in practice settings
  - Save as part of UserState
  - Ensure saved in persistence with other settings
- Must be sent to response agent preamble
  - Agent needs to clearly know gender of client at the time of the latest response 
  - Again, this might change through a session (a client practicing different language forms, for example)
  - Make it clear in the response preamble
