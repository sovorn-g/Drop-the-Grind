# Drop the Grind — Project Overview

Drop the Grind is a local-first macOS desktop app for job discovery, job-result curation, and application preparation. It is built for a human-reviewed job hunt workflow: collect opportunities, inspect them as files, and later tailor resumes/outreach one job at a time.

The app is not an auto-apply bot. It should keep the user in control and keep generated artifacts inspectable in the local workspace.

## Current architecture

- Frontend: React 19 + TypeScript + Vite
- Desktop shell: Tauri 2
- Backend/native commands: Rust
- Local persistence/settings: `~/.dropthegrind/`
- Project workspaces: `~/.dropthegrind/workspace/<project-slug>/`
- Release bundles: `src-tauri/target/release/bundle/`

## Product principles

1. Local-first by default.
2. Human-reviewed; never auto-apply to jobs.
3. Structured deterministic backend flows for product-critical tasks.
4. Agent/chat workflows are separate from deterministic scrape tasks.
5. Store job hunt artifacts as readable files so the user and agent can inspect them.
6. Avoid feeding every job to the agent at once; use one focused job file per downstream task.

## Current HuntBrief flow

Start Hunting uses the Drop the Grind backend and the direct Apify Actor API. It does not use Codex or Apify MCP for scraping.

Flow:

```txt
HuntBrief form
→ create hunt_run/<name>/
→ run selected Apify actors with actor-specific input adapters
→ fetch dataset items
→ normalize with actor-specific output adapters
→ post-filter using HuntBrief settings
→ dedupe
→ write results.md index
→ write jobs/*.md detail files
```

Current output shape:

```txt
hunt_run/<hunt-name>/
├── results.md
└── jobs/
    ├── 001-title-company.md
    ├── 002-title-company.md
    └── ...
```

`results.md` is a summary/index containing hunt settings, selected sources, max scrape results, counts, and links to individual job files.

Each `jobs/*.md` file contains one normalized job context for later resume tailoring or outreach.

## Apify integration

Apify is configured through the app Settings popup as an Apify API connection. The token is stored in Drop the Grind app settings, not in Codex config.

Current predefined actors:

Standard:

- 54 Career Sites — `fantastic-jobs/career-site-job-listing-api`
- Indeed — `misceres/indeed-scraper`
- LinkedIn — `fantastic-jobs/advanced-linkedin-job-search-api`
- YC Startup Jobs — `memo23/y-combinator-scraper`
- Welcome to the Jungle — `shahidirfan/jungle-job-scraper`

Remote:

- HiringCafe — `memo23/apify-hiring-cafe-scraper`
- We Work Remotely — `shahidirfan/weworkremotely-job-scrapper`
- 4 Day Week — `crawlerbros/four-day-week-jobs-scraper`
- Himalayas — `inlifeprojects/himalayas-jobs-scraper`
- JustRemote — `kinaesthetic_millionaire/justremote`
- Remotive — `santamaria-automations/remotive-scraper`

Inactive (removed from HuntBrief UI/backend, docs preserved in `docs/apify/`):

- **Wellfound** — `crawlerbros/wellfound-scraper` (returned 0 items in live sample audit)
- **FlexJobs** — `jupri/flexjobs-scraper` (replaced `stealth_mode/flexjobs-jobs-search-scraper`; Apify 403 / full-permission-actor-not-approved)

Developer schema helper scripts live in `docs/apify/`:

```bash
docs/apify/generate-api-input-schema.py
docs/apify/generate-api-output-schema.py
```

These regenerate developer notes from the current actor list files.

## Agent/Codex role

Agent Chat uses Codex app-server for user conversation and project-aware help. It should not be used as the deterministic Start Hunting scraper.

Recommended agent usage later:

```txt
User selects one jobs/*.md file
→ agent reads that job file + profile/resume files
→ agent tailors resume/outreach for that one job
```

Do not load a whole 50–100 job results file into the agent for resume tailoring.

## Workspace model

Primary workspace root:

```txt
~/.dropthegrind/workspace/<project-slug>/
```

Important generated/useful folders:

```txt
profile/
hunt_run/
applications/
chats/
project.json
```

Older folders such as `sources/` and `jobs/` may still exist from earlier flows, but HuntBrief now writes to `hunt_run/<name>/`.

## Current important source files

- `src/main.tsx` — React app UI, HuntBrief panel, Settings modal, app layout, frontend invoke calls.
- `src/styles.css` — app styling and dark glass visual system.
- `src-tauri/src/lib.rs` — Rust backend commands, workspace filesystem safety, Settings connection commands, direct Apify Actor API run logic, normalization/filtering, Codex app-server chat path.
- `build.sh` — local release build helper.

## Build commands

```bash
npm install
npm run build
./build.sh
```

If the repo path changes and Tauri build fails due stale absolute paths, delete generated target cache:

```bash
rm -rf src-tauri/target
./build.sh
```

## Known follow-up improvements

- Improve salary formatting in job files.
- Strengthen seniority/experience filtering.
- Better real-company extraction for recruiter-style LinkedIn posts.
- Add match scoring/ranking rather than only filtering/deduping.
- Add UI affordances to open individual `jobs/*.md` files from HuntBrief results.
- Eventually build resume/outreach packet generation around one selected job file.
