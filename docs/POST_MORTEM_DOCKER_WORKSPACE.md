# Post Mortem: Docker Workspace & Local Tools

## The Incident
The Docker build failed with `failed to load manifest for workspace member` because `corpus-processor`, a local-only tool listed in the `[workspace]`, was excluded from the Docker build context via `.dockerignore`.

## The Root Cause: Atomic Workspaces
The core issue is a misunderstanding of how Rust Workspaces function in relation to builds.
1.  **Workspaces are Atomic**: A `Cargo.toml` defining a `[workspace]` asserts that all listed `members` **must** exist to resolve the dependency graph (`Cargo.lock`).
2.  **Universal Resolution**: Even if you only want to build `backend`, Cargo *must* read the manifests of *all* workspace members to ensure dependencies are compatible and unified across the entire project.
3.  **The Conflict**: By excluding `corpus-processor` from Docker (to keep the image clean), we broke the atomic contract of the workspace. `cargo metadata` failed because it couldn't find a required member.

## Why Previous Attempts Were "Hacks"
1.  **`sed` Removal**: Modifying `Cargo.toml` on the fly inside Docker is brittle because it creates a "Schrödinger's Workspace" — the code thinks it's one thing in dev, but forced to be another in build. This breaks caching and reproducibility.
2.  **Partial Context (`!Cargo.toml`)**: Adding exception rules to `.dockerignore` to allow just the manifest is a common pattern, but it effectively leaks local structure into production builds and relies on silent file system syncing rules.

## The Simple Solution: Architectural Seams
The user asked: *"How do we solve it SIMPLY ... without creating brittle points of failure?"*

We cannot use `features` to conditionally include workspace members; Cargo does not support this. `workspace.members` is static.

The architectural solution is to recognize that **Production Services** and **Local Tools** often have different lifecycles and shouldn't forcibly share a lockfile if it complicates deployment.

### Recommendation: Decouple Local Tools
If `corpus-processor` is never deployed and doesn't need to share the exact same dependency versions as the backend for compilation (only logic compatibility via `shared`), it should **not** be a member of the production workspace.

**Action**:
1.  Remove `"corpus-processor"` from the root `Cargo.toml` `[workspace.members]`.
2.  Keep `corpus-processor/Cargo.toml` as is (it can still depend on `path = "../shared"`).

**Result**:
-   **Docker**: The root workspace now only contains deployment artifacts (`backend`, `frontend`). It builds cleanly without any knowledge of `corpus-processor`.
-   **Local**: You can still run `cargo run` inside `corpus-processor`. It works fine, just with its own `Cargo.lock`.
-   **Simplicity**: No sed hacks, no tricky ignore rules, no brittle config. The project structure matches reality: "These things deploy together; that thing is for my machine."
