# Mistake and Explained Implementation Status

## Implementation Phases

### Setup ✅
- [x] Move analysis doc to current-plans/
- [x] Create status tracking document

### Phase 1: Restructure Mistake ✅
- [x] Review CLAUDE.md for code style guidelines
- [x] Create MistakeCategory enum with all variants
- [x] Implement Display for MistakeCategory
- [x] Replace Mistake.content with specific_mistake and mistake_category
- [x] Update get_content() to return &self.specific_mistake
- [x] Remove From<String> trait implementations
- [x] Add unit tests for MistakeCategory Display
- [x] Add unit tests for new Mistake serialization
- [x] Update planning status doc

### Phase 2: Restructure Explained ✅
- [x] Review CLAUDE.md for code style guidelines
- [x] Replace Explained.content with new_phrase and explanation
- [x] Update get_content() to return &self.new_phrase
- [x] Remove From<String> trait implementations
- [x] Add unit tests for new Explained serialization
- [x] Update planning status doc

### Phase 3: Fix Broken Tests ✅
- [x] Review CLAUDE.md for code style guidelines
- [x] Update all existing tests using Mistake/Explained
- [x] Run cargo test --lib to verify 100% pass rate (38/38 shared, 5/5 frontend, 14/14 corpus-processor)
- [x] Fix any compilation errors in backend/frontend (none required)
- [x] Update planning status doc

### Phase 4: Update Agent Instructions ✅
- [x] Review CLAUDE.md for code style guidelines
- [x] Update output_format_spec() for Corrective mode
- [x] Update output_format_spec() for Explanatory mode
- [x] Update teaching_desc() for Corrective mode
- [x] Update teaching_desc() for Explanatory mode
- [x] Update planning status doc

### Phase 5: User Integration Testing ✅
- [x] Review CLAUDE.md for code style guidelines
- [x] User verifies end-to-end data flow
- [x] User checks UI display in learning panel
- [x] Fixed UI visibility issues (color alpha at score=0)
- [x] Updated panel styling to match design system
- [x] Added visual distinction between mistakes and explanations
- [x] Update planning status doc

---

## Agreements Made

**Date: 2025-10-24**

**User instruction (verbatim):**
> "When restructuring Mistake/Explained, should I keep the existing 'content' field for backward compatibility, or completely replace it with the new fields?"="Replace, and update the get_content methods to still provide a clear place to get the simple STring (specific_mistake, new_phrase)"

**User instruction (verbatim):**
> "After restructuring, what should Mistake.get_content() return for the UI display?"="Just specific_mistake"

**User instruction (verbatim):**
> "After restructuring, what should Explained.get_content() return for the UI display?"="Just new_phrase"

**User instruction (verbatim):**
> "Should I remove the From<String> trait implementations for Mistake/Explained since they won't make sense with structured data?"="Yes, remove them"

**User instruction (verbatim):**
> "Every phase must start with 'Review @.claude/CLAUDE.md for code style guidelines.' Every phase must end with 'Update the planning status doc.'"

---

## Explicitly Rejected

- Keeping 'content' field for backward compatibility (user chose to replace entirely)
- Keeping From<String> trait implementations (user chose to remove them)

---

## Discoveries and Corrections

### Setup Phase
- Successfully moved analysis doc to current-plans/
- Created status tracking document with all required sections

### Phase 1 & 2 & 3
**User correction (verbatim):**
> "No useless comments. Ruthless simplicity. Don't add shit you don't need."

**User correction (verbatim):**
> "If 'type' is going to be a pain to use, rename the GrammaticalError field to 'category'"

- Combined Phases 1 and 2 into single edit session (both restructuring same file)
- Changed GrammarError field from `type` to `category` to avoid keyword issues
- All tests updated and passing (100% pass rate: 38 shared + 5 frontend + 14 corpus-processor = 57 total)
- No compilation errors in any crate

### Phase 4
**User feedback (verbatim):**
> "Hmm, seeing this, I think it will confuse the AI and increase context needlessly. While I don't LIKE losing the readibility of the specific names, show what would be necessary to change all of the MistakeCategory fields to be named 'context'."

**User decision (verbatim):**
> "Yes" [to changing all fields to "context"]

**User correction (verbatim):**
> "I think we still need to explain what exactly is 'relevant info' per category, we can just make the format easier."

- Simplified all MistakeCategory fields to use uniform name "context"
- Updated output_format_spec() to explain what context means for each category type
- Updated teaching_desc() with instructions on when to populate mistakes/explained arrays
- All tests passing (57/57), all crates compile successfully

### Phase 5
**User issue (verbatim):**
> "Biggest issue for the moment: The UI doesn't work. <ul class='learning-items'>...</ul> But nothing shows up. I think the text is the same color as its background. (Also, the white background of the panel is jarring, and its sharp edges don't cohere with the more rounded nature of most of the rest of the UI.)"

**User correction (verbatim):**
> "WHAT THE BLOODY HELL ARE YOU DOING????? THE COLOR MUST BE DETERMINED FROM THE SCORE> THE SCORE MUST BE PASSED IN."

**User request (verbatim):**
> "Ok, let's start at 50%. And make it clear when we are looking at mistakes vs explanations."

**User request (verbatim):**
> "Good start, but also, can we make the specific item's background color (or border?) different for mistake vs explanation?"

**User approval (verbatim):**
> "Nice. That's what I wanted. Bárbaro, as the Argentineans say."

- Fixed color alpha issue: changed from 0-100% to 50-100% opacity range
- Mistakes now use coral/red color, explanations use teal color
- Added distinct background tints and left borders (coral for mistakes, teal for explanations)
- Updated panel styling: rounded corners (--r-l), softer background (--surface-muted), design system tokens
- Scores properly passed and displayed, starting at 0 with visible text

---

## Lessons Learned

### Setup Phase
- Always include "Review CLAUDE.md" at start of each phase
- Always include "Update planning status doc" at end of each phase
- Document user agreements verbatim, not paraphrased

### Phases 1-3
- Remove all unnecessary comments from code (ruthless simplicity)
- Avoid using Rust keywords as field names (use synonyms like "category" instead of "type")
- When restructuring related types in same file, can combine phases efficiently
- Updated tests must use new struct format with all required fields

### Phase 4
- Uniform field naming reduces AI context and confusion (all fields named "context")
- Still need to document what "context" means for each category type in agent instructions
- Simpler JSON schema: just {"type": "category_name", "context": "relevant_info"}
- Agent instructions now explicitly state when to populate mistakes/explained arrays

### Phase 5
- Score-based color opacity must start high enough to be visible (50% minimum)
- Different colors for different item types improves UX (coral=mistakes, teal=explanations)
- Background tint + border provides clear visual distinction without being overwhelming
- Must preserve score-based progression (scores are for future enhancement, not current display)
