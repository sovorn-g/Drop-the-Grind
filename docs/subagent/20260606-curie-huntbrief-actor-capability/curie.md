## Scope Investigated

- Full HuntBrief actor capability architecture: frontend types/caps, backend `actor_supports_field`, `effective_hunt_for_site(s)`, `build_actor_input`, `post_filter_reason`, all 11 actor normalizers.
- Did **not** investigate: the `normalize_generic` fallback path, the import/upload flow, Codex chat path, Tauri command wiring beyond `start_hunt_apify`.

## Findings

### 1. Current capability model is binary (API-only) — missing post-filter awareness

- `type ActorCapabilities` in `src/main.tsx#L51`  
  Only has `nativeFields`, `foldableFields`, `unsupportedFields` (computed as remainder). No concept of "post-filterable."
- `fn actor_supports_field` in `src-tauri/src/lib.rs#L809`  
  Mirrors frontend: returns true only for native or foldable. No third category.
- `fn caps(...)` in `src/main.tsx#L52`  
  Only accepts `nativeFields` and optional `foldableFields`. No third array.

**Relevance**: This is the root structure that must be expanded to support the desired four-category model (native / foldable / postFilterable / unsupported).

### 2. UI disable logic is driven by `ignoredFieldsForSites`

- `ignoredFieldsForSites` in `src/main.tsx#L74-L76`  
  A field is "ignored" if NO selected site supports it (`supportsHuntFieldForSite` checks native+folder only).
- `isIgnored` in `src/main.tsx#L264`  
  Used directly as `disabled` prop on every filter input in `HuntBriefPanel`.
- `ignoredReason` in `src/main.tsx#L265-L268`  
  Shows `"ignored by X"` hint. No distinction between pre-filter/post-filter/unsupported.

**Relevance**: To stop disabling post-filterable fields, `ignoredFieldsForSites` must only return truly unsupported fields.

### 3. Backend neutralizes values for "unsupported" fields — blocking post-filtering

- `effective_hunt_for_site` in `src-tauri/src/lib.rs#L835`  
  For each field not in `actor_supports_field`, calls `neutral_value_for_field` which sets `"Any"`, `"Worldwide"`, `"Standard"`, or empty string.
- `effective_hunt_for_sites` in `src-tauri/src/lib.rs#L856`  
  Similarly neutralizes at the multi-site level.

**Relevance**: Post-filterable fields must **not** be neutralized. Their user values must survive through to `post_filter_reason`.

### 4. `post_filter_reason` already checks all fields — ready for post-filterable data

- `post_filter_reason` in `src-tauri/src/lib.rs#L1590`  
  Checks: excludeKeywords, workMode, location, postedWithin, salary, seniority, experience.  
  It already works correctly if values are not neutralized.

**Relevance**: No changes needed to the post-filter logic itself.

### 5. Every actor normalizer produces at least some structured output fields

All 11 normalizers (in `src-tauri/src/lib.rs#L1273-L1456`) were inspected. Key findings:

| Actor | Output fields that enable post-filtering |
|---|---|
| 54 Career Sites | `seniority` (ai_experience_level), `salary` (ai_salary_minvalue/maxvalue), `posted_date` |
| LinkedIn | Same as 54 Career Sites (reuses normalizer) |
| Indeed | `seniority` (level/jobLevel), `salary` (estimatedSalary), `posted_date`; experience is **empty** |
| Wellfound | `work_mode` (remote boolean), `salary` (compensation), `posted_date`; experience is **empty** |
| **YC Startup Jobs** | `seniority` ← from actor `experience` field, `salary` ← from `salaryRange`, `posted_date` ← from `datePosted` |
| Welcome to the Jungle | `work_mode` (remote boolean), `salary`; seniority/experience are **empty** |
| HiringCafe | `seniority`, `experience`, `salary` (compensation), `posted_date` (estimated_publish_date) |
| We Work Remotely | `work_mode` (always "Remote"), `salary`, `posted_date`; seniority/experience are **empty** |
| FlexJobs | `seniority` (levels), `salary`, `posted_date`; experience is **empty** |
| Himalayas | `seniority` (experience_level), `work_mode`, `salary` (salary_min/max); experience is **empty** |
| Remotive | `work_mode` (always "Remote"), `salary`, `posted_date`; seniority/experience are **empty** |

Additionally, **`excludeKeywords` is universally post-filterable** for all 11 actors because `job_search_text` (in `src-tauri/src/lib.rs#L1483`) checks `title`, `company`, `location`, `description`, `requirements`, `skills`, and `seniority` — all normalizers produce at least title, company, and description.

**Actors that never produce `experience` output**: Indeed, Wellfound, YC Startup Jobs, Welcome to the Jungle, We Work Remotely, FlexJobs, Himalayas, Remotive (8 of 11).

### 6. `build_actor_input` only uses native+foldable fields — no change needed

`build_actor_input` in `src-tauri/src/lib.rs#L893` already constructs API input per-site using only the fields that actor accepts. Post-filterable fields are naturally excluded from the API payload.

### 7. Debug logging in `start_hunt_apify` labels everything as "ignored"

`start_hunt_apify` in `src-tauri/src/lib.rs#L1040-L1045` iterates fields and logs those not in `actor_supports_field` as `ignoredFields`. Should be updated to categorize native/folded/post-filter/ignored.

### 8. Rust `actor_supports_field` mirrors frontend caps exactly

`actor_supports_field` in `src-tauri/src/lib.rs#L809` has a hardcoded per-site match identical to the frontend `ACTOR_CAPABILITIES`. Both must be updated in sync.

## Relationships

- **Frontend caps → UI disabled state**: `ACTOR_CAPABILITIES` → `supportsHuntFieldForSite` → `ignoredFieldsForSites` → `isIgnored` → `disabled` prop on all 9 filter inputs in `HuntBriefPanel`.
- **Frontend caps → Backend caps**: Must stay in sync. Frontend `ACTOR_CAPABILITIES` and Rust `actor_supports_field` are independent code blocks that encode the same data.
- **Backend caps → value neutralization**: `actor_supports_field` → `effective_hunt_for_site/sites` → decides which fields get neutralized via `neutral_value_for_field`.
- **Backend caps → API input**: `build_actor_input` uses per-site logic (not `actor_supports_field` directly) — already correct.
- **Backend caps → logging**: Used to build `ignored` list for debug events in `start_hunt_apify`.
- **Normalizer output → post-filter**: `normalize_hunt_job` → `normalize_*` site-specific → fills `HuntJob` fields → `post_filter_reason` reads them.

## Open Questions / Gaps

- **`experience` field**: 8 of 11 actors set `experience: String::new()` in their normalizer. The `experience_matches` function checks `j.experience` + `j.requirements`. For actors that DO produce `requirements` (54 Career Sites, HiringCafe), experience post-filtering might partially work via requirements text. But for others, experience remains genuinely unsupported. Confirm whether "unsupported" vs "post-filterable via requirements" is the right classification.
- **`seniority_matches` checks `j.title`**: Even actors with empty `j.seniority` in their normalizer could theoretically post-filter by scanning title text for "senior," "lead," etc. This means seniority could be classified as post-filterable for more actors than listed above. Need design decision: is title-only seniority matching reliable enough to enable the field?
- **Multi-site runs**: When sites have mixed capabilities (some native, some post-filterable, some unsupported), `effective_hunt_for_sites` currently neutralizes if NO site supports a field. With post-filterable fields, this logic needs careful adjustment — should neutralize only if ALL sites are unsupported (not post-filterable), but keep values if at least one site is post-filterable or native.
- **Frontend hint text**: The `ignoredReason` currently shows a single "ignored by X" message. The design for per-field hints (pre-filter / folded / post-filter / ignored) needs specification — what exact text to show per site and per field.

## Start Here

1. **`src/main.tsx#L49-L76`** — Update `HuntField`, `ActorCapabilities` type, `caps()` function, and `ignoredFieldsForSites()`. Add `postFilterableFields` to the type and adjust the ignore logic.
2. **`src-tauri/src/lib.rs#L809-L880`** — Update `actor_supports_field` (or replace with category-aware function), update `effective_hunt_for_site` and `effective_hunt_for_sites` to not neutralize post-filterable fields.
