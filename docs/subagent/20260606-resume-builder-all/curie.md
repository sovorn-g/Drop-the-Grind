## Scope Investigated

- Agent skills system (slash commands, skill registry, keyword matching, prompt injection)
- Agent chat/Codex integration flow (frontend AgentPanel → Rust `start_agent_run` → Codex app-server JSON-RPC)
- Resume parsing, validation, Typst → PDF rendering pipeline
- Hunt run directory structure and job file layout
- Profile file conventions (RESUME.md, USER.md, RESUME_TEMPLATE.md)
- Agent tools available (`read_file`, `write_file`, `run_command`, `search_web`)
- EditorView UI component and PDF open behavior
- Tauri command registration

What was not investigated:
- Codex app-server internals (external binary, not in this repo)
- Typst binary bundling/scripts
- SQLite chat session storage details
- CSS styling specifics (not relevant to the task architecture)

## Findings

### 1. Skills Registry — `SKILLS` in `src-tauri/src/resume.rs#L88-L94`
- Finding: The skills system is a simple `const SKILLS: &[Skill]` array. Each Skill has `name`, `description`, and `keyword_patterns`. Currently contains only one skill: `/fix-render`.
- Relevance: The new `/resume-builder-all` skill must be added here with appropriate keyword patterns (e.g. `"/resume-builder-all"`, `"resume builder"`, `"tailor all resumes"`).

### 2. Skill Instructions — `skill_instructions` and `skill_registry_prompt` in `src-tauri/src/resume.rs#L97-L124`
- Finding: `skill_registry_prompt()` renders the list for the system prompt. `matching_skill(message)` checks keywords in the user message. `skill_instructions(name)` returns the full instruction text by name.
- Relevance: A new `SKILL_RESUME_BUILDER_ALL` const must be added with detailed instructions. The match arm in `skill_instructions` must include the new skill name. The prompt should instruct the agent to read `profile/RESUME.md` and `profile/USER.md`, iterate over jobs in the user-specified folder, and write tailored resumes to `personalized-resume/` inside the hunt folder.

### 3. Skill Prompt Injection — `start_agent_run` in `src-tauri/src/lib.rs#L1989-L2006`
- Finding: The system prompt already mentions `profile/resume_current.*` and `hunt_run/<name>/results.md`. Skill instructions are appended when `matching_skill` matches. The `linkedFilePath` is also injected if provided.
- Relevance: The `/resume-builder-all` skill instructions should tell the agent to ask the user which `jobs-YYYY-MM-DD/` folder to use, then read all `.md` files in that folder, combine with profile data, and write output.

### 4. Agent Tools — `execute_tool` in `src-tauri/src/lib.rs#L1942-L1973`
- Finding: Agent has `read_file`, `write_file`, `run_command` (whitelisted: pandoc, pdflatex, python3, node, git, grep, cat, etc.), and `search_web`. Paths are resolved via `resolve_workspace_path` which constrains to the project root.
- Relevance: The agent CANNOT directly call `render_resume_pdf` (that's a Tauri command, not a tool). The agent would need to write `resume.md` files, then instruct the user to trigger render from the UI, OR a new tool call could be added. For the "agent can render" requirement, either add a new tool or tell the user to click a render button.

### 5. PDF Rendering — `render_resume_pdf` in `src-tauri/src/resume.rs#L596-L675`
- Finding: Takes `ResumeInput { project_slug, job_path }`. Looks for `resume.md` at `project_root + job_path + "resume.md"`. Generates Typst, compiles to PDF at same location. No path restriction currently — any `resume.md` in any folder can be rendered.
- Relevance: Must be updated to restrict PDF generation to paths containing `personalized-resume/`. Validation check needed at the top of the function.

### 6. ResumeInput and Validate — `src-tauri/src/resume.rs#L65-L68`, `validate_resume` at L570
- Finding: `ResumeInput` has `project_slug` and `job_path`. Both `validate_resume` and `render_resume_pdf` use this. `render_resume` at L679 calls both as a pipeline.
- Relevance: The `job_path` field description says "relative path under hunt_run/<name>/jobs/<job-name>/" but it works for any relative path. For `personalized-resume/`, just pass `hunt_run/<slug>/personalized-resume/` as the path prefix.

### 7. No UI Render Button — `EditorView` in `src/main.tsx#L332-L349`
- Finding: EditorView has no "Render to PDF" button. PDF files are opened externally via `open_project_file`. Currently no UI calls `render_resume` or `render_resume_pdf`.
- Relevance: A "Render PDF" button must be added to the EditorView toolbar when the selected file is a `resume.md` inside a `personalized-resume/` folder. The button calls `render_resume` or `render_resume_pdf` Tauri command.

### 8. Tauri Command Registration — `run()` in `src-tauri/src/lib.rs#L2309`
- Finding: `resume::validate_resume`, `resume::render_resume_pdf`, `resume::render_resume`, and `resume::list_skills` are already registered.
- Relevance: No new Tauri commands need to be registered unless adding a new tool for the agent. The existing commands are sufficient if the UI button approach is used.

### 9. Hunt Run Structure — `create_hunt_run` in `src-tauri/src/lib.rs#L654` and `start_hunt_apify` at L1023
- Finding: Hunt runs create `hunt_run/<slug>/` with `results.md`, `.hunt_config.json`, `.hunt_result.json`. Jobs are stored as `jobs-YYYY-MM-DD/###-title-company.md`. Job detail files are generated by `job_detail_markdown` at L978.
- Relevance: The new `personalized-resume/` folder should be a sibling of `jobs-YYYY-MM-DD/` inside `hunt_run/<slug>/`. The agent skill instructions should tell the agent to write `personalized-resume/<original-job-file-name>-resume.md` or similar.

### 10. Profile Files — `create_project` in `src-tauri/src/lib.rs#L307-L322`
- Finding: On project creation, `profile/RESUME.md`, `profile/RESUME_TEMPLATE.md`, and `profile/USER.md` are scaffolded. Also `profile/resume_current.*` from uploads.
- Relevance: The skill instructions should reference `profile/RESUME.md` and `profile/USER.md` as the canonical profile sources.

### 11. Frontend Skills Menu — `AgentPanel` in `src/main.tsx#L350`
- Finding: The skills menu opens when typing `/` in the chat composer. It calls `list_skills` (mapped to `resume::list_skills`) and displays skill names. Selecting a skill inserts its name into the draft.
- Relevance: The new skill will automatically appear in the skills menu once added to `SKILLS` array and `list_skills`. No frontend changes needed for the menu itself.

## Relationships

- **Skills → Agent Prompt**: `SKILLS` array → `skill_registry_prompt()` → system prompt in `start_agent_run` → Codex app-server
- **Skill Keyword Match → Instruction Injection**: `matching_skill()` → `skill_instructions()` → appended to prompt in `start_agent_run`
- **Agent Tools → File System**: `execute_tool` → `resolve_workspace_path` → project-constrained read/write
- **PDF Render → Typst**: `render_resume_pdf` → `parse_resume` → `resume_to_typst` → `resolve_typst_binary` → Typst CLI
- **Frontend → Backend**: `EditorView` / future render button → `call('render_resume_pdf', ...)` → Rust tauri command
- **Hunt Run → Jobs**: `create_hunt_run` creates folder → `start_hunt_apify` writes `jobs-YYYY-MM-DD/*.md` → `personalized-resume/` goes alongside

## Open Questions / Gaps

1. **Agent render tool**: The agent currently cannot call `render_resume_pdf`. Options: (a) add a new tool to `execute_tool` that invokes the Rust render function, (b) have the skill instructions tell the agent to write `resume.md` and ask the user to click a render button, (c) add a `resume:render` tool. The task says "agent can mention/fix/update a particular resume file and render it" — this likely requires option (a) or (c).

2. **PDF render restriction granularity**: Should the restriction check for exactly `personalized-resume/resume.md` or any `resume.md` under a `personalized-resume/` path? The task says "only be allowed for `resume.md` files inside `personalized-resume/` folders" — likely any `resume.md` whose parent path contains `personalized-resume`.

3. **File naming convention**: Should tailored resumes be named `personalized-resume/<original-filename>.md`, `personalized-resume/<company>-<title>-resume.md`, or `personalized-resume/resume-<job-slug>.md`? The task says "preserving the same resume Markdown structure used by `profile/RESUME.md`" but doesn't specify exact naming.

4. **Manual render trigger UX**: Where exactly should the render button appear? In EditorView toolbar when path matches `personalized-resume/*/resume.md`? Or a right-click context menu? The task says "when editing a resume.md inside personalized-resume/, changes can be rendered to PDF into that same folder."

5. **Skill keyword matching scope**: The current `/fix-render` skill uses broad patterns like `"render"`, `"pdf"`, `"typst"`. The new `/resume-builder-all` needs keywords that won't accidentally trigger when the user is talking about the `/fix-render` skill. Needs careful pattern selection.

## Start Here

1. `src-tauri/src/resume.rs` — add `SKILL_RESUME_BUILDER_ALL` constant and new `Skill` entry in `SKILLS` array, update `skill_instructions` match arm.
2. `src-tauri/src/resume.rs#L596` — add path restriction check in `render_resume_pdf` for `personalized-resume/`.
3. `src/main.tsx#L332` — add Render PDF button in EditorView (`EditorView` component).
