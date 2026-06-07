# Plan – Resume Builder All

## What

- Add a `/resume-builder-all` agent skill that generates tailored resume Markdown files for every job in a selected hunt `jobs-YYYY-MM-DD/` folder.
- Restrict PDF rendering to `resume.md` files under `personalized-resume/` folders.
- Let both the UI and the agent render an eligible personalized `resume.md` to `resume.pdf` in the same folder.

## How

- High-level approach: reuse the existing Rust skill registry, agent tool bridge, resume validation/render pipeline, and `EditorView` toolbar. Keep resume generation as an agent skill that writes Markdown files; keep PDF generation deterministic through the existing Rust `render_resume` command.
- **Scope**:
  - In scope: `/resume-builder-all` skill registration/instructions, render-path restriction, agent `render_resume` tool, and a Render PDF button for eligible files.
  - Out of scope: changing HuntBrief scraping, changing the Typst resume renderer layout, storing resume outputs in DB, auto-apply behavior, and new frontend panels.
  - Scope assumptions: tailored outputs are file-based and live under the hunt run, not global profile files.
- **Assumptions**:
  - Each generated resume uses `hunt_run/<hunt-slug>/personalized-resume/<job-file-stem>/resume.md` so the existing renderer can output `resume.pdf` beside it.
  - `/resume-builder-all` asks for the target `jobs-YYYY-MM-DD/` folder when the user does not specify one or when it cannot infer one from the selected file/path.
  - The skill renders PDFs only when the user explicitly asks to render PDFs, or when fixing/rendering a named personalized resume file; otherwise it writes Markdown and reports the generated paths.
- **Reuses**:
  - `SKILLS`, `matching_skill`, `skill_instructions`, `skill_registry_prompt` from `src-tauri/src/resume.rs`.
  - `validate_resume`, `render_resume_pdf`, `render_resume`, `ResumeInput`, `RenderOutput` from `src-tauri/src/resume.rs`.
  - `execute_tool`, `resolve_workspace_path`, `start_agent_run` from `src-tauri/src/lib.rs`.
  - `EditorView`, `editor-actions`, `open_project_file`, `refreshTree`, `saveFile` patterns from `src/main.tsx`.
- Key constraints/trade-offs: render eligibility must be enforced in Rust, not only in the UI or prompt. Keep skill keyword patterns specific so ordinary “resume” messages do not accidentally trigger `/resume-builder-all` or override `/fix-render`.

## TODO

1. Update `src-tauri/src/resume.rs` `ResumeInput` comment to describe `job_path` as a workspace-relative directory containing `resume.md`, with PDF rendering restricted to paths under `personalized-resume/`.
2. Add `src-tauri/src/resume.rs` private helper `validate_personalized_resume_job_path(job_path: &str) -> Result<(), String>` that rejects empty paths, absolute paths, `..` components, and paths whose normalized components do not include `personalized-resume`.
3. Update `src-tauri/src/resume.rs` `render_resume_pdf` to call `validate_personalized_resume_job_path(&input.job_path)` before deriving `job_dir`, `resume_path`, or `pdf_path`.
4. Add `src-tauri/src/resume.rs` const `SKILL_RESUME_BUILDER_ALL` with instructions to read `profile/RESUME.md` and `profile/USER.md`, identify the requested `hunt_run/<hunt-slug>/jobs-YYYY-MM-DD/` folder, read every `.md` job file in it, and write one tailored `hunt_run/<hunt-slug>/personalized-resume/<job-file-stem>/resume.md` per job while preserving the schema accepted by `parse_resume`.
5. Add `src-tauri/src/resume.rs` `SKILLS` entry for `/resume-builder-all` with specific keyword patterns: `"/resume-builder-all"`, `"resume-builder-all"`, `"resume builder all"`, `"build all resumes"`, `"tailor all resumes"`, and `"personalized resumes for all jobs"`.
6. Update `src-tauri/src/resume.rs` `skill_instructions` to return `SKILL_RESUME_BUILDER_ALL` for `"/resume-builder-all"`.
7. Update `src-tauri/src/resume.rs` `SKILL_FIX_RENDER` references from `profile/resume.md` to `profile/RESUME.md`, and state that renderable files must be `personalized-resume/.../resume.md`.
8. Update `src-tauri/src/lib.rs` `execute_tool` signature to accept `project_slug: &str` in addition to `project_root`, and update both `execute_tool` call sites inside `start_agent_run` to pass the cloned input project slug.
9. Add `src-tauri/src/lib.rs` `execute_tool` branch for `"render_resume" | "render_resume_pdf" | "render_pdf"` that requires `path` or `file` to be a workspace-relative `.../personalized-resume/.../resume.md`, validates it with `resolve_workspace_path`, strips the trailing `/resume.md` into `job_path`, calls `resume::render_resume(ResumeInput { project_slug: project_slug.to_string(), job_path })`, and returns the generated PDF path.
10. Update `src-tauri/src/lib.rs` `start_agent_run` base prompt to mention the available local tools by name, including `render_resume` for eligible personalized resume files.
11. Update `src/main.tsx` `EditorView` props to accept `setStatus`, `debugLog`, and `refreshTree`, and update its call site to pass the existing functions.
12. Add `src/main.tsx` `EditorView` derived values `canRenderResume`, `resumeJobPath`, and `resumePdfPath` where `canRenderResume` is true only for `selectedPath` ending in `/resume.md` and containing `/personalized-resume/`.
13. Add `src/main.tsx` `EditorView` async `renderPdf` handler that saves dirty editable content via `saveFile()`, invokes `render_resume` with `{ projectSlug: project.slug, jobPath: resumeJobPath }`, refreshes the file tree, opens `resumePdfPath` via `open_project_file`, and reports success/failure through `setStatus` and `debugLog`.
14. Add `src/main.tsx` Render PDF toolbar button inside `.editor-actions` only when `canRenderResume` is true; label it `Save & Render PDF` when `dirty` is true and `Render PDF` otherwise.

## Outcome

- `/resume-builder-all` appears in the slash-command skills menu and injects deterministic instructions for generating one personalized `resume.md` per job file.
- PDF generation fails from Rust for any `job_path` outside `personalized-resume/`.
- The agent can render an eligible personalized `resume.md` using the new `render_resume` tool.
- The editor can save and render an eligible personalized `resume.md`, then open the resulting `resume.pdf` in the system PDF viewer.
- Existing HuntBrief output, profile files, and non-personalized Markdown files remain unchanged.