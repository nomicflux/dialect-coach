# Task: Resolve Neon Rope Rendering Failure (Agent 7)

**Context**:
You are the **7th Agent**. The Neon Rope renders White/Grey.

**The Diagnosis (Verbatim from External Analysis)**:
> The SVG references a gradient and a filter that are **not defined anywhere in the SVG**.
> - `stroke="url(#global-neon-gradient)"`
> - `filter="url(#global-neon-glow)"`
> There is no corresponding `<defs>` block defining them in the same SVG.

**The Mandatory Fix**:
> **Both must be inside `<defs>` and inside the same `<svg>` element as the referencing element.**
> Defining the missing `<defs>` in the correct scope will restore the intended gradient rendering.

**Your Mission**:
Execute this fix. Ensure the `<defs>` (Gradient & Filter) are physically present inside the same `<svg>` element that utilizes them.

**Deliverable**:
A working gradient.
