# Drop the Grind — UX/UI Design Guide

Drop the Grind should feel like a premium dark macOS productivity cockpit for job search operations: local-first, calm, file-aware, and serious.

## Product design intent

The user is doing repetitive, high-stress job-search work. The UI should reduce friction, make progress visible, and keep generated artifacts transparent. The app should feel like a first-party desktop utility or an AI coding workspace, not a generic SaaS dashboard or resume-builder landing page.

## Design keywords

- premium dark mode
- glassy macOS sidebar
- compact pro-tool density
- LinkedIn-blue action color
- quiet status feedback
- local file transparency
- human-reviewed workflow
- operational, not playful

## Layout model

Use the current split workspace layout from `src/styles.css`:

```txt
┌──────────────────────────────────────────────────────────────────────────────┐
│ Drop the Grind                                                               │
├───────────────────────┬───────────────────────────────────────┬──────────────┤
│                       HuntBrief / primary workflow            │              │
│                       spans left + center columns              │              │
│                                                               │              │
├───────────────────────┬───────────────────────────────────────┤ Agent Chat   │
│ Files / project tree  │ Editor / Preview                       │ spans full   │
│ Settings at bottom    │ selected file, results, job files      │ height above  │
│                       │                                       │ status bar   │
├───────────────────────┴───────────────────────────────────────┴──────────────┤
│ Status bar                                                                    │
└──────────────────────────────────────────────────────────────────────────────┘
```

CSS grid model:

```css
.workspace-frame {
  grid-template-columns: 270px minmax(0, 1fr) 320px;
  grid-template-rows: 30% calc(70% - 28px) 28px;
}

.hunt-brief-panel { grid-column: 1 / 3; grid-row: 1; }
.files-panel      { grid-column: 1;     grid-row: 2; }
.editor-panel     { grid-column: 2;     grid-row: 2; }
.chat             { grid-column: 3;     grid-row: 1 / 3; }
.statusbar        { grid-column: 1 / 4; grid-row: 3; }
```

### HuntBrief panel

Purpose:

- top workspace command center
- spans the left and center columns
- Find Jobs and Import Links entry points
- source selection
- Start Hunting dashboard/status view

Treatment:

- prominent top panel, not a left sidebar
- compact vertical height because it shares the screen with Files/Editor below
- transparent dark glass surface matching the app theme
- clear loading/progress state while a hunt runs

### Files panel

Purpose:

- lower-left project tree
- open generated files such as `hunt_run/<name>/results.md` and `jobs/*.md`
- provide lower-left Settings access

Treatment:

- compact file browser density
- glass/dark panel styling
- clear active file row
- Settings button anchored at the bottom

### Editor / Preview panel

Purpose:

- lower-center file viewer/editor
- inspect `results.md`, individual job files, notes, and generated artifacts

Treatment:

- readable Markdown/text surface
- minimal distraction
- no noisy background behind long-form content
- strong save/dirty/read-only states

### Agent Chat panel

Purpose:

- full-height right column above the status bar
- project-aware conversation
- later resume/outreach assistance using selected files
- not the Start Hunting execution trigger

Treatment:

- visually stable right rail
- match the glass direction
- white/gray chat surfaces, not brown/warm tones
- do not visually overpower HuntBrief or Editor

## Color direction

Primary action/accent: LinkedIn-like blue.

Recommended token direction:

```css
--dtg-bg: #090a0c;
--dtg-panel: rgba(15, 18, 24, 0.78);
--dtg-panel-strong: rgba(20, 24, 32, 0.88);
--dtg-border: rgba(255, 255, 255, 0.08);
--dtg-border-strong: rgba(255, 255, 255, 0.14);
--dtg-text: #f3f5f7;
--dtg-muted: #a7adb7;
--dtg-subtle: #737b88;
--dtg-blue: #0a66c2;
--dtg-blue-soft: rgba(10, 102, 194, 0.18);
--dtg-red: #ff5d5d;
```

Use blue for primary buttons, selected controls, focus rings, and active source states. Use red only for destructive hover/confirmation states.

## HuntBrief UX

HuntBrief is the main product workflow.

### Find Jobs tab

Keep fields top-down and scannable:

- role inputs
- seniority
- experience
- min salary
- include keywords
- avoid keywords
- posted within
- location full-width row
- curated source selection
- Start Hunting action

### Source selection

Standard and Remote are vertical mode buttons inside the curated source section. Source options should be visible without cramped scroll where practical.

Default source selection:

- Standard: `54 Career Sites`
- Remote: `HiringCafe`

Only `54 Career Sites` should have the explanatory hover tooltip. Do not add hover descriptions to every source or Settings chip.

### Start Hunting dashboard

During Start Hunting, HuntBrief should become a progress/dashboard view with safe operational states, not private agent reasoning. Show steps such as:

- Preparing hunt run
- Checking Apify API
- Running source actors
- Reading dataset items
- Normalizing/filtering jobs
- Writing results
- Complete

The dashboard should include clear completion actions such as Open results and Start another hunt.

## Results UX

Hunt results should be file-first and agent-friendly.

```txt
results.md = summary/index
jobs/*.md = individual job detail files
```

This prevents overwhelming the agent with 50–100 jobs and makes resume tailoring a one-job-at-a-time workflow.

`results.md` should include:

- run name
- mode
- sources
- generated timestamp
- max scrape results
- HuntBrief settings
- summary counts
- linked job list

`jobs/*.md` should include:

- source
- company
- location
- salary
- posted date
- apply/original URLs
- work mode
- seniority/experience when available
- requirements
- key skills
- capped description
- resume/outreach note

Do not show raw actor slugs in the job metadata because Source is the user-facing label.

## Interaction principles

- Buttons should be compact and tactile.
- Primary actions use blue; destructive hovers use red.
- Avoid hidden horizontal overflow from tooltips or pseudo-elements.
- Dropdowns should use dark styled scrollbars and avoid bright native white scrollbars.
- Location dropdown should be searchable and compact.
- Settings popup should open quickly from cached connection state and refresh on demand.

## Avoid

- raw API JSON in user-facing Markdown
- one huge file for all job details
- warm/brown chat boxes
- playful/illustration-heavy design
- generic SaaS homepage styling
- silent long-running Start Hunting operations
- agent chat being used as the Start Hunting execution trigger
