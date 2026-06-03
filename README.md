# Drop the Grind

Drop the Grind is a local-first macOS desktop app for job discovery and application preparation. It helps turn structured HuntBrief settings into curated job-result files, then keeps each job readable for later resume tailoring or outreach.

It is not an auto-apply bot. It is a human-reviewed job hunt workspace.

## Current core workflow

1. Create or open a local project.
2. Configure Apify API in Settings.
3. Use HuntBrief → Find Jobs.
4. Pick Standard or Remote curated sources.
5. Click Start Hunting and name the hunt.
6. Drop the Grind runs the selected Apify actors directly from the Rust backend.
7. Results are normalized, post-filtered, deduped, and written as Markdown.

Output example:

```txt
~/.dropthegrind/workspace/my-2026-job-search/
└── hunt_run/
    └── ai-engineer-uk-nz/
        ├── results.md
        └── jobs/
            ├── 001-ai-engineer-company.md
            └── 002-ai-automation-engineer-company.md
```

`results.md` is a summary/index. Each `jobs/*.md` file is one focused job context for later agent work.

## Current integrations

- Apify API for deterministic HuntBrief scraping.
- Tavily API key storage/testing in Settings.
- Codex app-server based Agent Chat for project conversation/help.

Start Hunting does not use Codex or Apify MCP.

## Tech stack

- React 19
- TypeScript
- Vite
- Tauri 2
- Rust
- SQLite/local filesystem storage

## Development

Install dependencies:

```bash
npm install
```

Run frontend build:

```bash
npm run build
```

Run Rust check:

```bash
PATH="$(rustup which cargo | xargs dirname):$PATH" cargo check --manifest-path src-tauri/Cargo.toml
```

Build the macOS app bundle:

```bash
./build.sh
```

Outputs:

```txt
src-tauri/target/release/bundle/macos/Drop the Grind.app
src-tauri/target/release/bundle/dmg/Drop the Grind_0.1.0_aarch64.dmg
```

If the repo path changes and Tauri reports stale absolute paths, clear generated build output:

```bash
rm -rf src-tauri/target
./build.sh
```

## Documentation

- `PROJECT.md` — current architecture, decisions, and workflow.
- `DESIGN.md` — UX/UI design direction.
- `AGENTS.md` — guidance for future coding agents.
- `docs/apify/` — Apify actor notes and schema generator scripts.

## Safety principles

- Local-first workspace under `~/.dropthegrind/`.
- Human-reviewed job workflow.
- No auto-apply behavior.
- No raw API JSON in user-facing HuntBrief results.
- One job file per downstream resume/outreach task.
