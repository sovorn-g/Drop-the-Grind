# Reflection Review – HuntBrief Actor Field Capabilities

## Scope Reviewed

- **Plan**: `docs/subagent/20260606-curie-huntbrief-actor-capability/davinci.md`
- **Discovery**: `docs/subagent/20260606-curie-huntbrief-actor-capability/curie.md`
- **Source code verified**:
  - `src/main.tsx`: `ActorCapabilities` type, `caps()`, `ACTOR_CAPABILITIES`, `supportsHuntFieldForSite`, `supportedFieldsForSites`, `ignoredFieldsForSites`, `neutralizeUnsupportedHuntSettingsForSites`, `HuntBriefPanel` (`isIgnored`, `ignoredReason`, `fieldLabel`), `generateHuntFiles` debug text
  - `src-tauri/src/lib.rs`: `actor_supports_field`, `neutral_value_for_field`, `effective_hunt_for_site`, `effective_hunt_for_sites`, `build_actor_input`, `run_actor_api`, `post_filter_reason`, `seniority_matches`, `experience_matches`, `start_hunt_apify` (API call + post-filter call sites), all 11 normalizers
  - `src/styles.css`: `.field-hint` class variants
- **Not reviewed**: `run_actor_api` internals beyond confirming it passes `h` to `build_actor_input`; Tauri command wiring; `create_hunt_run` backend beyond its input parameters; frontend `Channel`/event plumbing.

## 🔴 Blocking Issues

### 1. Shared `site_hunt` dual-use creates a correctness dilemma that the plan does not address

In `start_hunt_apify`, `effective_hunt_for_site(&effective, site)` is called in **two separate places**:

- **Line ~1039** — `site_hunt` is passed to `run_actor_api`, which calls `build_actor_input(site, &site_hunt)` to construct the API payload.
- **Line ~1080** — `site_hunt` is passed to `post_filter_reason(&site_hunt, &norm)`.

The plan updates `actor_supports_field` to return `true` for post-filterable fields (TODO 11) and says `effective_hunt_for_site` should then "neutralize only truly unsupported fields" (TODO 12). But this creates an unsolvable conflict with a single shared `site_hunt`:

**If post-filterable fields are NOT neutralized** (to make post-filter work):

Several actors' `build_actor_input` functions **directly reference** fields the plan classifies as post-filterable. These actors would start receiving API parameters they never received before:

| Actor | Post-filterable field | `build_actor_input` usage | Current behavior | After change |
|---|---|---|---|---|
| 54 Career Sites | `salary` | `"minSalary": min_salary_num(h)` | Neutralized → `"Any"` → no salary filter sent | **Real value sent as `minSalary`** |
| Wellfound | `salary` | `"minSalary": min_salary_num(h)` | Neutralized → no salary filter | **Real value sent** |
| Himalayas | `salary` | `"minSalary": min_salary_num(h)` | Neutralized → no salary filter | **Real value sent** |
| Remotive | `salary` | `"requireSalary": min_salary_num(h) > 0, "minSalary": min_salary_num(h)` | Neutralized → `false`/no filter | **Real value sent** |

This is a breaking change to API behavior. Actors that previously ignored salary would start filtering by it at the API level, potentially returning zero results or wrong results.

**If post-filterable fields ARE neutralized** (to keep API behavior safe):

`post_filter_reason` receives neutral values for post-filterable fields. Post-filtering by salary, experience, etc. becomes a no-op. **The entire feature is non-functional.**

**What the plan must do**: Either (a) split `site_hunt` into an API-neutralized version and a post-filter-preserving version, or (b) update each affected `build_actor_input` to guard against post-filterable fields it shouldn't send. The plan's TODO list has no step for either approach.

## 🟡 Should Fix

### 2. No CSS class for "post-filtered" hint

TODO 6 introduces a `fieldHintForSites` returning `{ text, className }`. The new hint text "post-filtered after fetch" needs a visual style distinct from `.folded` (italic), `.ignored` (orange), and `.plain` (subtle). The plan assumes "Existing CSS classes … are sufficient" but none semantically matches "post-filtered." At minimum, add a `.field-hint.postfiltered` rule (e.g., a distinct color or icon treatment) to differentiate it from plain hints. Without this, the implementer must guess at styling.

### 3. Frontend debug text format is unspecified

TODO 8 says to "describe `enabled fields` and `ignored fields` from the full capability model." The current debug text (`effective fields: … · ignored fields: …`) uses `supportedFieldsForSites` which will now include post-filterable fields. The plan doesn't specify whether to break this into subcategories (e.g., "native: … · folded: … · post-filtered: … · ignored: …") or keep the two-group format. Since the entire point of this change is distinguishing these categories, the debug text format should be explicit.

### 4. `experience` classification for Indeed is inconsistent with stated reasoning

The plan's Assumptions section says:

> `experience` is post-filterable only where existing normalized fields provide `j.experience` or `j.requirements`: `LinkedIn` and `HiringCafe`

But `experience_matches` (`src-tauri/src/lib.rs:1538-1547`) checks `format!("{} {}", j.experience, j.requirements.join(" "))`. Indeed's normalizer sets `experience: String::new()` but also `requirements: vec![]`. However, 54 Career Sites sets `requirements: listify(v, &["ai_requirements_summary"])` and LinkedIn reuses that normalizer — so LinkedIn gets requirements.

The issue: if Indeed's `requirements` is always `vec![]` (confirmed in source), then Indeed experience post-filtering would be a no-op anyway (empty text → returns true). So classifying Indeed as `unsupported` is behaviorally harmless. But the stated reasoning ("post-filterable only where … `j.experience` or `j.requirements`") is correct and should exclude Indeed. The TODO matrix already does exclude Indeed, so the matrix is right but the reasoning text could be misread as including it. Clarify the reasoning to say: "experience is post-filterable only for LinkedIn and HiringCafe because those are the only actors whose normalizers produce non-empty `j.experience` or `j.requirements`."

## 💡 Optional Suggestions

### 5. `fieldHintForSites` priority has an unaddressed edge case

When some sites are `unsupported` and others are `native` (but not post-filterable or foldable), the priority falls through to `null`. The user sees the field enabled with no explanation. Consider adding a priority case: "some unsupported + some native/foldable → `partially supported: ignored by <sites>`." This is a minor UX gap.

### 6. Debug payload in `start_hunt_apify` already uses `ignoredFields`

TODO 14 adds `nativeFields`, `foldableFields`, `postFilterableFields` to the debug payload but says "keep `ignoredFields` as the unsupported-only list." The current code at line ~1042 builds `ignored` from `!actor_supports_field(site, f)`. After TODO 11, this list changes meaning. Consider explicitly noting that the existing `ignored` variable construction at line ~1042 must be replaced with capability-aware categorization, not just the TODO 14 debug payload.

## ✅ What Is Solid

- **Matrix accuracy**: The TODO 3 capability matrix correctly maps each actor's normalizer output fields to post-filterable categories. Verified against all 11 normalizer functions in `src-tauri/src/lib.rs`.
- **Frontend type/helper changes (TODOs 1-7)**: The progression from type expansion → `caps()` update → matrix update → helper functions → UI integration is well-ordered and deterministic. An implementer can follow it sequentially.
- **Multi-site neutralization at `effective_hunt_for_sites` level**: The plan correctly identifies that `effective_hunt_for_sites` neutralizes only when ALL sites are unsupported. Updating `actor_supports_field` to return `true` for post-filterable fields correctly preserves values when at least one site supports any capability. This part is sound.
- **`build_actor_input` exclusion principle**: The plan correctly states "Leave `build_actor_input` unchanged; post-filterable fields must not be sent to actor APIs unless already handled." The principle is right — but as noted in Blocking Issue 1, the plan doesn't provide the mechanism to enforce it given the shared `site_hunt` architecture.
- **Scope boundaries**: The plan correctly excludes normalizer changes, post-filter predicate changes, and DB/storage changes. These are genuinely unnecessary.
- **Frontend neutralization preservation**: `neutralizeUnsupportedHuntSettingsForSites` correctly preserves post-filterable values for backend transmission after TODO 5's update.

## ⚖️ Verdict

REVISE: revise the implementation plan before implementation.
