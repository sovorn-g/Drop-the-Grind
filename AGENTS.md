# AGENTS.md — Guidance for Coding Agents

This file gives future agents enough context to work safely in Drop the Grind.

## Project summary

Drop the Grind is a local-first macOS Tauri app for job hunting. The core workflow is HuntBrief: run selected Apify actors through the direct Apify Actor API, normalize/filter results, then write human-readable Markdown artifacts.

Do not reintroduce Codex/Apify MCP into Start Hunting. Codex is for chat and later resume/outreach assistance, not deterministic scraping.

## Important paths

```txt
src/main.tsx              React UI and frontend command wiring
src/styles.css            visual design system and component styling
src-tauri/src/lib.rs      Rust backend commands and Apify execution
PROJECT.md                current project architecture and decisions
DESIGN.md                 UI/UX style guide
README.md                 user/developer quickstart
docs/apify/               actor notes and schema generator scripts
```

Runtime/workspace paths:

```txt
~/.dropthegrind/settings.json
~/.dropthegrind/workspace/<project-slug>/
```

Build output:

```txt
src-tauri/target/
dist/
```

## Current HuntBrief behavior

Start Hunting should:

1. create `hunt_run/<name>/`
2. run selected Apify Actor API calls from Rust
3. normalize each actor's output with actor-specific adapters
4. post-filter by HuntBrief settings where possible
5. dedupe
6. write `results.md` as an index
7. write one detailed file per job under `jobs/*.md`

Do not write raw API JSON into `results.md` or job files.

## Coding rules

- Keep product-critical flows deterministic in Rust/Tauri commands.
- Keep user-facing job artifacts Markdown and human-readable.
- Prefer one focused job file for agent resume/outreach work.
- Do not silently mutate global Codex config such as `~/.codex/config.toml`.
- Do not store HuntBrief result data in DB unless the user explicitly asks.
- Keep app settings app-local under `~/.dropthegrind/`.
- Do not implement auto-apply behavior.

## Apify actor maintenance

Actor list notes live in:

```txt
docs/apify/standard/file.md
docs/apify/remote/file.md
```

Schema note generators:

```bash
docs/apify/generate-api-input-schema.py
docs/apify/generate-api-output-schema.py
```

If actor lists change, update the UI actor catalog/input adapters/output adapters in source code and regenerate notes as needed.

## Build and checks

Use:

```bash
npm run build
PATH="$(rustup which cargo | xargs dirname):$PATH" cargo check --manifest-path src-tauri/Cargo.toml
./build.sh
```

If the repo moved and Tauri fails with stale absolute paths:

```bash
rm -rf src-tauri/target
./build.sh
```

Existing Rust warnings in the Codex chat path may appear; do not treat them as new HuntBrief failures unless changed.

## When editing UI

Follow `DESIGN.md`.

Important current visual direction:

- dark glass macOS-style UI
- LinkedIn-blue primary accents
- compact pro-tool density
- transparent HuntBrief panels
- Settings opened from lower-left app Settings, not a separate HuntBrief settings button

## When editing HuntBrief output

Keep:

```txt
hunt_run/<name>/results.md
hunt_run/<name>/jobs/001-title-company.md
```

`results.md` should be an index. Job detail files should be focused enough for an agent to read individually.
