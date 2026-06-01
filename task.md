# Drop the Grind — Agent Implementation Checklist

Work **phase by phase**. Do not skip ahead. At the end of each phase, stop and report what was built, tested, and still risky.

Reference plan: [`project.md`](./project.md)

## Core direction

- [x] Build a macOS desktop app for local-first job hunting workflow automation.
- [x] Use Tauri 2 + React + TypeScript + Vite.
- [x] Use Rust-owned SQLite and filesystem commands.
- [x] Use workspace root: `~/.dropthegrind/workspace`.
- [ ] Implement Hybrid Job Ops Workspace UI:
  - [ ] Left: mode nav + file tree.
  - [ ] Center: Setup/Sources/Job Inbox/Packets/Files/Settings views.
  - [ ] Right: local-only project chat UI.
- [ ] Implement Apify MCP milestone 1:
  - [ ] Generate MCP-ready config/task files.
  - [ ] Import produced JSON output manually.
  - [ ] Do **not** implement direct in-app MCP execution yet.
- [ ] Do **not** implement auto-apply behavior.
- [ ] Do **not** implement model/API execution in milestone 1.

---

# Phase 1 — App foundation and local workspace shell

## Goal

- [x] Create the desktop app foundation and prove safe local file/project operations.

## Build checklist

- [x] Initialize Tauri 2 + React + TypeScript + Vite app.
- [x] Add Tailwind CSS.
- [x] Add shadcn/ui foundation.
- [x] Create premium dark macOS-style app shell:
  - [x] Left panel: mode navigation + project file tree placeholder.
  - [x] Center panel: active view area.
  - [x] Right panel: local chat UI placeholder.
  - [x] Top toolbar with project switcher/settings affordance.
- [ ] Implement workspace/project creation under:

  ```txt
  ~/.dropthegrind/workspace/<project-slug>/
  ```

- [x] Generate default project folder/file template:

  ```txt
  profile/
    resume_original.pdf
    resume_extracted.md
    user_profile.md
    preferences.json
  sources/
    apify_sources.json
    apify_mcp_config.json
    run_apify_actor.md
    imports/
  jobs/
    normalized/
    ranked/
    approved/
  applications/
  chats/
    project-chat.md
  project.json
  ```

- [x] Implement Rust Tauri command: `create_project`.
- [x] Implement Rust Tauri command: `list_workspace_tree`.
- [x] Implement Rust Tauri command: `read_text_file`.
- [x] Implement Rust Tauri command: `write_text_file` with atomic writes.
- [x] Enforce filesystem safety:
  - [x] Canonicalize paths.
  - [x] Reject path traversal.
  - [x] Reject writes outside workspace.
  - [x] Treat PDF/binary files as read-only.
- [x] Add center file editor/viewer for:
  - [x] Markdown.
  - [x] JSON.
  - [x] Plain text.
  - [x] LaTeX.
- [x] Add first-run empty state.
- [x] Add project setup checklist.

## Acceptance checklist

- [ ] App launches locally.
- [ ] User can create a project under `~/.dropthegrind/workspace`.
- [ ] File tree shows generated folders/files.
- [ ] User can open text files.
- [ ] User can edit text files.
- [ ] User can save text files.
- [ ] User can reload saved text files.
- [ ] Binary/PDF files are not editable.
- [ ] `Cmd+S` saves the active file.
- [ ] Unsafe paths are rejected in Rust command tests.

## Test checklist

- [x] Add Rust tests for path containment.
- [x] Add Rust tests for traversal rejection.
- [ ] Add Rust/Tauri command tests for create/list/read/write.
- [ ] Run frontend smoke/manual QA: create project → edit file → save/reload.

## Stop point

- [ ] Stop after Phase 1.
- [ ] Report files created.
- [ ] Report commands implemented.
- [ ] Report tests run.
- [ ] Report known issues.

---

# Phase 2 — Apify MCP source workflow, job import, and packet generation

## Goal

- [ ] Implement the core job-hunt loop without direct model or MCP execution.

## Build checklist

- [ ] Add Rust-owned SQLite persistence.
- [ ] Create initial table: `projects`.
- [ ] Create initial table: `source_configs`.
- [ ] Create initial table: `imports`.
- [ ] Create initial table: `jobs`.
- [ ] Create initial table: `application_packets`.
- [ ] Create initial table: `chat_sessions`.
- [ ] Create initial table: `chat_messages`.
- [ ] Add migration/version handling.
- [ ] Add `project.json` schema version.
- [ ] Implement Sources view.
- [ ] Add actor name field.
- [ ] Add MCP server URL field defaulting to `https://mcp.apify.com`.
- [ ] Add input template JSON editing/opening flow.
- [ ] Add button to generate `sources/apify_mcp_config.json`.
- [ ] Add button to generate `sources/run_apify_actor.md`.
- [ ] Add button to generate `sources/apify_actor_input.json`.
- [ ] Add import output JSON flow from `sources/imports/` or selected file.
- [ ] Implement Apify/MCP JSON import:
  - [ ] Save raw import file.
  - [ ] Parse JSON arrays.
  - [ ] Parse wrapped dataset output.
  - [ ] Normalize into `NormalizedJob`.
  - [ ] Support common field aliases.
  - [ ] Validate required fields: title, company, applyUrl/sourceUrl.
  - [ ] Record skipped rows with reasons.
- [ ] Implement dedupe:
  - [ ] Generate deterministic dedupe key from company/title/applyUrl/sourceUrl.
  - [ ] Add unique index by project + dedupe key.
- [ ] Implement Job Inbox view:
  - [ ] List imported jobs.
  - [ ] Add status filters.
  - [ ] Add search basics.
  - [ ] Add sort basics.
  - [ ] Add approve action.
  - [ ] Add reject action.
  - [ ] Add source badge.
  - [ ] Add location/remote badge.
  - [ ] Add salary badge when present.
- [ ] Implement application packet generation:

  ```txt
  applications/{company-slug}-{title-slug}-{short-job-id}/
  ├── input/
  │   ├── resume_original.pdf
  │   ├── resume_extracted.md
  │   ├── job_posting.json
  │   └── user_preferences.json
  ├── tasks/
  │   ├── tailor_resume.md
  │   ├── verify_resume.md
  │   └── generate_outreach.md
  ├── templates/
  │   └── resume_template.tex
  └── output/
  ```

- [ ] Make packet generation idempotent:
  - [ ] Do not create duplicate packet for same job.
  - [ ] Never overwrite existing output files silently.
  - [ ] Open/reuse existing packet when present.
- [ ] Implement Packets view:
  - [ ] List generated packets.
  - [ ] Open packet.
  - [ ] Reveal in Finder if feasible.
  - [ ] Open task files in center editor.

## Acceptance checklist

- [ ] User can configure an Apify actor/source for a project.
- [ ] App generates MCP config/task/input files.
- [ ] User can place/import sample Apify output JSON.
- [ ] Jobs appear in Job Inbox.
- [ ] Duplicate imports do not create duplicate jobs.
- [ ] User can approve a job.
- [ ] User can generate an application packet.
- [ ] Re-generating a packet for the same job reuses existing packet safely.

## Test checklist

- [ ] Add unit tests for normalization.
- [ ] Add unit tests for field aliases.
- [ ] Add unit tests for dedupe key.
- [ ] Add unit tests for slug generation.
- [ ] Add integration test: import JSON → jobs → approve → packet.
- [ ] Test malformed JSON.
- [ ] Test partial success.
- [ ] Test skipped rows.
- [ ] Test duplicate imports.

## Stop point

- [ ] Stop after Phase 2.
- [ ] Report schema/migrations.
- [ ] Report sample fixture used.
- [ ] Report import results.
- [ ] Report packet generation behavior.
- [ ] Report tests run.
- [ ] Report remaining risks.

---

# Phase 3 — Local chat UI, polish, accessibility, and portfolio readiness

## Goal

- [ ] Make the app coherent, trustworthy, and demo-ready while keeping execution local-only.

## Build checklist

- [ ] Implement right-panel local chat UI:
  - [ ] Message list.
  - [ ] Composer.
  - [ ] Local persistence in SQLite.
  - [ ] Current file context chip.
  - [ ] Selected job context chip.
  - [ ] Selected packet context chip.
  - [ ] Clear local-only empty state copy.
- [ ] Add chat action: create task file from chat content.
- [ ] Add chat action: append to current task file.
- [ ] Add optional chat transcript mirror/export to `chats/project-chat.md`.
- [ ] Improve interaction states:
  - [ ] No project.
  - [ ] Loading project.
  - [ ] Empty source state.
  - [ ] Empty job state.
  - [ ] Empty packet state.
  - [ ] Malformed import error.
  - [ ] Partial import success.
  - [ ] Permission denied.
  - [ ] Packet already exists.
  - [ ] Save failure.
- [ ] Polish visual design:
  - [ ] Premium dark macOS style.
  - [ ] Compact panel density.
  - [ ] Clear focus rings.
  - [ ] Status badges/pills.
  - [ ] Subtle dividers.
  - [ ] Rounded cards.
- [ ] Add accessibility basics:
  - [ ] Keyboard navigation for mode nav.
  - [ ] Keyboard navigation for file tree.
  - [ ] Keyboard navigation for job list.
  - [ ] Retain `Cmd+S` support.
  - [ ] Visible focus states.
  - [ ] Accessible labels for chat composer.
  - [ ] Accessible labels for primary actions.
  - [ ] Do not rely on color alone for statuses.
- [ ] Add Settings view:
  - [ ] Workspace path display.
  - [ ] Apify MCP settings placeholder.
  - [ ] Codex/opencode/API provider placeholders marked as future.
- [ ] Add README with:
  - [ ] Product pitch.
  - [ ] Architecture overview.
  - [ ] Local-first/privacy notes.
  - [ ] Screenshots/GIF placeholders.
  - [ ] Development instructions.
- [ ] Add sample Apify output fixture(s).
- [ ] Document how to run the demo flow.

## Acceptance checklist

- [ ] Right chat panel is usable.
- [ ] Right chat panel is clearly local-only.
- [ ] User can create task markdown from chat content.
- [ ] User can update task markdown from chat content.
- [ ] Main empty/error/success states exist.
- [ ] UI looks portfolio-ready in dark mode.
- [ ] Keyboard/focus basics work.
- [ ] README explains the project.
- [ ] README explains the demo path.
- [ ] Full demo flow works:

  ```txt
  create project → generate Apify MCP task/config → import sample JSON → review jobs → generate packet → edit task → use chat to update task
  ```

## Test checklist

- [ ] Add chat persistence tests.
- [ ] Add task creation from chat tests.
- [ ] Complete manual accessibility/keyboard checklist.
- [ ] Complete end-to-end smoke test of full demo flow.

## Stop point

- [ ] Stop after Phase 3.
- [ ] Report final demo flow.
- [ ] Report screenshots/GIF paths if created.
- [ ] Report tests run.
- [ ] Report remaining product/technical risks.
- [ ] Report recommended next phase.

---

# Explicit non-goals for these 3 phases

- [ ] Do not implement direct Apify MCP execution inside the app.
- [ ] Do not implement direct Codex/opencode execution inside the app.
- [ ] Do not implement API model providers.
- [ ] Do not implement auto-apply.
- [ ] Do not implement custom Greenhouse/Lever/Ashby scrapers.
- [ ] Do not implement full resume builder.
- [ ] Do not implement full PDF rendering pipeline.
- [ ] Do not implement complex onboarding wizard.

---

# Agent working rules

- [ ] Work phase by phase and stop after each phase.
- [ ] Keep generated files transparent and editable.
- [ ] Prefer simple, testable vertical slices over broad unfinished systems.
- [ ] Preserve local-first privacy and user control.
- [ ] Do not silently overwrite user-created workspace artifacts.
- [ ] Update this task file or create follow-up task files if scope changes.
