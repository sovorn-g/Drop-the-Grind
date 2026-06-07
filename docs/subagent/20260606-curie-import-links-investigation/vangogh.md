# Revised Plan – Import Links Extraction

## What

- Replace the current `Import Links` Apify JSON import with a Tavily Extract-backed import flow.
- When the active HuntBrief tab is `Import Links`, the right-panel `Start Hunting` button becomes `Extract Jobs` and creates a fresh `import-links/<name>/` folder with one Markdown file per successfully extracted URL.
- Reuse the existing HuntBrief event-channel progress pattern and dashboard styling without changing the Apify `Find Jobs` flow.

## How

- Add Rust import commands/helpers in `src-tauri/src/lib.rs` that validate/dedupe pasted URLs, create a unique import folder, call Tavily `/extract`, write human-readable Markdown, and stream `AgentRunEvent` updates.
- Make Tavily Extract parsing defensive: use the expected request/response contract, retry with a minimal body on request-shape failure, accept known alternate content fields, and fail clearly if the response schema is not recognized.
- Lift `huntTab` and `importText` from `HuntBriefPanel` to `App` so `AgentPanel` can switch button labels, disabled states, and modal behavior deterministically.
- Extend `HuntProgress`/`HuntProgressDashboard` with an import summary mode while preserving current hunt summary rendering.

**Scope**

- In scope: pasted URL parsing/deduping, Tavily connection gating, unique `import-links/<name>/` folder creation, Markdown file writing, progress dashboard, import-specific right-panel button/modal, larger textarea.
- Out of scope: Apify hunt behavior changes, Codex chat behavior changes, database storage for imported links, auto-apply behavior, separate `results.md` for imports.
- Scope assumptions: every import run creates a new folder; imported link files are standalone job-context Markdown files rather than normalized `HuntJob`s.

**Assumptions**

- Tavily Extract is called at `POST https://api.tavily.com/extract`; the exact response schema is not guaranteed by the repository, so implementation must use tolerant parsing and clear schema-mismatch errors instead of assuming one rigid shape.
- Continue processing remaining URLs when one URL fails; summarize per-URL failures in the dashboard.

**Reuses**

- `Channel`, `AgentRunEvent`, `emit_event`, `emit_event_payload` from `src-tauri/src/lib.rs`.
- `read_tavily_key`, `project_root`, `safe_project_path`, `slugify` from `src-tauri/src/lib.rs`.
- `generateHuntFiles` event-handling pattern from `src/main.tsx`.
- `HuntProgressDashboard`, `hunt-status-*` styles, `.import-drop` styles from `src/main.tsx` and `src/styles.css`.
- `call<T>()`, `refreshTree`, `openPath`, `debugLog`, `tavily` state from `App` in `src/main.tsx`.

**Review fixes applied**

- Added explicit Tavily Extract fallback/tolerant schema handling.
- Specified that `extractImportedJobs` reads `importText` from `App` and parses it via `extractImportUrls(importText)`.
- Specified import modal contents and actions.
- Made per-URL event payloads, `ImportLinkFile` fields, clickable dashboard file rows, and old `import_apify_json` removal explicit.

## TODO

1. Update `src-tauri/src/lib.rs` near existing input/result structs to add `TavilyExtractInput`, `TavilyExtractOutput`, `ImportJobLinksInput`, `ImportLinkFile`, `ImportLinkFailure`, and `ImportJobLinksSummary` with `serde(rename_all = "camelCase")`; define `ImportLinkFile` fields as `url`, `title`, `file_path: Option<String>`, `extracted`, and `error: Option<String>`.
2. Add `tavily_extract_urls` in `src-tauri/src/lib.rs` near `search_tavily` to call `POST https://api.tavily.com/extract` with `read_tavily_key`, first using `{ urls, extract_depth, include_images, format }`, then retrying once with minimal `{ urls }` if Tavily rejects the request shape. (uses: `read_tavily_key` from `src-tauri/src/lib.rs`)
3. Update `tavily_extract_urls` parsing in `src-tauri/src/lib.rs` to accept `results[]` entries with `url` plus one of `raw_content`, `content`, or `text`, collect failures from `failed_results[]` or equivalent failure arrays when present, and return a clear `Tavily Extract response schema not recognized` error when no extractable result/failure shape exists. (uses: `serde_json::Value` from `src-tauri/src/lib.rs`)
4. Add `tavily_extract` command in `src-tauri/src/lib.rs` that validates one `http`/`https` URL, calls `tavily_extract_urls`, and returns `TavilyExtractOutput`. (uses: `tavily_extract_urls` from `src-tauri/src/lib.rs`)
5. Add import Markdown helpers in `src-tauri/src/lib.rs`: `valid_import_url`, `dedupe_import_urls`, `import_title_from_content`, `import_link_file_name`, and `import_link_markdown`; ensure Markdown includes URL, extracted title, extracted content, and no raw JSON dump. (uses: `slugify` from `src-tauri/src/lib.rs`)
6. Add `import_job_links` command in `src-tauri/src/lib.rs` following `start_hunt_apify`: create unique `import-links/<slug>` folder, emit `started`, emit `status` per URL with payload `{ index, total, url }`, emit `debug` for concise extraction/write details, write `001-<slug>.md` files for successes, and emit `completed` with `ImportJobLinksSummary`. (uses: `emit_event`, `emit_event_payload`, `project_root`, `safe_project_path` from `src-tauri/src/lib.rs`)
7. Update the `tauri::generate_handler!` list in `src-tauri/src/lib.rs` to register `tavily_extract` and `import_job_links`.
8. Remove stale Rust Apify JSON import surface from `src-tauri/src/lib.rs`: `ImportInput`, `ImportResult`, `import_apify_json`, and the `import_apify_json` generate-handler entry.
9. Update `src/main.tsx` type aliases: remove `ImportResult`, add `ImportSummary`, `ImportLinkFile`, and `ImportLinkFailure`, widen `HuntProgress.summary` to `HuntSummary | ImportSummary`, and add optional `contextLabel` and `resetLabel` fields to `HuntProgress`.
10. Add `extractImportUrls(text: string): string[]` in `src/main.tsx` that matches `https?://\S+`, trims trailing punctuation characters, rejects non-HTTP(S) values, and dedupes in first-seen order.
11. Refactor `App` in `src/main.tsx` to own `huntTab` and `importText`, pass them plus setters to `HuntBriefPanel`, and pass `huntTab`, `importLinkCount`, and Tavily/project readiness props to `AgentPanel`.
12. Replace `importJobs` in `src/main.tsx` with `extractImportedJobs(importName: string)`: read `importText` from `App` scope, parse URLs with `extractImportUrls(importText)`, validate project/Tavily/URL count, set import `HuntProgress`, call `import_job_links` with a `Channel<AgentRunEvent>`, handle streamed events like `generateHuntFiles`, refresh the tree, and open the first written Markdown file when available. (uses: `call`, `refreshTree`, `openPath`, `debugLog` from `src/main.tsx`)
13. Update `HuntBriefPanel` in `src/main.tsx` to use controlled `tab` and `importText` props, remove the in-card `Extract jobs` button, keep the link counter from `extractImportUrls(importText).length`, and show an import hint that extraction runs from the right-panel button.
14. Update `HuntProgressDashboard` in `src/main.tsx` to render `progress.contextLabel || "Start Hunting"`, branch on `summary.mode === "import"`, show submitted/extracted/written/failed counts, render clickable file rows that call `openPath(file.filePath)` for successful files, render failure details, and use `progress.resetLabel || "Start another hunt"`. (uses: existing `hunt-status-*` classes from `src/styles.css`)
15. Update `AgentPanel` in `src/main.tsx` props and top button logic: when `huntTab === "import"`, render `Extract Jobs`, disable when no project/no links/no Tavily connection, and open an import-only modal titled `Extract Jobs` with one name input, `Cancel`, and primary `Extract jobs` button; no hunt profile list, no re-run options, Enter submits when name is non-empty, Escape cancels.
16. Update `AgentPanel` in `src/main.tsx` to preserve the existing `Start Hunting` modal and re-run profile behavior unchanged when `huntTab !== "import"`.
17. Update `src/styles.css` to increase `.import-drop textarea` `min-height` to at least `240px`, add styles for the import hint, and add compact dashboard styles for import file rows and failure rows.
18. Remove stale frontend state/wiring in `src/main.tsx`: `importPath`, `importResult`, `setImportResult`, `refreshJobs` calls used only by the old import flow, and any `call('import_apify_json', ...)` usage.

## Outcome

- Pasting links in `Import Links` makes the right-panel button become `Extract Jobs`.
- Extracting requires a selected project, at least one parsed link, and a connected Tavily key.
- Each extraction creates a fresh `import-links/<name>/` folder and writes one readable Markdown file per successful URL.
- The progress dashboard shows import progress, clickable generated files, failure counts/details, and a reset action without showing HuntBrief search settings.
- Existing `Find Jobs` / Apify Start Hunting behavior remains unchanged.
- Generated import files are human-readable Markdown and contain no raw API JSON dumps.