# Feedback System - Specification

**Status**: Specification Phase
**Created**: 2025-11-09

## Overview

Add a user feedback system that allows users to submit bug reports, AI quality issues, and general suggestions. The system should capture feedback text and optionally include conversation history for debugging purposes.

## User-Facing Features

### 1. Feedback Modal Window
- New modal component that can be opened from the UI
- Contains a form for submitting feedback
- Modal can be dismissed/closed without submitting

### 2. Core Form Elements

**Free-form text input:**
- Multi-line textbox for feedback content
- User can write any feedback they want
- Required field (cannot submit empty feedback)

**Optional conversation history attachment:**
- Checkbox control (unchecked by default)
- Label: "Include my conversation history with this feedback"
- When checked: current conversation history will be included with submission
- When unchecked: only the feedback text is submitted

### 3. Feedback Guidelines

Display prominent guidelines in the modal explaining appropriate use cases:
- "Submit inappropriate AI answers"
- "Submit errors from AI"
- "Report restrictive rate limiting issues"
- "Report inaccurate dialect examples"
- "Report inaccurate learning items"
- "UI suggestions and improvements"

These guidelines help users understand what kind of feedback is valuable.

### 4. Submission

- Submit button to send feedback
- Cancel/close button to dismiss modal without submitting
- Visual feedback during submission (loading state)
- Success/error messaging after submission attempt

## Backend Requirements

### 1. Feedback Storage

Persist feedback with the following fields:
- `user_id`: ID of the user submitting feedback
- `timestamp`: When the feedback was submitted
- `feedback_text`: The free-form text content
- `conversation_history`: Optional field - ONLY populated if user checked the box
- Additional metadata (TBD during implementation planning)

### 2. Data Model

New data structure needed:
```rust
struct Feedback {
    user_id: String,
    timestamp: i64,
    feedback_text: String,
    conversation_history: Option<Vec<Message>>,  // Only if checkbox checked
    // Additional fields TBD
}
```

### 3. API Endpoint

New endpoint to receive feedback submissions:
- Route: TBD
- Method: POST
- Auth: Requires authenticated user
- Payload: Feedback text + optional conversation history
- Response: Success/error status

## Technical Considerations

### Frontend (Yew/WASM)
- New modal component
- Form state management
- Conversation history serialization (when checkbox is checked)
- API client call to submit feedback

### Backend (Axum)
- New API route handler
- Feedback storage mechanism (database, file, service - TBD)
- Validation of feedback submissions
- User authentication/authorization

### Shared Types
- Feedback request/response types in `dialect-coach-shared`
- Serialization format for conversation history

## Open Questions (To Be Resolved During Planning)

1. **Storage Backend**: Where do we persist feedback?
   - Database table?
   - File-based logging?
   - External service?

2. **Rate Limiting**: How do we prevent spam?
   - Per-user submission limits?
   - Time-based throttling?

3. **Conversation History Size**:
   - Do we limit how much history can be attached?
   - Full conversation or recent N messages?

4. **Privacy/Data Retention**:
   - How long do we keep feedback?
   - User consent for conversation history storage?

5. **Feedback Review**:
   - Admin interface to view feedback?
   - Export mechanism?

6. **Modal Trigger**:
   - Where in the UI does the "Submit Feedback" button live?
   - Always visible or in a menu?

## Success Criteria

- Users can open feedback modal from anywhere in the app
- Users can submit text feedback successfully
- Users can optionally attach conversation history
- Feedback is persisted with user_id and timestamp
- Conversation history is ONLY stored when explicitly requested
- Clear guidelines are visible to users
- Submission succeeds/fails with appropriate user feedback

## Out of Scope (For This Spec)

- Admin interface for reviewing feedback
- Automated feedback categorization/routing
- Email notifications
- Feedback response/resolution tracking
- Public feedback visibility
