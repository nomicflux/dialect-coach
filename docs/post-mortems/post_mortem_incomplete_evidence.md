# Post-Mortem: Incomplete Evidence & Inference vs Proof

## Date
2026-01-04

## Incident
The agent was tasked with verifying the research for a configuration change (`data-wasm-opt-params`). The user explicitly demanded a quote proving "BOTH THE FLAG _AND_ THE PLACEMENT".

The agent provided a quote describing the flag (`data-wasm-opt-params`) and *inferred* the placement based on the context of the documentation section (`Link Asset Types / rust`).

The user correctly identified that the provided quote **did not explicitly state** that this tag belongs in `index.html`, failing the strict evidentiary standard.

## Root Cause Analysis
1.  **Inference as Evidence**: The agent relied on the *implied* context that "Link Asset Types" are HTML tags and therefore go in an HTML file. It did not find and quote the specific sentence linking these tags to the "source HTML file".
2.  **Failure to Quote "The Glue"**: The documentation contains the text: *"If not specified, Trunk will look for a Cargo.toml in the parent directory of the **source HTML file**."* This is the text that explicitly places the `rel="rust"` tag within the HTML file. The agent omitted this crucial context.
3.  **Laziness in Citation**: The agent quoted the attribute definition but failed to quote the *parent* definition that anchors the attribute to the file type.

## Corrective Actions
1.  **Complete Chain of Custody**: Evidence must show the full chain: `Attribute` -> `Tag` -> `File`.
    *   *Bad*: Quoting just the Attribute.
    *   *Good*: Quoting the Attribute AND the text saying "This tag goes in the source HTML file."
2.  **No Implied Context**: If the prompt asks for proof of placement, "It's a link tag" is not proof. You must quote the text that says "HTML file".

## Assessment
The verification was rejected because it required the user to trust the agent's inference about where "Link Asset Types" go. Strict verification requires zero trust. The agent failed to provide the complete textual proof.
