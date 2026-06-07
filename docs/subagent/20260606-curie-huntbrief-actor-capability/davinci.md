# Plan – HuntBrief Actor Field Capabilities

## What

Update HuntBrief actor field capability handling from API-only support to four categories: `native`, `foldable`, `postFilterable`, and `unsupported`. Use the full capability model to decide disabled frontend fields, preserve post-filterable values for backend filtering, and categorize backend debug logging.

## How

- Keep `src/main.tsx` and `src-tauri/src/lib.rs` capability matrices synchronized.
- Treat fields as enabled when any selected site has `native`, `foldable`, or `postFilterable` capability.
- Neutralize values only when every selected site is `unsupported` for that field.
- Leave `build_actor_input` unchanged; post-filterable fields must not be sent to actor APIs unless already handled by existing per-actor input code.
- Leave `post_filter_reason` unchanged; it already applies the post-fetch checks when values survive neutralization.

**Scope**:
- In scope: `src/main.tsx` capability types/helpers/matrix, disabled-field logic, field hints, HuntBrief debug text; `src-tauri/src/lib.rs` capability helper/matrix, effective settings neutralization, per-source debug payload.
- Out of scope: actor input schema changes, normalizer changes, post-filter predicate changes, raw result output, DB/storage changes.
- Scope assumptions: no new shared config file; duplicate the matrix in TypeScript and Rust as the current code already does.

**Assumptions**:
- `seniority` is post-filterable for non-native actors with normalized `title` because existing `seniority_matches` intentionally checks `j.title`.
- `experience` is post-filterable only where existing normalized fields provide `j.experience` or `j.requirements`: `LinkedIn` and `HiringCafe`; `54 Career Sites` remains `native` for `experience`.
- Existing CSS classes `.field-hint`, `.field-hint.folded`, `.field-hint.ignored`, and `.field-hint.plain` are sufficient for the new hints.

**Reuses**:
- `HUNT_FIELDS`, `caps`, `ignoredFieldsForSites`, `neutralizeUnsupportedHuntSettingsForSites` from `src/main.tsx`.
- `actor_supports_field`, `neutral_value_for_field`, `effective_hunt_for_site`, `effective_hunt_for_sites`, `post_filter_reason` from `src-tauri/src/lib.rs`.
- Existing normalizer output fields in `src-tauri/src/lib.rs`: `HuntJob.title`, `work_mode`, `seniority`, `experience`, `salary`, `posted_date`, `requirements`, `skills`, `description`.

## TODO

1. Update `src/main.tsx` `ActorCapabilities` to include `postFilterableFields: HuntField[]` and keep `unsupportedFields: HuntField[]` derived from all three usable categories.
2. Update `src/main.tsx` `caps(nativeFields, foldableFields, postFilterableFields)` so category priority is `nativeFields` > `foldableFields` > `postFilterableFields` > `unsupportedFields`, with no duplicate field in a lower-priority derived category.
3. Update `src/main.tsx` `ACTOR_CAPABILITIES` to this matrix:
   - `54 Career Sites`: native `roles,seniority,experience,location,workMode,postedWithin,includeKeywords,excludeKeywords`; foldable none; post-filterable `salary`.
   - `LinkedIn`: native `roles,seniority,location,workMode,postedWithin,includeKeywords,excludeKeywords`; foldable none; post-filterable `experience,salary`.
   - `Indeed`: native `roles,seniority,location,workMode,postedWithin`; foldable `includeKeywords`; post-filterable `salary,excludeKeywords`.
   - `Wellfound`: native `roles,seniority,salary,location,workMode,postedWithin,includeKeywords`; foldable none; post-filterable `excludeKeywords`.
   - `YC Startup Jobs`: native `roles,location,workMode`; foldable `includeKeywords`; post-filterable `seniority,salary,excludeKeywords,postedWithin`.
   - `Welcome to the Jungle`: native `roles,location,postedWithin`; foldable `includeKeywords`; post-filterable `seniority,salary,excludeKeywords,workMode`.
   - `HiringCafe`: native `roles,location,workMode`; foldable `includeKeywords`; post-filterable `seniority,experience,salary,excludeKeywords,postedWithin`.
   - `We Work Remotely`: native `roles,location,salary`; foldable `includeKeywords`; post-filterable `seniority,workMode,excludeKeywords,postedWithin`.
   - `FlexJobs`: native `roles,location,workMode,seniority`; foldable `includeKeywords`; post-filterable `salary,excludeKeywords,postedWithin`.
   - `Himalayas`: native `roles,seniority,location,postedWithin,includeKeywords`; foldable none; post-filterable `salary,workMode,excludeKeywords`.
   - `Remotive`: native `roles,location,salary,postedWithin`; foldable `includeKeywords`; post-filterable `seniority,workMode,excludeKeywords`.
4. Add `src/main.tsx` helper type `ActorFieldCapability = 'native' | 'foldable' | 'postFilterable' | 'unsupported'` and helper `fieldCapabilityForSite(site: string, field: HuntField): ActorFieldCapability`.
5. Update `src/main.tsx` `supportsHuntFieldForSite`, `supportedFieldsForSites`, `ignoredFieldsForSites`, and `neutralizeUnsupportedHuntSettingsForSites` so `postFilterable` counts as supported and only `unsupported` fields are neutralized.
6. Add `src/main.tsx` helper `fieldHintForSites(sites: string[], field: HuntField): { text: string; className: string } | null` using this priority: all unsupported -> `ignored by <site>` or `ignored by selected sites`; some unsupported -> `ignored by <comma-separated unsupported sites>`; any post-filterable -> `post-filtered after fetch`; any foldable -> `folded into search`; otherwise `null`.
7. Update `src/main.tsx` `HuntBriefPanel` `isIgnored`, `ignoredReason`, and `fieldLabel` to use `fieldHintForSites`; disable inputs only when `ignoredFieldsForSites(selectedVisibleSites)` contains the field.
8. Update `src/main.tsx` `generateHuntFiles` debug text to describe `enabled fields` and `ignored fields` from the full capability model.
9. Add `src-tauri/src/lib.rs` enum `ActorFieldCapability { Native, Foldable, PostFilterable, Unsupported }` near the current capability helpers.
10. Add `src-tauri/src/lib.rs` function `actor_field_capability(site: &str, field: &str) -> ActorFieldCapability` using the same matrix as `src/main.tsx`.
11. Update `src-tauri/src/lib.rs` `actor_supports_field(site, field)` to return `actor_field_capability(site, field) != ActorFieldCapability::Unsupported`.
12. Update `src-tauri/src/lib.rs` `effective_hunt_for_site` and `effective_hunt_for_sites` to rely on the updated `actor_supports_field`; verify their behavior now neutralizes only truly unsupported fields.
13. Add `src-tauri/src/lib.rs` helper `field_capability_label(capability: ActorFieldCapability) -> &'static str` only if needed for debug payload construction.
14. Update `src-tauri/src/lib.rs` `start_hunt_apify` debug payload to include `nativeFields`, `foldableFields`, `postFilterableFields`, and `ignoredFields` for each site; keep `ignoredFields` as the unsupported-only list.
15. Update `src-tauri/src/lib.rs` `start_hunt_apify` debug event text if needed so `pre-API effective fields` does not imply post-filterable fields were sent to the actor API.

## Outcome

- Post-filterable fields remain enabled in HuntBrief instead of being disabled as ignored.
- Frontend run settings preserve post-filterable values and neutralize only fields unsupported by all selected sites.
- Backend per-site settings preserve post-filterable values for `post_filter_reason` and still exclude them from actor API input unless existing actor input logic uses them.
- Debug logs distinguish native, folded, post-filtered, and ignored fields.
- Multi-site runs keep a field active when at least one selected site can use it through any capability category.
