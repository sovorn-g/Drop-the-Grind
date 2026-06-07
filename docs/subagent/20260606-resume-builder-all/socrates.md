# Reflection Review – Resume Builder All Feature

## Scope Reviewed

**Files/artifacts reviewed:**
- `docs/subagent/20260606-resume-builder-all/curie.md` (discovery handoff)
- `docs/subagent/20260606-resume-builder-all/davinci.md` (implementation plan)
- `src-tauri/src/resume.rs` (skill registry, render pipeline, ResumeInput)
- `src-tauri/src/lib.rs` (execute_tool, start_agent_run, resolve_workspace_path)
- `src/main.tsx` (EditorView component and call site)

**What was not reviewed:**
- Codex app-server internals (external binary)
- Typst binary bundling/scripts
- SQLite chat session storage
- CSS styling specifics

## 🔴 Blocking Issues

**None.**

## 🟡 Should Fix

**1. Codex app-server tool registration mechanism is unclear**

The plan adds a `render_resume` tool handler in `execute_tool` (TODO #9) and mentions updating the base prompt to list available tools including `render_resume` (TODO #10). However, the plan does not verify or document how the Codex app-server learns about this new tool.

The existing tools (`read_file`, `write_file`, `run_command`, `search_web`) are handled by the Rust side, but the app-server must offer them to the agent first. If the app-server has hardcoded tool definitions, adding `render_resume` requires updating the app-server configuration or registration mechanism (which is external and not in this repo). If the app-server reads tools from the prompt or has a dynamic registration mechanism, the plan should document this.

**Recommendation:** Verify how the Codex app-server discovers available tools. If it requires explicit registration (environment variables, config file, or JSON-RPC tool list), add a TODO to update that registration. If the prompt mention is sufficient, document this assumption explicitly in the plan. Add a contingency note: "If the agent cannot call `render_resume` after implementation, verify the app-server tool registration mechanism."

**2. `execute_tool` signature change requires capturing `project_slug` before thread spawn**

TODO #8 says to update `execute_tool` to accept `project_slug: &str` and update both call sites inside `start_agent_run` to pass the cloned input project slug. However, the call sites are inside a `thread::spawn` closure (lines 2103 and 2121 in `lib.rs`), and `input` is consumed by the function before the thread spawn.

The plan does not explicitly mention cloning `input.project_slug` before the thread spawn and capturing it in the closure. Without this, the code will not compile because `input` is not accessible inside the thread.

**Recommendation:** Add a TODO before #8: "Clone `input.project_slug` before the `thread::spawn` closure in `start_agent_run` and capture it as `project_slug_for_tools` (or similar) for use in the `execute_tool` call sites."

**3. Path stripping logic in agent tool is ambiguous**

TODO #9 says the `render_resume` tool branch should "strip the trailing `/resume.md` into `job_path`". It is unclear whether this stripping is applied to:
- The original relative path string from the tool arguments (e.g., `"hunt_run/my-hunt/personalized-resume/job-stem/resume.md"` → `"hunt_run/my-hunt/personalized-resume/job-stem"`), or
- The resolved absolute path from `resolve_workspace_path`.

The `job_path` field in `ResumeInput` is a relative path, so the stripping should be on the original relative path string. The plan should clarify this to avoid implementation errors.

**Recommendation:** Update TODO #9 to explicitly state: "Strip the trailing `/resume.md` from the original relative `path` argument (not the resolved absolute path) to produce the `job_path` value for `ResumeInput`."

## 💡 Optional Suggestions

**1. Document skill keyword ordering and precedence**

The plan adds `/resume-builder-all` to the `SKILLS` array with specific keyword patterns (TODO #5). The existing `/fix-render` skill has broad patterns like `"render"` and `"pdf"`. The `matching_skill` function returns the first matching skill in array order.

While the plan's keyword sets are disjoint enough to avoid accidental triggering (e.g., "tailor all resumes" does not match `/fix-render`'s patterns), the plan does not specify the order in which `/resume-builder-all` should be added to the `SKILLS` array relative to `/fix-render`.

**Suggestion:** Add a note to TODO #5: "Add `/resume-builder-all` after `/fix-render` in the `SKILLS` array. The keyword sets are disjoint, but array order determines precedence if future keyword patterns overlap."

**2. Clarify `<job-file-stem>` definition in skill instructions**

TODO #4 says the skill writes tailored resumes to `hunt_run/<hunt-slug>/personalized-resume/<job-file-stem>/resume.md`. The term `<job-file-stem>` is not explicitly defined. It is implied to be the job file name without the `.md` extension (e.g., `001-software-engineer-acme` from `001-software-engineer-acme.md`), but the plan should state this explicitly.

**Suggestion:** Update TODO #4 to say: "... write one tailored `hunt_run/<hunt-slug>/personalized-resume/<job-file-stem>/resume.md` per job, where `<job-file-stem>` is the job file name without the `.md` extension (e.g., `001-software-engineer-acme`)."

**3. Consider error message clarity for path validation**

TODO #2 adds `validate_personalized_resume_job_path` to reject paths not containing `personalized-resume`. The plan does not specify the error message. A clear error message would help users and agents understand the restriction.

**Suggestion:** Add to TODO #2: "Return an error message like: 'PDF rendering is only allowed for resume.md files under personalized-resume/ folders. The provided path does not contain personalized-resume/.'"

## ✅ What Is Solid

**1. Path restriction enforcement in Rust**

The plan correctly identifies that the PDF render restriction must be enforced in Rust (`render_resume_pdf`), not only in the UI or agent prompt (see plan's "Key constraints/trade-offs" section). This is defense-in-depth and prevents bypass via direct Tauri command calls or agent tool misuse.

**2. Reuse of existing patterns and helpers**

The plan reuses:
- The existing skill registry (`SKILLS`, `matching_skill`, `skill_instructions`)
- The existing render pipeline (`validate_resume`, `render_resume_pdf`, `render_resume`, `ResumeInput`, `RenderOutput`)
- The existing agent tool bridge (`execute_tool`, `resolve_workspace_path`)
- The existing `EditorView` toolbar pattern

This minimizes new code and aligns with the codebase's current architecture.

**3. Frontend and backend validation alignment**

The plan adds `canRenderResume` derivation in the frontend (TODO #12) that mirrors the backend path validation (TODO #2-3). This ensures the UI only shows the render button for eligible files, and the backend rejects invalid paths even if the UI is bypassed.

**4. Skill keyword specificity**

The plan's keyword patterns for `/resume-builder-all` (TODO #5) are specific and unlikely to trigger accidentally: `"/resume-builder-all"`, `"resume-builder-all"`, `"resume builder all"`, `"build all resumes"`, `"tailor all resumes"`, `"personalized resumes for all jobs"`. These do not overlap with `/fix-render`'s patterns (`"render"`, `"pdf"`, `"typst"`, etc.).

**5. Clear scope boundaries**

The plan explicitly states what is out of scope: changing HuntBrief scraping, changing the Typst renderer layout, storing resume outputs in DB, auto-apply behavior, and new frontend panels. This prevents scope creep.

**6. Deterministic file-based workflow**

The plan keeps resume generation as file-based (Markdown files under `personalized-resume/`) and PDF generation deterministic through the existing Rust pipeline. This aligns with the project's principle of keeping product-critical flows deterministic in Rust/Tauri commands (per AGENTS.md).

## ⚖️ Verdict

**PASS: ready for implementation.**

The plan is grounded in the discovery handoff, deterministic enough for implementation, consistent with existing patterns, and clear about scope and constraints. The should-fix issues are minor implementation details that can be resolved during coding without requiring a plan revision. The optional suggestions are cosmetic improvements that do not affect correctness.

The plan correctly identifies the critical architectural decision (enforce path restriction in Rust, not just UI/prompt) and reuses existing helpers and patterns appropriately. The skill keyword patterns are specific enough to avoid accidental triggering, and the frontend/backend validation alignment provides defense-in-depth.

The implementation can proceed with the current plan. The should-fix issues can be addressed as the coding agent works through the TODOs.
