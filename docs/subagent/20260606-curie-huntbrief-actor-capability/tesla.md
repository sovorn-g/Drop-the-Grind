# Implementation Report – HuntBrief Actor Field Capabilities

## Completed

Implemented the full revised plan for upgrading HuntBrief field capability tracking from binary `supported/unsupported` to four-tier `native`, `foldable`, `postFilterable`, `unsupported` across both frontend and backend.

### Frontend (`src/main.tsx`)
- Added `ActorFieldCapability` type (`'native' | 'foldable' | 'postFilterable' | 'unsupported'`).
- Updated `ActorCapabilities` to include `postFilterableFields`, with `unsupportedFields` derived as the remainder.
- Updated `caps()` helper to accept `postFilterableFields` with priority deduplication (`native` > `foldable` > `postFilterable` > `unsupported`).
- Updated the entire `ACTOR_CAPABILITIES` matrix with the specified four-tier capabilities for all 11 sites.
- Added `fieldCapabilityForSite()` returning the exact capability for a site/field pair.
- Updated `supportsHuntFieldForSite()` to return `true` for any capability except `unsupported`.
- Added `bestCapabilityForSites()` using priority ranking across all selected visible sites.
- Added `fieldsByBestCapabilityForSites()` putting each `HUNT_FIELDS` entry into one priority bucket.
- Updated `ignoredFieldsForSites()` to use `bestCapabilityForSites`, so only fields with best capability `unsupported` are treated as ignored.
- Added `fieldHintForSites()` with full priority: all unsupported → `ignored`, some unsupported → `partially ignored`, best=`postFilterable` → `post-filtered after fetch`, best=`foldable` → `folded into search`, best=`native` → `null`.
- Updated `HuntBriefPanel.fieldLabel()` to use `fieldHintForSites()` for all hint rendering (removed `ignoredReason`).
- Updated `generateHuntFiles()` debug text to the exact format: `native: <fields|none> · folded: <fields|none> · post-filtered: <fields|none> · ignored: <fields|none>`.

### Styles (`src/styles.css`)
- Added `.field-hint.postfiltered` rule with `color: var(--accent)` and `font-style: normal`.

### Backend (`src-tauri/src/lib.rs`)
- Added `ActorFieldCapability` enum with `Native`, `Foldable`, `PostFilterable`, `Unsupported` variants (derives `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`).
- Added `actor_field_capability(site, field)` using the same four-tier matrix as the frontend.
- Updated `actor_supports_field(site, field)` to return `true` only for `Native` and `Foldable` (preserving API-support semantics).
- Added `actor_can_filter_field(site, field)` returning `true` for `Native`, `Foldable`, or `PostFilterable`.
- Refactored `effective_hunt_for_site` into `effective_hunt_for_site_for_api` (uses `actor_supports_field`) and `effective_hunt_for_site_for_post_filter` (uses `actor_can_filter_field`), with shared `apply_effective_fields` helper.
- Updated `effective_hunt_for_sites` to use `actor_can_filter_field` so run-level settings neutralize only when ALL selected sites are `Unsupported`.
- In `start_hunt_apify` actor loop: computes both `api_hunt` and `post_filter_hunt`, passes `api_hunt` to `run_actor_api`, emits `capability-adjusted fields` debug event with both API and post-filter effective fields plus categorized field arrays.
- In `start_hunt_apify` post-filter loop: uses `effective_hunt_for_site_for_post_filter` before calling `post_filter_reason`.
- Replaced the old `ignored` vector construction with full capability categorization arrays (`nativeFields`, `foldableFields`, `postFilterableFields`, `ignoredFields`).

## Files Changed

- `src/main.tsx` - Capability types, `caps()` helper, `ACTOR_CAPABILITIES` matrix, all new capability helpers (`fieldCapabilityForSite`, `bestCapabilityForSites`, `fieldsByBestCapabilityForSites`, `fieldHintForSites`), updated `supportsHuntFieldForSite`, `ignoredFieldsForSites`, `neutralizeUnsupportedHuntSettingsForSites`, `HuntBriefPanel.fieldLabel`, `generateHuntFiles` debug text
- `src/styles.css` - Added `.field-hint.postfiltered` rule
- `src-tauri/src/lib.rs` - Added `ActorFieldCapability` enum, `actor_field_capability`, `actor_can_filter_field`, `apply_effective_fields`, `effective_hunt_for_site_for_api`, `effective_hunt_for_site_for_post_filter`; updated `actor_supports_field`, `effective_hunt_for_sites`, `start_hunt_apify` actor loop and post-filter loop

## Verification

- `cargo check` on `src-tauri/Cargo.toml`: **0 errors, no new warnings** (4 pre-existing warnings in unchanged code).
- `vite build`: **Build succeeds** (2 pre-existing TypeScript errors on `minSalary` type and `Record<string, string>` cast — unchanged by this work).
- `npm run build`: **Build succeeds**.

## Blockers

None.

## Observations

- Two pre-existing TypeScript errors (`TS2678` on `minSalary`, `TS2352` on the `as Record<string, string>` cast) exist in `neutralizeUnsupportedHuntSettingsForSites`. These were not introduced by this change and do not affect the Vite production build.
- The `create_hunt_run` function in Rust uses `effective_hunt_for_sites` (which now considers `postFilterable` as usable), so the initial `results.md` will correctly show post-filterable values in the hunt settings display rather than neutralizing them.
