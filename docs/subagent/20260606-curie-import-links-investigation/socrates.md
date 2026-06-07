# Reflection Review – Import Links Extraction

## Scope Reviewed

- Implementation plan: `docs/subagent/20260606-curie-import-links-investigation/davinci.md`
- Discovery handoff: `docs/subagent/20260606-curie-import-links-investigation/curie.md`
- Key source files verified: `src-tauri/src/lib.rs` (Rust commands, helpers, event streaming pattern), `src/main.tsx` (React components, state management, type definitions)
- Not reviewed: CSS implementation details, Tavily API documentation (plan flags as assumption)

## 🔴 Blocking Issues

None.

## 🟡 Should Fix

**1. Tavily Extract API assumptions need explicit fallback handling**

The plan assumes `POST https://api.tavily.com/extract` with `{urls, extract_depth, include_images, format}` and response `{results[], failed_results[]}`. Curie flagged this as an open question. The plan should either:
- Verify against actual Tavily API docs before implementation, OR
- Add explicit error handling in `tavily_extract_urls` for when the API spec differs (e.g., different field names, different response structure)

Without verification, implementation could fail at runtime if the API contract differs. The plan's "assumptions" section acknowledges this but TODO #2 doesn't include validation logic.

**2. `extractImportedJobs` needs clearer URL source specification**

TODO #10 says `extractImportedJobs(importName: string)` should "validate project/Tavily/URLs" but doesn't specify where URLs come from. Since `importText` is being lifted to `App` state (TODO #9), the function needs to access `importText` and call `extractImportUrls(importText)` internally. The plan should make this explicit: `extractImportedJobs` reads `importText` from `App` scope, parses URLs via the shared helper, then proceeds.

**3. Import modal UI needs explicit specification**

TODO #13 says the import name modal should be "without hunt profile re-run options" but doesn't describe what it should include. The existing hunt name modal has:
- Name input field
- Hunt profile dropdown (for re-runs)
- Create/Submit button

The import modal should have:
- Name input field only
- Extract button
- Cancel button

Without explicit UI spec, implementation may need to guess at the layout or accidentally include irrelevant hunt profile options.

## 💡 Optional Suggestions

**1. Per-URL event streaming could be more explicit**

TODO #5 says "stream status per URL" but doesn't specify event structure. The existing `start_hunt_apify` emits `started`, `status`, `debug`, `completed`, `failed`. The import command should follow the same pattern:
- Emit `status` per URL: "Extracting URL 3 of 10: https://..."
- Emit `debug` for extraction details
- Include URL index and total in progress updates

This would help the dashboard show granular progress. Consider adding to TODO #5.

**2. `ImportLinkFile` struct fields could be explicit**

TODO #1 defines `ImportLinkFile` but doesn't list fields. Based on the plan's outcome (human-readable Markdown files per URL), it should have:
- `url: String`
- `title: String` (extracted or derived)
- `file_path: String` (relative path to .md file)
- `extracted: bool` (success/failure)
- `error: Option<String>` (failure reason)

Adding this to TODO #1 would reduce ambiguity.

**3. Dashboard file list interaction could be specified**

TODO #12 says to render "import-specific summary rows/file list" but doesn't specify if files are clickable. The hunt dashboard has `openPath` for opening result files. The import dashboard should probably allow clicking each file to open it. Consider adding to TODO #12: "file list with clickable rows that call `openPath(file.filePath)`".

**4. `import_apify_json` removal decision could be definitive**

TODO #15 says "keep the Rust `import_apify_json` command only if other code still references it". From grep, only `main.tsx:196` (the `importJobs` function being replaced) references it. The plan should just say "remove `import_apify_json` from Rust" since no other code uses it.

## ✅ What Is Solid

- **State lifting approach**: Lifting `huntTab` and `importText` from `HuntBriefPanel` to `App` is correct and follows React best practices for cross-component communication. The plan correctly identifies that `AgentPanel` needs access to tab state to switch button behavior.

- **Event streaming pattern reuse**: Reusing `Channel<AgentRunEvent>`, `emit_event`, `emit_event_payload` from the existing `start_hunt_apify` flow is deterministic and follows established patterns.

- **Helper function reuse**: Plan correctly identifies and reuses `project_root`, `safe_project_path`, `slugify`, `read_tavily_key` from existing Rust code.

- **Folder structure consistency**: Creating `import-links/<name>/` with timestamp/numeric suffix mirrors `hunt_run/<name>/` pattern, making it predictable and consistent.

- **Type extension approach**: Extending `HuntProgress` with optional `contextLabel` and `resetLabel` fields, and widening `summary` to `HuntSummary | ImportSummary`, preserves backward compatibility while supporting import-specific data.

- **Scope boundaries**: Plan explicitly excludes Apify hunt changes, Codex chat, database storage, auto-apply, and separate `results.md` for imports. This keeps scope focused.

- **Button behavior specification**: Clear spec that "Extract Jobs" button should disable when no project/no links/no Tavily connection, and should open import-specific modal (not hunt profile modal).

- **Markdown output requirement**: Explicit requirement that generated files are "human-readable Markdown and contain no raw API JSON dumps" aligns with project guidelines.

## ⚖️ Verdict

REVISE: revise the implementation plan before implementation.

The plan is well-grounded and follows existing patterns correctly. The three should-fix issues (Tavily API validation, URL source clarity, import modal spec) are non-blocking but would reduce implementation risk if addressed. Once revised, the plan will be ready for execution.
