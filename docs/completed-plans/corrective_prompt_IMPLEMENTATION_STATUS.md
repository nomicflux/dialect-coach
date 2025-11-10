# Corrective Prompt Implementation Status

## Agreements Made
- 2025-11-09: "It should have been corrected to \"منيه -> منيح\"."
- 2025-11-09: "I also have had multiple cases where the agent provides entire phrases as mistakes and corrections, despite only a single word being wrong. (The incorrect word should be the mistake, the exact replacement the correct; anything else should be put in a BRIEF explanation field IF needed.)"
- 2025-11-09: "It is POSSIBLE for a mistake to be a phrase (multiple tokens), but then the mistake needs to be THE ENTIRE PHRASE and not one word in it."

## Explicitly Rejected
- 2025-11-09: "I also have had multiple cases where the agent provides entire phrases as mistakes and corrections, despite only a single word being wrong."

## Implementation Details
- `mistakes` entries must isolate the incorrect token in `specific_mistake` and supply the exact replacement in `correction`; any supporting context belongs in `mistake_category.context` as a brief note.
- Multi-token corrections are allowed only when the entire phrase is wrong; `specific_mistake` must capture that exact phrase (no partial tokens), and `correction` must provide the full replacement phrase.
- Corrections must respect Levantine Arabic usage, e.g., replacing "منيه" with the dialect-appropriate "منيح".

## Issues Encountered
- None recorded as of 2025-11-09.

