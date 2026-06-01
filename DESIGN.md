# Drop the Grind — Design Guidelines

Drop the Grind should feel like a **premium local-first macOS job ops cockpit**: dark, compact, calm, file-aware, and operational. It should look closer to Raycast, Arc, Linear, Cursor, or a native AI coding workspace than a generic SaaS dashboard.

These guidelines define the aesthetic and interaction direction for the app UI.

---

## Design concept

**Concept:** Local-first macOS productivity cockpit for job hunting.

The app helps a tired, busy job seeker turn scattered job sources into reviewed application packets. The interface should reduce friction, show the next action clearly, and make generated files feel inspectable and trustworthy.

**Keywords:**

- premium dark mode
- macOS-native
- compact pro-tool density
- frosted sidebar
- quiet neon accents
- local-first transparency
- human-reviewed workflow
- operational, not playful

---

## Visual direction

### Overall feel

Use a refined dark desktop aesthetic:

- Deep graphite/charcoal surfaces, never flat pure black everywhere.
- Layered panels with subtle 1px borders.
- Rounded macOS-like window and card geometry.
- Compact controls and dense information layout.
- Accent colors used sparingly as signals, not decoration.
- Calm, serious, trustworthy mood.

The UI should feel like a tool someone can use for hours during a stressful job hunt.

### Avoid

- Generic SaaS landing-page visuals.
- Oversized hero cards.
- Loud purple/blue gradients.
- Playful resume-builder styling.
- Overly cute illustrations.
- Chatbot-first layout that hides the job workflow.
- Pure black backgrounds with no material depth.

---

## Layout system

Use the **Hybrid Job Ops Workspace** layout:

```txt
┌──────────────────────────── Drop the Grind ────────────────────────────┐
│ Toolbar: Project switcher · source/import status · settings             │
├──────────── Left ────────┬────────────── Center ──────────────┬─────────┤
│ Mode nav + file tree     │ Active workspace view               │ Chat    │
│                          │                                      │ panel   │
│ Setup                    │ Setup checklist / Source setup /     │ Local   │
│ Sources                  │ Job Inbox / Packet detail / Editor   │ project │
│ Job Inbox                │                                      │ chat    │
│ Packets                  │                                      │         │
│ Files                    │                                      │         │
│ Settings                 │                                      │         │
│                          │                                      │         │
│ Project files            │                                      │         │
└──────────────────────────┴──────────────────────────────────────┴─────────┘
```

### Panel sizing

- Left panel: `260–320px` default, `220px` minimum.
- Center panel: flexible, `520px` minimum.
- Right chat panel: `320–380px` default, `280px` minimum.

### Narrow window behavior

1. Collapse right chat panel first.
2. Collapse file tree second.
3. Keep center workflow usable.

---

## Color system

Use named tokens. Values can be adjusted during implementation, but maintain the hierarchy.

```css
:root {
  --dtg-bg: #090a0c;
  --dtg-bg-elevated: #111215;
  --dtg-panel: #17181b;
  --dtg-panel-soft: #202124;
  --dtg-card: #242528;
  --dtg-card-hover: #2a2b2f;

  --dtg-border: rgba(255, 255, 255, 0.08);
  --dtg-border-strong: rgba(255, 255, 255, 0.14);

  --dtg-text: #f2f2f0;
  --dtg-text-muted: #a7a7a2;
  --dtg-text-subtle: #73736d;

  --dtg-blue: #4f8cff;
  --dtg-blue-soft: rgba(79, 140, 255, 0.16);
  --dtg-green: #21d07a;
  --dtg-yellow: #f2b84b;
  --dtg-red: #ff5d5d;
  --dtg-magenta: #df5cff;

  --dtg-focus: #7aa7ff;
}
```

### Usage

- Background: `--dtg-bg`.
- Main panels: `--dtg-panel`.
- Cards and inputs: `--dtg-card` / `--dtg-panel-soft`.
- Borders: `--dtg-border`.
- Active selection: blue soft fill + subtle border.
- Success/ready: green.
- Warning/partial: yellow.
- Error/destructive: red.
- Brand/special status: tiny magenta accent only.

---

## Sidebar treatment

The left sidebar should feel slightly **frosted/glassy**.

Recommended treatment:

- Dark translucent base.
- Subtle navy/blue ambient gradient.
- Optional fine noise texture.
- Thin right border.
- Soft active row highlight.

Example direction:

```css
.sidebar {
  background:
    radial-gradient(circle at 20% 30%, rgba(64, 105, 205, 0.35), transparent 32%),
    linear-gradient(180deg, rgba(28, 30, 34, 0.92), rgba(17, 18, 20, 0.96));
  border-right: 1px solid var(--dtg-border);
  backdrop-filter: blur(18px);
}
```

Do not make the sidebar bright or colorful. The gradient should be felt more than seen.

---

## Typography

Use compact, legible typography suitable for dense desktop tools.

### Direction

- Small text sizes.
- Clear hierarchy through weight, opacity, and spacing.
- Avoid giant page titles.
- Prefer concise labels and direct action copy.

### Suggested scale

- Tiny metadata: `11px`
- Secondary labels: `12px`
- Body/UI text: `13–14px`
- Section titles: `15–16px`
- Large empty-state title: `20–24px`

### Font choice

Use a high-quality UI font with a native/macOS feel. Avoid generic web-marketing typography.

Acceptable options:

- macOS system UI stack for native fidelity.
- A refined variable sans if bundled intentionally.
- Monospace only for JSON, paths, logs, and code/task files.

Do not use decorative fonts. This app is a serious productivity tool.

---

## Components

### Buttons

Buttons should be compact and tactile.

- Height: `28–34px` for most controls.
- Border radius: `8–10px`.
- Primary buttons: filled dark/blue-accented, not oversized.
- Secondary buttons: dark filled with subtle border.
- Ghost buttons: sidebar/nav/tool actions.
- Destructive buttons: red text or red-tinted border, not giant red blocks.

Button copy should be action-specific:

- `Create Project`
- `Generate MCP Task`
- `Import Output JSON`
- `Approve`
- `Reject`
- `Generate Packet`
- `Reveal in Finder`

### Cards

Cards are used for jobs, sources, packets, and setup checklist groups.

Style:

- Dark filled surface.
- 1px subtle border.
- Radius `12–16px`.
- Minimal shadow.
- Hover state raises contrast slightly.

Avoid heavy elevation or bright outlines.

### Badges and pills

Use small rounded badges for:

- job status
- remote type
- salary present
- source type
- packet status
- import state

Badge style:

- Font size `11–12px`.
- Rounded capsule.
- Muted fill.
- Subtle border.

Examples:

- `Remote`
- `Hybrid`
- `Imported`
- `Packet created`
- `Not scored yet`
- `Partial import`

### Inputs

Inputs should be dark, compact, and bordered.

- Filled dark background.
- 1px border.
- Clear focus ring.
- Placeholder text muted.
- Search fields should include an icon if available.

### Dividers

Use 1px borders and soft opacity. Do not use thick separators.

---

## Primary views

### Setup view

First project screen should show a checklist:

- Add `resume_original.pdf`
- Add or edit `resume_extracted.md`
- Review `preferences.json`
- Add Apify actor/source
- Generate MCP task/config
- Import Apify output JSON
- Review jobs
- Generate first application packet

Each checklist item should deep-link to the relevant view or file.

### Sources view

Use source cards for Apify/MCP setup.

Each card should show:

- Source name.
- Actor name.
- MCP server URL.
- Input template path.
- Last import status.
- Buttons:
  - `Edit Input JSON`
  - `Generate MCP Config`
  - `Generate Run Task`
  - `Import Output JSON`

Important copy:

> Generate MCP instructions for Codex/opencode, then import the output JSON here.

Do not imply the app directly runs Apify in milestone 1.

### Job Inbox view

This is the core product screen.

Job cards/rows should show:

- title
- company
- location/remote badge
- salary badge if available
- source badge
- fit score placeholder or `Not scored yet`
- short explanation/warnings
- actions: `Approve`, `Reject`, `Open Apply URL`, `Generate Packet`

Empty state:

> No jobs imported yet. Add an Apify actor in Sources, generate the MCP task, then import the JSON output.

Partial import state:

> 42 jobs imported · 7 skipped because required fields were missing.

### Packets view

Packet rows/cards should show:

- company + job title
- packet path
- packet status
- created/updated time
- task file hints
- actions:
  - `Open Packet`
  - `Reveal in Finder`
  - `Open tailor_resume.md`
  - `Open verification_report.md`

Existing packet copy:

> Packet already exists. Open it instead?

Actions:

- `Open Existing`
- `Write .new task files`
- `Cancel`

### Files view/editor

Editor requirements:

- Breadcrumb path.
- Dirty-state indicator.
- `Save` button.
- `Cmd+S` support.
- Monospace text area/editor for JSON/Markdown/LaTeX initially.

Read-only binary/PDF banner:

> Preview/editing is not supported for this file type yet. Reveal in Finder instead.

### Right chat panel

The right panel is a real local chat interface, but milestone 1 has no model execution.

Empty state copy:

> Project Chat  
> Draft instructions, notes, and task prompts for this project.  
> Model execution is not connected yet. Use this panel to compose task files for Codex/opencode or future providers.

Required elements:

- Message list.
- Composer input.
- Context chips:
  - current file
  - selected job
  - selected packet
- Actions:
  - `Save message`
  - `Create task file`
  - `Append to current task`

Do not use fake typing indicators or animated assistant behavior before model execution exists.

---

## Motion and interaction

Keep motion subtle and native-feeling.

Use:

- Quick fades for panel/content changes.
- Slight background shift on hover.
- Gentle scale or opacity for active pills/buttons.
- Toasts for save/import/packet success.

Avoid:

- Bouncy animations.
- Large page transitions.
- Constant glowing effects.
- AI “thinking” animations before real execution exists.

Suggested timings:

- Hover: `120–160ms`
- Panel/content fade: `160–220ms`
- Toast entrance: `180–240ms`

---

## Interaction states

Every primary view should handle these states deliberately:

- no project
- loading project
- empty source/job/packet states
- missing actor name
- malformed JSON import
- partial import success
- permission denied
- unsaved editor changes
- save success/failure
- binary/PDF read-only
- packet already exists
- chat local-only empty state
- chat save failure
- narrow-window collapsed panels

Error states should include a recovery action, not only an error message.

---

## Accessibility

Minimum requirements:

- Visible focus rings on all interactive controls.
- Keyboard navigation for mode nav, file tree, and job list.
- `Cmd+S` saves active file.
- Chat composer has an accessible label.
- Buttons use clear labels, not icon-only controls unless labelled.
- Job statuses do not rely on color alone.
- Text contrast must remain readable on dark surfaces.
- Destructive actions require confirmation or undo.

Focus ring direction:

```css
:focus-visible {
  outline: 2px solid var(--dtg-focus);
  outline-offset: 2px;
}
```

---

## Trust and safety copy

Use subtle trust cues in setup, settings, and relevant empty states.

Recommended copy snippets:

- `Local-first: project files stay under ~/.dropthegrind/workspace.`
- `Human-reviewed: Drop the Grind prepares packets; it does not auto-apply.`
- `Resume-grounded: generated tasks should tailor from your existing resume, not invent experience.`
- `MCP-assisted: Apify MCP tasks are run externally in milestone 1; import outputs back into the app.`

---

## Implementation notes

Suggested shadcn/ui primitives:

- `ResizablePanelGroup`
- `ScrollArea`
- `Tabs` or segmented controls
- `Card`
- `Badge`
- `Button`
- `Dialog`
- `Toast`
- `Alert`

Use Tailwind tokens or CSS variables to preserve a consistent dark material system.

The first implementation can be simple, but screenshots should already communicate:

1. This is a serious macOS desktop app.
2. The job workflow is first-class.
3. Files are transparent and local.
4. AI/model execution is intentionally not faked.
