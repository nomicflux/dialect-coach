# Brainstorm: Plan Generator Agent

## Objective
Create a backend agent capable of converting raw documents (Text, HTML, PDF) into structured Learning Plans (`SimpleImport` format).

## User Requirements
1.  **Input**: UI to accept documents (Text/File).
2.  **Processing**: Split content into steps (learning & review).
3.  **Constraints**:
    *   Reviews at least every 4 steps.
    *   Max 10 items per learning step.
    *   Output: `SimpleImport` format.

## Proposed Architecture

### 1. Backend Agent Service
We will add a new agent channel `PLANNING` to the `AgentService`.
-   **File**: `backend/src/agent_service/plan_builder.rs` (New Module)
-   **Configuration**: Load `PLANNING_PROVIDER`, `PLANNING_MODEL` (defaulting to existing envs if unset).
-   **Prompt Strategy**:
    -   **Role**: Curriculum Designer.
    -   **Context**: "You are an expert at breaking down complex texts into manageable language learning chunks."
    -   **Constraints**: Explicitly list the numerical constraints (Max 10 items, Max 4 steps gap for reviews).
    -   **Schema**: Provide the definition of `SimpleImportLanguagePlan` JSON.

### 2. Document Processing Pipeline
We need to extract raw text before sending it to the LLM.
-   **Text**: Pass through.
-   **HTML**: Use `html2text` crate to strip markup and preserve readable structure.
-   **PDF**: Use `pdf-extract` (or `lopdf` if `pdf-extract` has broad system deps) to extract raw text layers.
    -   *Risk*: PDF columns/tables can result in jumbled text. We will accept "raw stream" text for MVP.

### 3. API Endpoint
-   `POST /api/plans/generate`
-   **Body**: Multipart form data.
    -   `file`: Binary (PDF/HTML) OR `text`: String.
    -   `dialect`: For context.
-   **Flow**:
    1.  Handler receives file.
    2.  Determines MIME type.
    3.  Extracts text string.
    4.  Calls `agent_service.generate_plan(text)`.
    5.  Returns JSON `SimpleImportLanguagePlan`.

### 4. Frontend Integration
-   **Location**: `ImportWorkflow` sidebar.
-   **UI**:
    -   New "AI Generator" tab.
    -   File Upload (Drag & Drop zone).
    -   Text Area (Paste content).
    -   "Generate Plan" button (with loading spinner).
    -   **Result**: Populates the existing "Import Text" area with the simplified YAML/JSON, allowing the user to review/tweak before hitting "Import".

## Dependencies Required
-   `html2text` (Backend) - For HTML parsing.
-   `pdf-extract` (Backend) - For PDF text extraction.
-   `multer` or `axum::extract::Multipart` (Backend) - For file uploads.

## Feasibility & Risks
-   **Context Window**: Large documents (entire books) will exceed token limits.
    -   *Mitigation*: For MVP, enforce a character limit (e.g., 100k chars) and return an error if exceeded.
-   **Hallucination**: LLM might miss the "Max 10 items" constraint.
    -   *Mitigation*: We can implement a post-processing step in Rust to validate the plan. If a step has >10 items, we can programmatically split it, or just warn the user. (Prompting usually works well for this count, though).

## Questions for User
1.  Is adding `pdf-extract` (and potentially `poppler` system dependency) acceptable, or should we stick to pure-Rust text extraction (which might be less accurate for complex PDFs)?
2.  Should the agent *only* generate the plan structure, or should it also try to extract the content (create translation items)? (Assumption: It extracts content as `SimpleLearningItem`s).
