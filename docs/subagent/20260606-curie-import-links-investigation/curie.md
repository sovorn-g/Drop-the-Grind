## Scope Investigated

- Current **Import Links** tab UI in `HuntBriefPanel` and its backing `importJobs` function
- **Start Hunting** flow: button placement, modal, `generateHuntFiles`, `start_hunt_apify` event pattern, folder structure
- **HuntProgressDashboard** component: states, fields, UI structure
- **Tavily** Rust integration: existing `tavily_connect`, `search_tavily`, key reading — and what is missing (`extract` endpoint)
- **Rust backend** structures: `HuntJob`, `HuntRunInput`, `HuntConfig`, command registration
- Did NOT investigate: CSS variants for non-hunt progress views, Codex agent chat flow, Apify actor details beyond what is needed for the Start Hunting pattern reference.

## Findings

### 1. Current Import Links UI (`HuntBriefPanel`, `src/main.tsx:248-264`)

- `HuntBriefPanel` renders tabs: "Find Jobs" | "Import Links" (line `src/main.tsx:259`)
- Import tab renders `div.import-links-layout > div.import-drop` containing:
  - `<Upload>` icon + heading "Paste job links" + description
  - `<textarea>` — links are newline-separated (placeholder confirms: `https://...\nhttps://...`)
  - `<div.actions>` with `"Extract jobs"` button and `<small>{extractedLinks} links detected</small>`
- `extractedLinks` computed as: `(importText.match(/https?:\/\/\S+/g)||[]).length` (line `src/main.tsx:257`)
- Relevance: This is the starting point — the "Extract jobs" button, counters, and textarea height all need to change per user spec.

### 2. `importJobs` callback is wired to old Apify JSON import (`src/main.tsx:196`)

- `const importJobs = async()=>{ ... call<ImportResult>('import_apify_json', ...) }`
- Calls the Rust command `import_apify_json` which reads a hardcoded `sources/imports/sample-raw.json` file and normalizes it.
- This has nothing to do with what the user wants. Entirely needs replacement.
- Relevance: New import flow must use a different Rust command. This function will be replaced or repurposed.

### 3. Start Hunting button placement (`AgentPanel`, `src/main.tsx:301`)

- The "Start Hunting" button lives inside `AgentPanel` (the right-side chat panel), NOT in `HuntBriefPanel`:
  ```html
  <div className="hunt-button-wrap">
    <button className="start-hunting" disabled={!project} onClick={...}>
      <Play size={16}/> Start Hunting
    </button>
  </div>
  ```
- On click, it opens a name modal (`huntNameOpen` portal) that prompts for a hunt name, then calls `onStartHunting` (which is `generateHuntFiles` from App).
- Relevance: User wants this button to **dynamically switch** to "Extract Jobs" when the Import Links tab is active. This requires cross-component communication between `HuntBriefPanel` (tab state) and `AgentPanel` (button).

### 4. `generateHuntFiles` flow and dashboard (`src/main.tsx:195`)

- The Start Hunting flow:
  1. `setHuntProgress({running:true, ...})` — switches `HuntBriefPanel` from showing tabs to showing `<HuntProgressDashboard>`
  2. Calls `create_hunt_run` Rust command → creates `hunt_run/<slug>/` folder
  3. Calls `start_hunt_apify` Rust command with event channel → runs Apify actors
  4. On completion, emits `HuntSummary` payload to `huntProgress.summary`
  5. Dashboard shows summary stats (rawFound, newJobs, filtered, duplicates, totalDistinctJobs, sourceFailures)
- Relevance: The import flow should follow the same pattern — set `huntProgress` to show a dashboard, call a new Rust command, receive events.

### 5. `HuntProgressDashboard` component (`src/main.tsx:267`)

- Takes `{progress: HuntProgress; onReset: ()=>void}` props
- Renders: `<small>Start Hunting</small>`, title, progress %, progress bar, detail text, settings chips, step indicators, summary (when done), reset button
- Summary section shows: `rawFound`, `newJobs`, `filtered`, `duplicates`, `totalDistinctJobs`, `sourceFailures`
- Relevance: The import dashboard should reuse this component or a similar-style variant. The `<small>Start Hunting</small>` label should change for import context.

### 6. `HuntProgress` type (`src/main.tsx:22-24`)

```ts
type HuntProgress = { running:boolean; title:string; progress:number; detail:string; steps:HuntProgressStep[]; resultPath?:string; settings?:HuntSettings; summary?:HuntSummary };
```
- `HuntSettings` has fields like `roles`, `location`, `workMode`, `selectedSites`, `maxItems`, etc. — these don't apply to import.
- `HuntSummary` has `rawFound`, `newJobs`, `filtered`, `duplicates`, `totalDistinctJobs`, `totalRuns`, `jobDirName`, `resultsPath`, `sourceFailures`
- Relevance: May need to extend or create a separate type for import progress. At minimum, `settings` won't be used and `summary` fields will differ (no `sourceFailures`, maybe different labels).

### 7. Dashboard display logic in `HuntBriefPanel` (`src/main.tsx:264`)

```tsx
{huntProgress ? <HuntProgressDashboard ... /> : tab==='find' ? <find-form> : <import-form>}
```
- When `huntProgress` is truthy, the dashboard replaces both tabs entirely.
- `clearHuntProgress` sets `huntProgress` to null.
- Relevance: This pattern works for import too — set `huntProgress` on extract, show dashboard, reset on done.

### 8. Tavily Rust backend — existing capabilities (`src-tauri/src/lib.rs`)

- `tavily_status` (line 2365): checks if key is stored, returns `TavilyStatus`
- `tavily_connect` (line 2374/2186): takes `{apiKey}`, tests against `https://api.tavily.com/search`, stores key
- `tavily_disconnect` (line 2391): removes stored key
- `search_tavily` (line 1809/1997): calls `POST https://api.tavily.com/search` with `{query, search_depth, max_results}`, parses results
- `read_tavily_key` (line 2169/2357): reads `tavilyApiKey` from settings store
- **No `tavily_extract` command exists.** Tavily's extract endpoint is `https://api.tavily.com/extract` (noted by user, not yet in codebase).
- Relevance: Need to add a new `tavily_extract` command that calls Tavily's `/extract` endpoint for a URL.

### 9. `create_hunt_run` Rust command (`src-tauri/src/lib.rs:650`)

- Creates `hunt_run/<slug>/` folder
- Writes `.hunt_config.json` (HuntConfig) and `.hunt_result.json` (empty HuntResultDB)
- Writes initial `results.md`
- Relevance: Similar command needed for `import-links/<name>/` — but simpler: just create folder, no `results.md`, no config/result DB.

### 10. `start_hunt_apify` Rust command pattern (`src-tauri/src/lib.rs:987`)

- Spawns a thread, uses `Channel<AgentRunEvent>` for streaming progress events back to frontend
- Events: `started`, `status`, `debug`, `completed` (with payload), `failed`
- Writes job MD files via `job_detail_markdown` (line 954), which produces structured per-job Markdown
- Relevance: New import command should follow the same event-streaming pattern, iterate over links, call Tavily extract per link, write MD files.

### 11. `HuntJob` struct (`src-tauri/src/lib.rs:1252`)

Fields: `title, company, location, work_mode, seniority, experience, salary, posted_date, apply_url, source_url, source_name, actor_slug, description, requirements: Vec<String>, skills: Vec<String>`
- This is the normalized job structure used by `job_detail_markdown` to write hunt job files.
- Relevance: Imported jobs from Tavily extract will have different fields. May need a simpler struct or adapt `HuntJob` with fewer required fields.

### 12. Command registration (`src-tauri/src/lib.rs:2216`)

All Tauri commands are registered in the `generate_handler!` macro. New commands (`tavily_extract`, `import_job_links`) must be added here.

### 13. CSS for import area (`src/styles.css`)

- `.import-links-layout` (line 109): `flex:1; display:flex; flex-direction:column; gap:10px`
- `.import-drop textarea` (line 109): `min-height:140px` — user wants this **increased**
- `.import-drop .actions` (line 66): holds the button + counter text
- `.import-note` (line 66): the result banner
- `.hunt-status-dashboard` (line 244): full dashboard styling

### 14. SettingsModal — Tavily connection UI (`src/main.tsx:227`)

Already has full Tavily connect/disconnect UI with key input and test. No changes needed here — the import flow just needs to check `tavily?.connected` before allowing extraction.

### 15. Tab/button communication gap

- `HuntBriefPanel` owns the `tab` state (`'find'|'import'`) — line `src/main.tsx:252`
- `AgentPanel` owns the "Start Hunting" button — line `src/main.tsx:301`
- These are sibling components under `App`. No mechanism currently passes tab state from `HuntBriefPanel` → `AgentPanel`.
- Relevance: Need to lift the active tab state to `App`, or add a prop/callback to coordinate button label switching.

## Relationships

- **`HuntBriefPanel`** renders tabs and imports form; receives `huntProgress`, `importJobs`, `clearHuntProgress` props from `App`
- **`AgentPanel`** renders Start Hunting button; receives `onStartHunting` prop from `App` (which is `generateHuntFiles`)
- **`App`** owns `huntProgress` state, `huntCreating` state, and all hunt/import callback functions; passes props down to both panels
- **`HuntProgressDashboard`** is the shared progress view used by both flows
- **Rust `start_hunt_apify`** is the event-streaming pattern to follow for the new `import_job_links` command
- **Rust `search_tavily`** uses `curl` to call Tavily API — same pattern needed for `/extract` endpoint
- **`job_detail_markdown`** writes per-job `.md` files — similar function needed for import jobs
- **`create_hunt_run`** creates hunt folders — simpler version needed for `import-links/<name>/`

## Open Questions / Gaps

1. **Tavily Extract API spec**: The exact JSON body for `POST https://api.tavily.com/extract` is unknown from the codebase. Typically it takes `urls: string[]` and returns extracted content. Check Tavily docs for field names and response structure.
2. **Extract response → job fields**: What fields does Tavily extract return? Likely `title`, `content`/`raw_content`, `url`. Need to map these into a job MD file. Unlike Apify actors, there won't be structured `company`, `salary`, `location`, etc. The MD files will be more freeform.
3. **MD file naming**: Hunt jobs use `job_file_name(i+1, norm)` → `001-title-company.md`. For import, the user's spec says each link gets its own `.md` file. Naming scheme TBD — likely `001-<slug-from-url>.md`.
4. **`HuntProgress` type reuse**: The current type has `settings?:HuntSettings` and `summary?:HuntSummary`. For import, `settings` is unused and `summary` fields differ. Need to either make these optional/union types, or create `ImportProgress`.
5. **Dashboard small label**: `HuntProgressDashboard` hardcodes `<small>Start Hunting</small>`. For import, this should say something like "Import Links" or be configurable via a prop. Currently no prop for this.
6. **No `results.md` for imports**: Confirmed from user spec — just `import-links/<name>/` with individual `.md` files, index shown in dashboard. No separate index file.
7. **Each import = new folder**: User specifies "each import is a new folder" — no re-run/same-folder logic needed (unlike hunt runs which support re-running into the same folder).

## Start Here

1. **`src/main.tsx:248`** — `HuntBriefPanel` function. Understand the `tab` state, the import tab rendering, and how `huntProgress` toggles between tabs vs dashboard.
2. **`src-tauri/src/lib.rs:987`** — `start_hunt_apify` function. This is the pattern to clone for the new `import_job_links` Rust command (event streaming, thread spawning, MD file writing).
