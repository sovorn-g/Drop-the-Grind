# Drop the Grind — Desktop Job Hunt Workspace Design

Date: 2026-06-01
Repo: `/home/drpain/github/Drop-the-Grind`
Branch: `main`
Status: DRAFT
Mode: Builder mode

## Problem statement

Job hunting is a numbers game, but the current workflow is too manual:

1. Search across many job-listing sites.
2. Open promising roles one by one.
3. Copy job descriptions and company context into an LLM.
4. Compare each role against an existing resume, personal info, and preferences.
5. Tailor the foundation resume and generate outreach/application materials.

Drop the Grind should reduce that repetitive work while keeping the user in control. It is not a resume builder and not an auto-apply bot. The existing resume remains the source of truth. The app helps collect jobs, rank them, generate application packet workspaces, and let AI execution tools such as Codex/opencode/API models operate on local files.

## Builder-mode goal

Primary goal: make the app useful for the creator's own job search.

Secondary goal: create a portfolio-quality macOS desktop app that impresses recruiters by showing:

- TypeScript-heavy application engineering.
- Rust/Tauri native desktop integration.
- Local-first workspace design.
- Adapter-based ingestion architecture.
- AI-agent execution abstraction without requiring model API keys at first.
- Human-in-the-loop job review and truthful application prep.

## Product direction chosen

The selected direction is a macOS desktop app with an IDE-like layout:

- Left panel: workspace folder/file tree.
- Center panel: file view/edit area.
- Right panel: chat session.
- Settings area: Codex/opencode/subagent auth/config and Apify worker configuration.
- Workspace model: each new project creates predefined folders and files under an app workspace directory, avoiding heavy onboarding questions at project creation.

Initial ingestion should be Apify-only, because Apify actors can cover many general and site-specific job boards without building every scraper immediately.

## Visual design direction

The app should use a polished macOS-native dark workspace aesthetic. Ignore any specific reference layout details; the relevant direction is the surface treatment, tone, and interaction feel.

Target aesthetic:

- **Premium dark-mode desktop tool:** deep charcoal surfaces, not pure black, with subtle contrast between app chrome, panels, cards, and editor areas.
- **macOS-native window feel:** rounded outer window corners, traffic-light controls, thin panel dividers, compact toolbar density, and restrained native-app affordances.
- **Frosted/translucent sidebar treatment:** left navigation can feel slightly glassy or blurred, with ambient wallpaper/color bleed where appropriate, while keeping text readable.
- **Soft ambient gradients:** use restrained blue/purple/green glow accents and background depth rather than flat black panels or loud gradients.
- **Compact pro-tool density:** small typography, tight but comfortable spacing, and information-dense controls similar to Linear, Raycast, Arc, CleanShot, or modern AI coding clients.
- **Rounded cards and pills:** use rounded metric/job cards, status pills, small badges, and compact buttons with subtle borders/shadows instead of heavy elevation.
- **Muted neutral palette with precise accents:** mostly grays and off-whites, with selective blue for active selections, green/yellow for job/status signals, and occasional magenta/purple brand accent.
- **Low-noise hierarchy:** avoid oversized hero UI. Emphasis should come from spacing, typography weight, active states, pills, and accent dots/rings.
- **Calm local-first workspace mood:** the UI should feel like a serious productivity dashboard/IDE for job operations, not a generic SaaS web app or playful resume builder.

Design implication: the first milestone can be simple, but it should already feel like a premium macOS utility. Use subtle borders, glassy sidebars, dark cards, compact controls, and carefully placed accent color so screenshots look portfolio-ready.

## Core premises

1. The right first problem is throughput and consistency, not fully autonomous applying.
2. If nothing is built, the workflow remains manual: find jobs, copy job/company info into an LLM, tailor repeatedly.
3. Existing patterns help: local-first desktop app, normalized job schema, Apify scraping layer, SQLite queue, packet folders, and later pluggable AI execution adapters.
4. The output artifact is the main product loop: each approved job becomes a local application packet folder that Codex/opencode/API models can work on.
5. Initial source scope should be Apify-only, with custom adapters later.
6. The app should feel like a focused job-hunt workspace, not a generic resume builder.

## Recommended app structure

### Workspace location

Proposed default:

```txt
~/.dropthegrind/workspace/
```

Alternative macOS-native option:

```txt
~/Library/Application Support/DropTheGrind/workspace/
```

The user requested an `.ourappname/workspace` style path. The exact folder name is still open, but the product should treat workspace files as first-class user artifacts.

### Project structure

Each new project should create predefined folders/files so the user can start quickly without answering many onboarding questions.

Example:

```txt
~/.dropthegrind/workspace/my-2026-job-search/
├── profile/
│   ├── resume_original.pdf
│   ├── resume_extracted.md
│   ├── user_profile.md
│   └── preferences.json
├── sources/
│   ├── apify_sources.json
│   └── imports/
├── jobs/
│   ├── normalized/
│   ├── ranked/
│   └── approved/
├── applications/
│   └── acme-ai-engineer/
│       ├── input/
│       │   ├── resume_original.pdf
│       │   ├── resume_extracted.md
│       │   ├── job_posting.json
│       │   └── user_preferences.json
│       ├── tasks/
│       │   ├── tailor_resume.md
│       │   ├── verify_resume.md
│       │   └── generate_outreach.md
│       ├── templates/
│       │   └── resume_template.tex
│       └── output/
│           ├── resume_tailored.tex
│           ├── resume_tailored.pdf
│           ├── outreach_email.md
│           └── verification_report.md
├── chats/
│   └── project-chat.md
└── project.json
```

## UI layout

The chosen design direction is a **Hybrid Job Ops Workspace**: a three-panel macOS desktop layout where primary job workflow modes are visible alongside the local file tree. The app should not feel like only a file browser; `Sources`, `Job Inbox`, and `Packets` are first-class modes.

### Left panel — mode nav + folder/file tree

Purpose:

- Show primary modes: Setup, Sources, Job Inbox, Packets, Files, Settings.
- Show the active project workspace.
- Let users navigate generated files, sources, jobs, application packets, task prompts, outputs, and chat logs.

Initial features:

- Create/open project.
- Show predefined workspace tree.
- Open files in center editor.
- Refresh file tree.
- Later: drag/drop resume, import Apify exports, reveal in Finder.

### Center panel — file view/edit

Purpose:

- Lightweight editor/viewer for project files.
- Make generated task prompts and application artifacts transparent and editable.

Initial supported file types:

- Markdown.
- JSON.
- Plain text.
- LaTeX.
- PDF preview can come later.

Initial features:

- Open file from tree.
- Edit and save text files.
- Read-only mode for binary/PDF files.
- Basic dirty-state indicator.

### Right panel — chat session

Purpose:

- Provide a project-aware chat surface.
- Eventually connect to Codex/opencode/API models.
- For MVP, can start as a local chat/task panel that records instructions and generated prompts.

Initial behavior:

- Show current project context.
- Let user draft/refine task instructions.
- Create task files inside application packet folders.
- Later: run Codex/opencode/API execution adapters and stream results.

### Settings

Settings should include:

- Apify config:
  - API token.
  - Actor IDs.
  - Default actor input templates.
  - Result dataset mapping.
- AI execution config:
  - Codex auth/connect settings.
  - opencode/subagent settings later.
  - API model provider keys later.
- Workspace config:
  - Default workspace path.
  - LaTeX/tectonic path.
  - File watcher settings.

## Data model

Initial normalized job schema:

```ts
type NormalizedJob = {
  title: string;
  company: string;
  location?: string;
  remoteType?: "remote" | "hybrid" | "onsite" | "unknown";
  description?: string;
  requirements?: string[];
  salaryRange?: string;
  applyUrl: string;
  sourceUrl: string;
  sourceType: string;
};
```

Additional internal fields likely needed:

```ts
type JobRecord = NormalizedJob & {
  id: string;
  projectId: string;
  sourceRunId?: string;
  dedupeKey: string;
  status: "new" | "shortlisted" | "approved" | "rejected" | "packet_created" | "applied";
  fitScore?: number;
  fitExplanation?: string;
  createdAt: string;
  updatedAt: string;
};
```

## Tech stack

Desktop:

- Tauri 2.
- Rust commands for filesystem, project creation, secure storage, native dialogs, file watching, process execution.

Frontend:

- React + TypeScript + Vite.
- Tailwind CSS + shadcn/ui.
- Monaco or CodeMirror for center editor later; textarea/simple editor acceptable for first milestone.

Local state:

- SQLite + Drizzle.
- Workspace files remain source-visible artifacts.

Scraping / ingestion:

- Apify-first.
- Source adapter interface retained for later expansion.

PDF / resume:

- LaTeX templates.
- tectonic for PDF rendering later.

AI execution:

- Initial: task/prompt bundle generator for Codex.
- Soon after: Codex execution adapter if feasible.
- Later: opencode/subagent execution adapter.
- Later: API model providers.

## Approaches considered

### Approach A — Minimal viable “Apify Inbox”

Build only ingestion, ranking, review, and packet generation.

Pros:

- Fastest useful version.
- Lower complexity.
- Good for the user's immediate job hunt.

Cons:

- Less distinctive UI.
- Less portfolio impact than a full desktop workspace.

### Approach B — Local Job Ops Workspace

Build the Tauri desktop app around a workspace/file-tree/editor/chat layout, with Apify ingestion and packet generation as the first real workflow.

Pros:

- Matches the desired macOS desktop app direction.
- Strong recruiter demo.
- Establishes architecture for Codex/opencode/API execution later.
- Makes files and generated artifacts transparent.

Cons:

- More upfront architecture.
- Risk of spending too long on shell/UI before job ingestion works.

### Approach C — CLI-first packet factory

Build the TypeScript core as a CLI first, then wrap it in Tauri later.

Pros:

- Fastest engine validation.
- Reusable core.

Cons:

- Not aligned with desired desktop-app portfolio outcome.
- Less satisfying to use daily.

## Recommended approach

Choose Approach B, but execute it in Approach A-sized milestones.

The app should be architected as the Local Job Ops Workspace, but the first working slice should be narrow:

1. Create/open project workspace.
2. Show file tree.
3. View/edit files.
4. Configure Apify settings.
5. Import Apify actor result JSON or dataset output.
6. Normalize jobs.
7. Generate application packet folders for approved jobs.
8. Generate Codex/opencode-ready task files.

Do not build custom scrapers, full PDF rendering, or API model execution until the local workspace and Apify packet loop works.

## First milestone scope

### Must have

- Tauri 2 + React + TypeScript + Vite app shell.
- Three-panel layout:
  - file tree left,
  - editor center,
  - chat/task panel right.
- Project creation under workspace directory.
- Predefined project folder/file templates.
- File read/write through Tauri commands.
- Settings screen or settings file for Apify and Codex/opencode placeholders.
- Apify JSON import path.
- Job normalization into `NormalizedJob`.
- Application packet generation for selected/approved jobs.
- Task markdown generation:
  - `tailor_resume.md`
  - `verify_resume.md`
  - `generate_outreach.md`

### Should have

- SQLite schema for projects, files, sources, jobs, application packets.
- Basic dedupe key.
- Basic deterministic filters from `preferences.json`.
- Simple fit score placeholder with explanation generated from rules, not LLM.
- Reveal packet in Finder.

### Not yet

- Custom Greenhouse/Lever/Ashby scrapers.
- Full autonomous applying.
- API model integrations.
- Complex onboarding wizard.
- Full resume builder.
- Perfect rich editor.
- Production-grade chat execution.

## Success criteria

Personal-use success:

- The user can create a job-search project in under one minute.
- The user can import Apify job results and see normalized jobs.
- The user can approve a job and get a complete local application packet.
- The packet is immediately usable with Codex/opencode/manual LLM workflow.
- The app reduces repeated copy/paste and folder setup work.

Portfolio success:

- Recruiters can understand the product in a short demo.
- The repo shows clear TypeScript/Rust boundaries.
- The architecture has clean adapters for sources and AI execution.
- The UI looks like a polished local-first desktop tool, not a toy.
- The product story is specific: human-reviewed job discovery and application prep, not generic AI resume generation.

## Distribution plan

Initial distribution is personal use from local dev builds.

Portfolio distribution:

- GitHub repo with README, architecture diagram, screenshots/GIF.
- Short demo video showing:
  1. create project,
  2. import Apify results,
  3. review/rank jobs,
  4. generate packet,
  5. run/use generated task prompts.
- Resume bullet framing:
  - Built a local-first macOS job application workspace using Tauri, React, TypeScript, Rust, SQLite, and Apify integrations.
  - Designed adapter-based job ingestion and AI-execution abstraction for Codex/opencode/API model workflows.

## Dependencies / blockers

- Decide exact workspace folder name:
  - `~/.dropthegrind/workspace`
  - or macOS app support directory.
- Decide editor library timing: simple editor first vs CodeMirror/Monaco.
- Decide whether Apify first version imports JSON manually or calls Apify API directly.
- Decide how much Codex integration is feasible initially versus generating task files only.
- Tauri permissions and filesystem scope must be configured carefully.

## Open questions

Resolved by engineering review:

1. Hidden app folder: `~/.dropthegrind/workspace`.
2. First Apify integration: generate MCP-ready config/task files, then import produced JSON output. Direct in-app MCP execution later.
3. Right panel: real local chat UI in milestone 1, without model execution.
4. Project settings: split between SQLite workflow state and `project.json` portable manifest.
5. Resume extraction: user manually provides `resume_extracted.md` first.

Still open:

1. Exact first Apify actor(s) to test with.
2. Exact first sample job output fixture shape.
3. Whether chat transcripts are mirrored to markdown by default or only exported on demand.


## Engineering review decisions — 2026-06-01

### Decisions made

1. **Workspace path:** use `~/.dropthegrind/workspace`.
2. **SQLite ownership:** Rust owns SQLite behind Tauri commands.
   - Frontend TypeScript does not access SQLite directly.
   - Rust command layer is the persistence boundary.
   - TypeScript owns UI/domain presentation, adapter normalization helpers where useful, and task/template generation UI flows.
3. **Apify integration scope:** milestone 1 generates **MCP-ready config/task files** for Codex/opencode or another external agent to run Apify, then the app imports the resulting JSON/dataset output.
   - Apify MCP docs: `https://docs.apify.com/platform/integrations/mcp`
   - Use hosted server config shape such as `https://mcp.apify.com`.
   - Project settings store actor names and input JSON templates.
   - Direct in-app MCP client execution is deferred.
4. **Right panel:** implement a real chat UI in milestone 1, but without model execution.
   - Chat messages are local project artifacts/state.
   - Chat can generate/save task prompts and reference current files/jobs.
   - Actual Codex/opencode/API execution is deferred.
5. **Resume extraction:** user manually provides `resume_extracted.md` in milestone 1.

### Source-of-truth rules

Use three layers deliberately:

1. **SQLite = local index and workflow state**
   - project records
   - source/import records
   - normalized jobs
   - job status
   - packet records
   - chat session/message metadata
   - generated task status

2. **Workspace files = user-visible artifacts**
   - original resume
   - extracted resume markdown
   - preferences/profile files
   - raw Apify/MCP imports
   - generated application packets
   - generated task markdown
   - LaTeX/output files
   - exported chat transcripts if desired

3. **`project.json` = portable manifest only**
   - project id/name
   - project schema version
   - created/updated timestamps
   - relative folder conventions
   - selected Apify actor names/input templates
   - no high-churn job queue state

If SQLite and files drift, SQLite can be rebuilt by scanning workspace files plus `project.json`, but generated packet contents are never silently overwritten.

### Tauri/Rust command boundary

Rust owns native operations:

```ts
type TauriCommands = {
  createProject(input: CreateProjectInput): Promise<Project>;
  listWorkspaceTree(input: { projectId: string; path?: string }): Promise<FileTreeNode[]>;
  readTextFile(input: { projectId: string; path: string }): Promise<{ content: string; version: string }>;
  writeTextFile(input: { projectId: string; path: string; content: string; expectedVersion?: string }): Promise<void>;
  importApifyJson(input: { projectId: string; filePath: string; sourceConfigId?: string }): Promise<ImportResult>;
  listJobs(input: { projectId: string; status?: JobStatus }): Promise<JobRecord[]>;
  updateJobStatus(input: { projectId: string; jobId: string; status: JobStatus }): Promise<JobRecord>;
  generateApplicationPacket(input: { projectId: string; jobId: string }): Promise<ApplicationPacket>;
  saveChatMessage(input: SaveChatMessageInput): Promise<ChatMessage>;
};
```

All command inputs must be validated in Rust. Frontend paths are logical project-relative paths, not arbitrary absolute paths, except for explicit user-selected import files from native dialogs.

### Filesystem safety rules

- All workspace operations are constrained under `~/.dropthegrind/workspace`.
- Canonicalize paths before read/write.
- Reject `../` traversal and absolute paths unless produced by an approved native file picker/import command.
- Reject or carefully resolve symlinks that escape the workspace.
- Use atomic writes for edited text files: write temp file, flush, then rename.
- Treat PDFs and unknown binary files as read-only.
- Generated packet creation must be idempotent and must not overwrite existing user-edited outputs.

### Initial SQLite schema outline

Core tables:

```txt
projects(id, name, slug, root_path, schema_version, created_at, updated_at)
source_configs(id, project_id, type, name, actor_name, mcp_server_url, input_template_json, created_at, updated_at)
imports(id, project_id, source_config_id, raw_file_path, item_count, status, error_message, created_at)
jobs(id, project_id, import_id, title, company, location, remote_type, description, requirements_json, salary_range, apply_url, source_url, source_type, dedupe_key, status, fit_score, fit_explanation, created_at, updated_at)
application_packets(id, project_id, job_id, relative_path, status, created_at, updated_at)
chat_sessions(id, project_id, title, created_at, updated_at)
chat_messages(id, session_id, role, content, linked_file_path, linked_job_id, created_at)
```

Add unique indexes:

```txt
projects.slug
jobs(project_id, dedupe_key)
application_packets(project_id, job_id)
```

### Adapter and normalization contracts

Keep the adapter contract even while Apify is the only initial source:

```ts
type NormalizedJob = {
  title: string;
  company: string;
  location?: string;
  remoteType?: "remote" | "hybrid" | "onsite" | "unknown";
  description?: string;
  requirements?: string[];
  salaryRange?: string;
  applyUrl: string;
  sourceUrl: string;
  sourceType: string;
};

interface SourceAdapter<TInput = unknown> {
  sourceType: string;
  normalize(input: TInput): NormalizedJob[];
}
```

Apify/MCP import shape:

```ts
type ApifyMcpProjectSource = {
  name: string;
  actorName: string; // e.g. apify/rag-web-browser or a job-board actor
  mcpServerUrl: "https://mcp.apify.com";
  inputTemplate: Record<string, unknown>;
};

type ApifyImport = {
  actorName?: string;
  datasetId?: string;
  importedAt: string;
  rawItems: unknown[];
};
```

The normalizer should support field aliases because different Apify actors return different output shapes.

### Apify MCP milestone 1 behavior

Milestone 1 does not run Apify MCP directly in-app. Instead, for each project/source the app can generate files such as:

```txt
sources/apify_mcp_config.json
sources/run_apify_actor.md
sources/imports/<timestamp>-raw.json
```

Example MCP config artifact:

```json
{
  "mcpServers": {
    "apify": {
      "url": "https://mcp.apify.com"
    }
  }
}
```

Example task instruction artifact:

```md
Use the Apify MCP server to run actor `<actorName>` with the input in `sources/apify_actor_input.json`.
Save the returned dataset items as JSON to `sources/imports/<timestamp>-raw.json`.
Do not tailor resumes or apply to jobs. Only collect job listing data.
```

This lets Codex/opencode or another agent perform the MCP interaction while Drop the Grind remains responsible for local project state, import, normalization, review, and packet generation.

### Application packet idempotency

Packet folders use deterministic slugs:

```txt
applications/{company-slug}-{title-slug}-{short-job-id}/
```

If a packet already exists for a job:

- do not create a duplicate;
- do not overwrite outputs;
- update missing generated task files only after warning, or write `.new` versions;
- open/reveal the existing packet.

### Right-panel chat scope

Milestone 1 chat is a real local chat UI, not just a textarea, but it has no external model execution.

Required chat features:

- message list with user/assistant/system-style local messages;
- local persistence in SQLite;
- optional export or mirror to `chats/project-chat.md`;
- ability to reference currently open file path;
- ability to reference selected job/application packet;
- buttons to generate/update task markdown files from chat content.

Deferred chat features:

- streaming model responses;
- Codex/opencode execution;
- API model providers;
- tool calls from chat;
- autonomous file edits.

### Edge-case and rescue map

| Failure / edge case | User impact | Detection | Prevention | Recovery |
|---|---|---|---|---|
| Malformed Apify/MCP JSON | Import fails | JSON parse and schema validation | Validate before DB writes | Show row-level errors, keep raw file in imports |
| Actor output fields vary | Missing title/company/apply URL | Normalizer warnings | Field alias mapping and required-field checks | Let user inspect skipped/unmapped rows |
| Duplicate jobs | Noisy queue | Dedupe key unique index | Hash company/title/applyUrl/sourceUrl | Merge/update existing job |
| Workspace path missing | Project cannot open | Startup/project open check | Create on first run | Prompt to recreate or select root |
| Permission denied | File tree/save/import fails | Rust IO errors | Tauri scoped FS permissions | Show actionable error and retry |
| Partial packet write | Broken application folder | Packet manifest/status check | Generate into temp folder then rename | Regenerate missing files from job |
| Editing binary file | Resume/PDF corruption | Extension/MIME check | Read-only binary mode | Restore from original import |
| SQLite/file mismatch | Stale UI | Reconciliation scan | Ownership rules and project schema version | Reindex project from workspace |
| Long import freezes UI | Bad UX | Duration logging | Async Rust task/background import | Cancel/retry import |
| Symlink path escape | Data exposure | Canonical path check | Reject escaped canonical path | Block operation and warn |
| Existing packet regenerated | User edits lost | Existing folder check | Idempotent packet records | Write `.new` files or ask user |
| MCP task run externally writes wrong file | Import missing | Expected file check | Generated task gives exact output path | User selects produced JSON manually |
| App schema changes | Old projects break | Manifest/schema version check | Migrations and backups | Backup then migrate/reindex |

### Test plan

Unit tests:

- workspace path validation and traversal rejection;
- dedupe key generation;
- slug generation;
- Apify field alias normalization;
- preferences/filter matching;
- packet template rendering;
- chat markdown export formatting.

Rust/Tauri command tests:

- create project creates expected folders/files;
- list tree only returns workspace-contained files;
- read/write allowed text file;
- reject write outside workspace;
- atomic write preserves previous file on failure;
- import JSON creates import and job rows transactionally;
- packet generation is idempotent.

Integration tests:

- create project → show tree → edit file → save/reload;
- generate MCP task/config → manually place sample output JSON → import → normalized jobs;
- approve job → generate packet → re-run generation → no duplicate;
- chat message saved → reload project → messages visible.

Manual smoke test:

1. Launch app.
2. Create project under `~/.dropthegrind/workspace`.
3. Add Apify actor name and input template.
4. Generate MCP task/config files.
5. Place sample Apify output JSON into `sources/imports`.
6. Import jobs.
7. Approve one job.
8. Generate packet.
9. Open task files in center editor.
10. Use right chat panel to draft/update a task prompt.

### Rollout and rollback plan

Initial rollout is local development builds only.

- Add `schema_version` to `project.json` and `projects` table.
- Before migrations, copy SQLite DB to a timestamped backup.
- Never delete workspace artifacts during migration.
- If migration fails, app opens project in read-only recovery mode and offers reindex-from-files.
- Keep sample Apify output fixtures in the repo for repeatable testing.

### Updated engineering risks

Remaining risks after decisions:

1. Building a polished chat UI may still distract from the import/packet loop.
2. Rust-owned SQLite reduces TypeScript DB complexity but requires careful Tauri command design.
3. Apify MCP external-agent workflow depends on users following generated task instructions correctly.
4. Different Apify actors may have very different output shapes, so normalization must start tolerant and inspectable.


## Design review decisions — 2026-06-01

### Decision made

Choose **Option B — Hybrid Job Ops Workspace**.

The app keeps the desired three-panel desktop layout, but job workflow becomes first-class instead of hiding everything inside the file tree.

Core layout:

```txt
┌────────────────────────────── Drop the Grind ──────────────────────────────┐
│ Toolbar: Project switcher · Active source/import status · Settings          │
├────────────── Left ──────────┬────────────── Center ──────────────┬─────────┤
│ Mode nav + file tree         │ Active workspace view               │ Chat    │
│                              │                                      │ panel   │
│ Setup                        │ Setup checklist / Source setup /     │ Local   │
│ Sources                      │ Job Inbox / Packet detail / Editor   │ project │
│ Job Inbox                    │                                      │ chat    │
│ Packets                      │                                      │         │
│ Files                        │                                      │         │
│ Settings                     │                                      │         │
│                              │                                      │         │
│ Project files                │                                      │         │
└──────────────────────────────┴──────────────────────────────────────┴─────────┘
```

### Design goals

1. **Make the job pipeline obvious.** The user should immediately see how to go from source setup → import → review jobs → generate packet.
2. **Keep files transparent.** Generated artifacts remain visible/editable, but file browsing is not the only navigation model.
3. **Avoid fake AI expectations.** The right panel is a real chat UI, but milestone 1 clearly labels it as local/project chat without model execution.
4. **Feel like a premium macOS productivity tool.** Dark, compact, calm, local-first, and portfolio-ready.
5. **Support tired/rushed users.** Job hunting is repetitive and stressful; the UI should reduce uncertainty and show the next action.

### Information architecture

Left panel contains two layers:

1. **Primary mode navigation**
   - `Setup`
   - `Sources`
   - `Job Inbox`
   - `Packets`
   - `Files`
   - `Settings`

2. **Project file tree**
   - Shows the physical workspace folders/files.
   - Can be collapsed if the user is focused on workflow modes.
   - Selecting a file switches center to file editor mode.

Center panel is mode-based:

- **Setup view:** project setup checklist and workspace health.
- **Sources view:** Apify actor/MCP configuration, generated MCP task/config files, import controls, latest import status.
- **Job Inbox view:** normalized jobs as reviewable cards/table rows with filters, status, fit score placeholder, and approve/reject actions.
- **Packets view:** generated application packets, status, folder path, task files, output files.
- **Files view:** text editor/viewer for markdown, JSON, plain text, and LaTeX; read-only binary/PDF handling.
- **Settings view:** workspace path, Apify/MCP settings, future Codex/opencode/API placeholders.

Right panel is contextual local chat:

- Always visible by default on wide screens.
- Shows current project, selected file/job/packet context.
- Saves messages locally.
- Can generate or update task markdown from selected chat content.
- Does not run a model in milestone 1.

### First-run and setup flow

On first launch with no project:

- Show a calm empty state:
  - Title: `Create your job-search workspace`
  - Body: `Drop the Grind stores your resume, sources, jobs, packets, and chat locally under ~/.dropthegrind/workspace.`
  - Primary action: `Create Project`
  - Secondary action: `Open Existing Project`

On new project creation, center opens to **Setup** with checklist:

```txt
Project Setup
[ ] Add resume_original.pdf
[ ] Add or edit resume_extracted.md
[ ] Review preferences.json
[ ] Add Apify actor/source
[ ] Generate MCP task/config
[ ] Import Apify output JSON
[ ] Review jobs
[ ] Generate first application packet
```

Checklist items should deep-link to the relevant mode/file.

### Source setup UX

The **Sources** view should use compact source cards.

Each source card includes:

- Source name.
- Actor name, e.g. `apify/some-job-scraper`.
- MCP server URL, default `https://mcp.apify.com`.
- Input template JSON file path.
- Buttons:
  - `Edit Input JSON`
  - `Generate MCP Config`
  - `Generate Run Task`
  - `Import Output JSON`
- Last import summary:
  - imported count
  - skipped count
  - last error if any

Copy requirement:

- Avoid saying the app “runs Apify” in milestone 1.
- Say: `Generate MCP instructions for Codex/opencode, then import the output JSON here.`

### Job Inbox UX

The **Job Inbox** is the core product screen and must be visible as a primary mode.

Recommended layout:

- Top bar:
  - import status
  - filter by status
  - search
  - sort by newest / fit score / company
- Main list:
  - job title
  - company
  - location/remote badge
  - salary badge if present
  - source badge
  - fit score placeholder or `Not scored yet`
  - short explanation/warnings
  - actions: `Approve`, `Reject`, `Open Apply URL`, `Generate Packet`
- Detail panel or expanded card:
  - description
  - requirements
  - source URL
  - raw JSON link

Empty state:

- Title: `No jobs imported yet`
- Body: `Add an Apify actor in Sources, generate the MCP task, then import the JSON output.`
- Action: `Go to Sources`

Partial import success state:

- `42 jobs imported · 7 skipped because required fields were missing`
- Action: `Review skipped rows`

### Packet UX

The **Packets** view shows generated application packet folders.

Each packet row/card includes:

- company + title
- packet path
- status
- created/updated time
- task file completion hints
- actions:
  - `Open Packet`
  - `Reveal in Finder`
  - `Open tailor_resume.md`
  - `Open verification_report.md`

Existing packet state:

- If user generates a packet for the same job again, show:
  - `Packet already exists. Open it instead?`
  - Actions: `Open Existing`, `Write .new task files`, `Cancel`

### File editor UX

Center file editor requirements:

- Show breadcrumb path.
- Show dirty-state indicator.
- `Save` button and `Cmd+S` support.
- Read-only banner for PDFs/binary files:
  - `Preview/editing is not supported for this file type yet. Reveal in Finder instead.`
- JSON should be monospaced and preserve formatting.
- Markdown/LaTeX can start as plain text with syntax-friendly styling.

### Right chat panel UX

The right panel is a real chat interface but local-only in milestone 1.

Empty state copy:

```txt
Project Chat
Draft instructions, notes, and task prompts for this project.

Model execution is not connected yet. Use this panel to compose task files for Codex/opencode or future providers.
```

Required UI elements:

- Message list.
- Composer input.
- Context chips:
  - current file
  - selected job
  - selected packet
- Actions:
  - `Save message`
  - `Create task file`
  - `Append to current task`
  - `Export chat transcript` later/optional

Do not use animated “AI typing” or assistant response affordances that imply a model is running. If assistant/system-style messages are used, label them as local generated guidance or saved notes.

### Visual hierarchy and component choices

Use shadcn/Tailwind components, adapted for compact desktop density:

- `ResizablePanelGroup` for three-panel layout.
- `ScrollArea` for sidebars, job list, chat messages.
- `Tabs` or segmented controls inside mode views where needed.
- `Card` for source/job/packet cards, but keep them compact.
- `Badge` for status/source/remote/salary.
- `Button` variants:
  - primary: main next action
  - secondary: file/navigation actions
  - ghost: sidebar and utility actions
  - destructive: reject/delete-like actions
- `Dialog` for create project, import JSON, destructive confirmations.
- `Toast` for save/import/packet success.
- `Alert` for permission/import/schema errors.

Panel sizing guidance:

```txt
Left panel: 260–320px default, 220px minimum
Center panel: flexible, minimum 520px
Right chat panel: 320–380px default, 280px minimum
```

Narrow-window behavior:

- Collapse right chat panel first.
- Collapse file tree under left mode nav second.
- Keep the active center workflow usable.

### Interaction states checklist

Must design before/while implementing:

- first-run/no-project state;
- project setup checklist;
- loading workspace/project;
- empty file selected state;
- unsaved editor changes;
- save success/failure;
- binary/PDF read-only state;
- source missing actor name;
- generated MCP config/task success;
- import loading/progress;
- malformed JSON import error;
- partial import success;
- empty job queue;
- job selected/no job selected;
- approve/reject success;
- packet generation loading;
- packet already exists;
- workspace permission denied;
- chat local-only empty state;
- chat message save failure;
- narrow-window collapsed panels.

### Accessibility requirements

- Keyboard support:
  - `Cmd+S` saves active file.
  - Arrow keys navigate file tree and job list.
  - `Enter` opens selected file/job.
  - `Cmd+K` or later command palette can jump to modes/files.
- Visible focus rings on all interactive controls.
- Minimum contrast suitable for dark mode text and borders.
- Do not rely on color alone for job status; use labels/icons.
- File tree and job list should expose semantic roles where practical.
- Chat composer must have an accessible label.
- Destructive actions require confirmation or undo.

### Trust and safety copy

Add subtle trust cues in setup/settings/about surfaces:

- `Local-first: project files stay under ~/.dropthegrind/workspace.`
- `Human-reviewed: Drop the Grind prepares packets; it does not auto-apply.`
- `Resume-grounded: generated tasks should tailor from your existing resume, not invent experience.`
- `MCP-assisted: Apify MCP tasks are run externally in milestone 1; import outputs back into the app.`

### Design risks after decision

1. **Too much UI before core workflow works.** Keep components simple and implement the first vertical slice through these modes.
2. **Chat may still feel fake without model execution.** The local-only label must be clear.
3. **File tree can compete with workflow nav.** Make modes visually primary and file tree secondary/collapsible.
4. **Dark premium UI can become low-contrast.** Use explicit contrast checks and strong focus states.

## Assignment / next steps

Pipeline reviews complete. The plan is ready for implementation after finalizing the first sample Apify actor/output fixture.

Before coding, define:

1. Tauri command boundary.
2. SQLite/Drizzle schema.
3. Workspace template files.
4. Frontend route/layout structure.
5. Source adapter interface.
6. Application packet generator interface.
7. Initial Apify JSON mapping strategy.

Then implement the first vertical slice:

```txt
Create project → show tree → edit file → import sample Apify JSON → normalize jobs → generate one packet
```

## What I noticed about how you think

You are optimizing for practical leverage, not novelty for its own sake. You pushed away complex custom scraper setup and chose Apify-first because it gets you broad source coverage faster. You also reframed the product from a wizard/onboarding flow into a workspace/file model, which suggests you prefer transparent, inspectable artifacts over opaque automation. Finally, you are balancing two outcomes at once: making your own job hunt less repetitive while creating a portfolio project that demonstrates real desktop, local-first, and AI-workflow engineering skill.
