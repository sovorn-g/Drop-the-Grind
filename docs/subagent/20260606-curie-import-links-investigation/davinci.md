# Plan – Import Links Extraction

## What

- Replace the current Import Links Apify JSON import with a Tavily Extract-backed flow.
- When the HuntBrief tab is `Import Links`, the right-panel `Start Hunting` button becomes `Extract Jobs` and creates a new `import-links/<name>/` folder containing one Markdown file per pasted URL.
- Reuse the existing HuntBrief progress dashboard/event-channel pattern for extraction status and completion summary.

## How

- Add a Rust `import_job_links` command that validates URLs, creates a unique import folder, calls Tavily `/extract`, writes human-readable Markdown files, and streams `AgentRunEvent`s.
- Add a small `tavily_extract` command only as a backend wrapper around the same internal Tavily extraction helper.
- Lift the Import Links tab state and textarea state from `HuntBriefPanel` to `App` so `AgentPanel` can switch button behavior and trigger extraction deterministically.
- Extend `HuntProgress`/`HuntProgressDashboard` to support import summaries while preserving current hunt summary behavior.

**Scope**
- In scope: pasted URL parsing/deduping, Tavily connection gating, unique import folder creation, Markdown file writing, progress dashboard, button text/modal updates, textarea sizing.
- Out of scope: Apify hunt changes, Codex chat changes, database job storage, auto-apply behavior, a separate `results.md` for imports.
- Scope assumptions: import runs are always new folders; imported link files are standalone job-context Markdown files rather than normalized `HuntJob`s.

**Assumptions**
- Tavily Extract request uses `POST https://api.tavily.com/extract` with bearer auth and JSON body containing `urls`, `extract_depth`, `include_images`, and `format`.
- Tavily Extract response contains `results[]` with `url` and `raw_content`, plus optional `failed_results[]`.
- If a URL cannot be extracted, continue processing remaining URLs and report it in the import summary.

**Reuses**
- `Channel`, `AgentRunEvent`, `emit_event`, `emit_event_payload` from `src-tauri/src/lib.rs`.
- `read_tavily_key`, `project_root`, `safe_project_path`, `slugify` from `src-tauri/src/lib.rs`.
- `generateHuntFiles` event-handling pattern from `src/main.tsx`.
- `HuntProgressDashboard`, `hunt-status-*` styles, and `.import-drop` styles from `src/main.tsx` and `src/styles.css`.
- `call<T>()`, `refreshTree`, `openPath`, `debugLog`, and `tavily` state from `App` in `src/main.tsx`.

## TODO

1. Update `src-tauri/src/lib.rs` near existing input structs to add `TavilyExtractInput`, `TavilyExtractOutput`, `ImportJobLinksInput`, `ImportLinkFile`, and `ImportJobLinksSummary` with `serde(rename_all = "camelCase")`.
2. Add `tavily_extract_urls` in `src-tauri/src/lib.rs` near `search_tavily` to call Tavily `/extract` with `read_tavily_key` and `curl`, parse `results[].raw_content`, and surface `failed_results[]` as per-URL failures. (uses: `read_tavily_key` from `src-tauri/src/lib.rs`)
3. Add `tavily_extract` command in `src-tauri/src/lib.rs` that validates one `http`/`https` URL, calls `tavily_extract_urls`, and returns `TavilyExtractOutput`. (uses: `tavily_extract_urls` from `src-tauri/src/lib.rs`)
4. Add import Markdown helpers in `src-tauri/src/lib.rs`: `valid_import_url`, `dedupe_import_urls`, `import_title_from_content`, `import_link_file_name`, and `import_link_markdown`. (uses: `slugify` from `src-tauri/src/lib.rs`)
5. Add `import_job_links` command in `src-tauri/src/lib.rs` following `start_hunt_apify`: create `import-links/<slug>` with timestamp/numeric suffix if needed, stream status per URL, write `001-<slug>.md` files, and emit a completed payload with `mode: "import"`, `submitted`, `extracted`, `written`, `failed`, `folderPath`, `files`, and `failures`. (uses: `emit_event`, `emit_event_payload`, `project_root`, `safe_project_path` from `src-tauri/src/lib.rs`)
6. Update the `tauri::generate_handler!` list in `src-tauri/src/lib.rs` to register `tavily_extract` and `import_job_links`.
7. Update `src/main.tsx` type aliases: remove the old `ImportResult` usage, add `ImportSummary`, widen `HuntProgress.summary` to `HuntSummary | ImportSummary`, and add optional `contextLabel` and `resetLabel` fields to `HuntProgress`.
8. Add a shared `extractImportUrls(text: string): string[]` helper in `src/main.tsx` that matches `https?://\S+`, trims trailing punctuation, and dedupes in first-seen order.
9. Refactor `App` in `src/main.tsx` to own `huntTab` and `importText`, pass them plus setters to `HuntBriefPanel`, and pass `huntTab`, `importLinkCount`, and Tavily/project readiness to `AgentPanel`.
10. Replace `importJobs` in `src/main.tsx` with `extractImportedJobs(importName: string)`: validate project/Tavily/URLs, set import `HuntProgress`, call `import_job_links` with a `Channel<AgentRunEvent>`, handle streamed events like `generateHuntFiles`, refresh the tree, and open the first written Markdown file when available. (uses: `call`, `refreshTree`, `openPath`, `debugLog` from `src/main.tsx`)
11. Update `HuntBriefPanel` in `src/main.tsx` to use controlled `tab` and `importText` props, remove the in-card extraction button, keep the link counter, and show a short hint that extraction runs from the right-panel button.
12. Update `HuntProgressDashboard` in `src/main.tsx` to render `progress.contextLabel || "Start Hunting"`, import-specific summary rows/file list when `summary.mode === "import"`, and `progress.resetLabel || "Start another hunt"`. (uses: existing `hunt-status-*` classes from `src/styles.css`)
13. Update `AgentPanel` in `src/main.tsx` props and top button logic: when `huntTab === "import"`, render `Extract Jobs`, open an import-name modal without hunt profile re-run options, disable when no project/no links/no Tavily connection, and submit to `onExtractJobs(name)`; otherwise preserve the existing Start Hunting flow.
14. Update `src/styles.css` to increase `.import-drop textarea` `min-height` to at least `240px`, add styles for the import hint, and add compact styles for dashboard import file rows.
15. Remove stale `importPath`, `importResult`, and `import_apify_json` frontend wiring from `src/main.tsx`; keep the Rust `import_apify_json` command only if other code still references it.

## Outcome

- Pasting links in `Import Links` makes the right-panel button become `Extract Jobs`.
- Extracting jobs requires a connected Tavily key, creates a fresh `import-links/<name>/` folder, and writes one readable Markdown file per successfully extracted URL.
- The progress dashboard shows import progress, generated file links, failure counts, and a reset action without showing HuntBrief search settings.
- Existing `Find Jobs` / Apify Start Hunting behavior remains unchanged.
- Generated import files are human-readable Markdown and contain no raw API JSON dumps.
