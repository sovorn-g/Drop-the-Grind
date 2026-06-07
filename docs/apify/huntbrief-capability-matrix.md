# HuntBrief actor capability matrix

Last updated: 2026-06-06

This matrix drives whether HuntBrief UI fields are enabled. Supported fields render as normal inputs with no UI label. A field is disabled only when every selected visible actor is `unsupported` for that field.

Legend:

- `native` — sent directly to the actor API input.
- `folded` — folded into an actor search/query/keyword input.
- `structured-post` — post-filtered from normalized structured output fields such as salary/date/work mode.
- `text-post` — post-filtered from title/description/requirements text; useful but approximate.
- `unsupported` — not reliably usable.

| Actor | Roles | Seniority | Experience | Salary | Include keywords | Avoid keywords | Posted within | Location | Work mode |
|---|---|---|---|---|---|---|---|---|---|
| 54 Career Sites | native | text-post | native | structured-post | native | native | native | native | native |
| LinkedIn | native | native | native | structured-post | native | native | native | native | native |
| Indeed | native | native | text-post | structured-post | folded | text-post | native | native | native |
| YC Startup Jobs | native | text-post | text-post | structured-post | folded | text-post | structured-post | native | native |
| Welcome to the Jungle | native | text-post | text-post | text-post | folded | text-post | native | native | structured-post |
| HiringCafe | native | structured-post | structured-post | structured-post | folded | text-post | structured-post | native | native |
| We Work Remotely | native | text-post | text-post | native | folded | text-post | structured-post | native | structured-post |
| Himalayas | native | native | text-post | structured-post | native | text-post | structured-post | native | native |
| Remotive | native | text-post | text-post | native | folded | text-post | native | native | structured-post |

## Inactive actors (disabled in HuntBrief UI/backend)

The following actors were removed from active use because they could not produce usable results:

| Actor | Reason |
|---|---|
| **Wellfound** | Live sample audit: actor succeeded but returned **0 items** for broad test queries. Not worth API credits. |
| **FlexJobs** | Apify returns `full-permission-actor-not-approved` / 403 for this actor. Requires Apify full-permission approval before it can run. |

Their documentation, adapter code, and live sample evidence remain in `docs/apify/` for potential future reactivation.

## Backend filtering decisions

- Seniority is level/title based. It uses normalized seniority where available plus job title signals such as `intern`, `entry`, `junior`, `associate`, `senior`, `staff`, `principal`, `lead`, `director`, `head`, and `executive`.
- Experience is years based. Phrases like `3+ years`, `6 years`, `1-3 years`, `YOE`, or nearby `experience` text are treated as experience, not seniority.
- Text-derived experience parsing uses normalized `experience`, requirements, and description text. It ignores numbers above 20 to avoid obvious salary/count false positives.
- Include keywords are only considered supported when sent natively or folded into query/keyword inputs. They are not silently post-filtered.
- Avoid keywords are text-post for most actors and native only where the actor has exclude inputs.
- Missing/empty structured values are permissive in post-filters to avoid dropping jobs just because an actor omitted a field.

## Live sample gap

This matrix was derived from local adapters/docs plus public schema audit. A paid/live small sample fetch for each actor is still recommended before treating output field confidence as final, especially for text-derived experience and salary/date formatting differences.

## Live sample audit — 2026-06-06

A sequential, low-concurrency Apify audit was run with tiny result reads. Raw compact evidence is saved in:

- `docs/apify/live-sample-audit-20260606.json`
- `docs/apify/live-sample-audit-20260606-retry.json`
- `docs/apify/live-sample-audit-20260606-corrected.json`
- `docs/apify/live-sample-audit-20260606-fantastic.json`

Observed usable samples:

| Actor | Live sample result |
|---|---|
| 54 Career Sites | 3 items after using `timeRange: "6m"` and actor minimum `limit: 10` |
| LinkedIn | 3 items after using `timeRange: "6m"` and actor minimum `limit: 10` |
| Indeed | 3 items after using `fromDays` as a string |
| Welcome to the Jungle | 3 items after using `countryCode` and `posted_within: "30d"` |
| HiringCafe | 9 items across 3 queries |
| We Work Remotely | 3 items |
| Himalayas | 3 items |
| Remotive | 3 items |
| YC Startup Jobs | 6 items across 2 successful queries |
| Wellfound | Actor succeeded but returned 0 items for broad test queries |
| FlexJobs | Not sampled: Apify returned `full-permission-actor-not-approved` / 403 until the actor permission is approved |

Corrections from the live audit:

- Fantastic Jobs actors accept `timeRange` values `1h`, `24h`, `7d`, `6m`; `1 month` is invalid.
- Fantastic Jobs actors require `limit >= 10`; dataset reads can still inspect only the first 3 items.
- `borderline/indeed-scraper` requires `fromDays` as a string.
- Welcome to the Jungle requires `posted_within` values `any`, `24h`, `7d`, `30d` and uses `countryCode` for country filtering.
- Remotive output uses `description_text` and `salary_text`/`salary_min`/`salary_max` fields.
- We Work Remotely output uses camelCase `applyUrl` and `descriptionText`/`descriptionHtml`.
- Indeed output uses `datePublished`, `postedToday`, `isRemote`, `workingSystem`, `descriptionText`, and `applyUrl` aliases.
