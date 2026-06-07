# Revised Plan – HuntBrief Actor Field Capabilities

## What

Update HuntBrief field capabilities from binary API support to `native`, `foldable`, `postFilterable`, and `unsupported`, while keeping actor API inputs safe and preserving post-filterable values for backend filtering.

## How

- Add category-aware capability helpers in `src/main.tsx` and `src-tauri/src/lib.rs`.
- In the frontend, enable a field when any selected visible site has `native`, `foldable`, or `postFilterable` capability.
- In the backend, split per-site settings into two deterministic views:
  - API view: neutralizes `postFilterable` and `unsupported` fields before `build_actor_input`.
  - Post-filter view: neutralizes only `unsupported` fields before `post_filter_reason`.
- Keep `build_actor_input` unchanged except for receiving the API-neutralized `HuntRunInput`.
- Keep `post_filter_reason` unchanged except for receiving the post-filter-preserving `HuntRunInput`.

**Scope**:
- In scope: `src/main.tsx` capability types/helpers/matrix, disabled-field logic, field hints, Start Hunting debug text; `src/styles.css` field hint styling; `src-tauri/src/lib.rs` capability helpers/matrix, effective settings neutralization, API/post-filter settings split, per-source debug payload.
- Out of scope: actor input schema changes, normalizer changes, post-filter predicate changes, raw result output, DB/storage changes, Codex/MCP behavior.
- Scope assumptions: no new shared config file; keep the duplicated TypeScript/Rust capability matrix because the current app already duplicates it.

**Assumptions**:
- `seniority` is post-filterable for non-native actors with normalized `title` because existing `seniority_matches` intentionally checks `j.title`.
- `experience` is post-filterable only for `LinkedIn` and `HiringCafe` because those are the only non-native actors whose normalizers produce non-empty `j.experience` or `j.requirements`; `Indeed` remains unsupported for `experience`.
- A field that is unsupported by some selected sites and usable by others remains enabled, but the UI must show partial support text.

**Reuses**:
- `HUNT_FIELDS`, `caps`, `ignoredFieldsForSites`, `neutralizeUnsupportedHuntSettingsForSites` from `src/main.tsx`.
- `.field-hint`, `.field-hint.folded`, `.field-hint.ignored`, `.field-hint.plain` from `src/styles.css`.
- `actor_supports_field`, `neutral_value_for_field`, `effective_hunt_for_site`, `effective_hunt_for_sites`, `build_actor_input`, `run_actor_api`, `post_filter_reason` from `src-tauri/src/lib.rs`.
- Existing normalized `HuntJob` fields used by filters: `title`, `work_mode`, `seniority`, `experience`, `salary`, `posted_date`, `requirements`, `skills`, `description`.

**Review fixes applied**:
- Fixed the blocking `site_hunt` dual-use issue by requiring separate API-neutralized and post-filter-preserving per-site settings.
- Added explicit `.field-hint.postfiltered` styling.
- Specified exact frontend debug text categories.
- Clarified `experience` post-filterability excludes `Indeed`.
- Added partial-support hint behavior for mixed selected-site capabilities.
- Required replacement of the current backend `ignored` variable construction with capability-aware categorization.

## TODO

1. Update `src/main.tsx` `ActorCapabilities` to include `postFilterableFields: HuntField[]` and derive `unsupportedFields` from fields absent from `nativeFields`, `foldableFields`, and `postFilterableFields`.
2. Update `src/main.tsx` `caps(nativeFields, foldableFields = [], postFilterableFields = [])` so each field is assigned by priority `native` > `foldable` > `postFilterable` > `unsupported`; filter duplicates out of lower-priority arrays.
3. Update `src/main.tsx` `ACTOR_CAPABILITIES` to this matrix:
   - `54 Career Sites`: native `roles, seniority, experience, location, workMode, postedWithin, includeKeywords, excludeKeywords`; foldable none; post-filterable `salary`.
   - `LinkedIn`: native `roles, seniority, location, workMode, postedWithin, includeKeywords, excludeKeywords`; foldable none; post-filterable `experience, salary`.
   - `Indeed`: native `roles, seniority, location, workMode, postedWithin`; foldable `includeKeywords`; post-filterable `salary, excludeKeywords`.
   - `Wellfound`: native `roles, seniority, salary, location, workMode, postedWithin, includeKeywords`; foldable none; post-filterable `excludeKeywords`.
   - `YC Startup Jobs`: native `roles, location, workMode`; foldable `includeKeywords`; post-filterable `seniority, salary, excludeKeywords, postedWithin`.
   - `Welcome to the Jungle`: native `roles, location, postedWithin`; foldable `includeKeywords`; post-filterable `seniority, salary, excludeKeywords, workMode`.
   - `HiringCafe`: native `roles, location, workMode`; foldable `includeKeywords`; post-filterable `seniority, experience, salary, excludeKeywords, postedWithin`.
   - `We Work Remotely`: native `roles, location, salary`; foldable `includeKeywords`; post-filterable `seniority, workMode, excludeKeywords, postedWithin`.
   - `FlexJobs`: native `roles, location, workMode, seniority`; foldable `includeKeywords`; post-filterable `salary, excludeKeywords, postedWithin`.
   - `Himalayas`: native `roles, seniority, location, postedWithin, includeKeywords`; foldable none; post-filterable `salary, workMode, excludeKeywords`.
   - `Remotive`: native `roles, location, salary, postedWithin`; foldable `includeKeywords`; post-filterable `seniority, workMode, excludeKeywords`.
4. Add `src/main.tsx` type `ActorFieldCapability = 'native' | 'foldable' | 'postFilterable' | 'unsupported'` and helper `fieldCapabilityForSite(site: string, field: HuntField): ActorFieldCapability`.
5. Update `src/main.tsx` `supportsHuntFieldForSite` to return true for every capability except `unsupported`.
6. Add `src/main.tsx` helper `bestCapabilityForSites(sites: string[], field: HuntField): ActorFieldCapability` using priority `native` > `foldable` > `postFilterable` > `unsupported`, returning `unsupported` when `sites` is non-empty and all selected visible sites are unsupported.
7. Add `src/main.tsx` helper `fieldsByBestCapabilityForSites(sites: string[]): Record<ActorFieldCapability, HuntField[]>` that puts each `HUNT_FIELDS` entry into exactly one best-capability array.
8. Update `src/main.tsx` `supportedFieldsForSites`, `ignoredFieldsForSites`, and `neutralizeUnsupportedHuntSettingsForSites` so `postFilterable` counts as usable and only fields with best capability `unsupported` are neutralized; for `salary`, continue writing the neutral value to `out.minSalary`.
9. Add `src/main.tsx` helper `fieldHintForSites(sites: string[], field: HuntField): { text: string; className: 'plain' | 'folded' | 'postfiltered' | 'ignored' } | null` with this priority: all unsupported -> `ignored by <site>` for one site or `ignored by selected sites` for multiple sites, class `ignored`; some unsupported -> `partially ignored by <comma-separated unsupported sites>`, class `plain`; any post-filterable -> `post-filtered after fetch`, class `postfiltered`; any foldable -> `folded into search`, class `folded`; otherwise `null`.
10. Update `src/main.tsx` `HuntBriefPanel` `isIgnored`, `ignoredReason`, and `fieldLabel` so inputs are disabled only by `ignoredFieldsForSites(selectedVisibleSites)`, and labels render `fieldHintForSites(selectedVisibleSites, field)` as `<span className={`field-hint ${hint.className}`}> ({hint.text})</span>`.
11. Update `src/main.tsx` `generateHuntFiles` debug text to use `fieldsByBestCapabilityForSites(visibleSites)` and this exact category format: `native: <fields|none> · folded: <fields|none> · post-filtered: <fields|none> · ignored: <fields|none>`.
12. Add `src/styles.css` rule `.field-hint.postfiltered` using the existing design tokens, with `color: var(--accent)` and non-italic text.
13. Add `src-tauri/src/lib.rs` enum `ActorFieldCapability { Native, Foldable, PostFilterable, Unsupported }` near the current HuntBrief capability helpers; derive `Copy`, `Clone`, `Debug`, `PartialEq`, and `Eq`.
14. Add `src-tauri/src/lib.rs` function `actor_field_capability(site: &str, field: &str) -> ActorFieldCapability` using the same matrix as `src/main.tsx`.
15. Update `src-tauri/src/lib.rs` `actor_supports_field(site, field)` to preserve its current API-support meaning: return true only for `Native` or `Foldable`, and false for `PostFilterable` or `Unsupported`.
16. Add `src-tauri/src/lib.rs` function `actor_can_filter_field(site, field)` that returns true for `Native`, `Foldable`, or `PostFilterable`, and false for `Unsupported`.
17. Refactor `src-tauri/src/lib.rs` `effective_hunt_for_site` into `effective_hunt_for_site_for_api(h, site)` and `effective_hunt_for_site_for_post_filter(h, site)`; the API function uses `actor_supports_field`, and the post-filter function uses `actor_can_filter_field`.
18. Update `src-tauri/src/lib.rs` `effective_hunt_for_sites` to use `actor_can_filter_field` so run-level settings neutralize a field only when all selected sites are `Unsupported`.
19. Update `src-tauri/src/lib.rs` `start_hunt_apify` actor loop to compute `api_hunt = effective_hunt_for_site_for_api(&effective, site)` and `post_filter_hunt = effective_hunt_for_site_for_post_filter(&effective, site)`; pass `api_hunt` to `run_actor_api` and use `post_filter_hunt` only for debug payload values that describe post-filter-preserved settings.
20. Update `src-tauri/src/lib.rs` post-filter loop to call `effective_hunt_for_site_for_post_filter(&effective, source)` before `post_filter_reason`.
21. Update `src-tauri/src/lib.rs` `start_hunt_apify` debug categorization to replace the existing `ignored` construction with capability-aware arrays `nativeFields`, `foldableFields`, `postFilterableFields`, and `ignoredFields`; use `ignoredFields` only for `Unsupported` fields.
22. Update `src-tauri/src/lib.rs` `start_hunt_apify` debug event text from `pre-API effective fields` to `capability-adjusted fields`, and include both `apiEffectiveFields` from `api_hunt` and `postFilterEffectiveFields` from `post_filter_hunt` in the payload.

## Outcome

- Post-filterable fields remain enabled in HuntBrief and are labeled as post-filtered.
- Actor API payloads do not start receiving newly post-filterable values by accident.
- Backend post-filtering receives preserved values for `postFilterable` fields and neutral values for truly unsupported fields.
- Multi-site runs keep a field active when at least one selected visible site can use it through any capability category.
- Frontend and backend debug output clearly separates native, folded, post-filtered, and ignored fields.
