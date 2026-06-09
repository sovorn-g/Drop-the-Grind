# Drop the Grind

**A local-first macOS desktop app for job hunting.** Collect opportunities from 11 curated job sources, organize them as plain Markdown files, and prepare tailored resumes — all from your machine.

It's not an auto-apply bot. It's a human-reviewed job search workspace that keeps you in control and your data on disk.

---

## What it does

### HuntBrief — Find & import jobs

**Find Jobs** — Search across 11 job boards with one click. Drop the Grind runs Apify actors through your own API token, normalizes the results, filters and deduplicates them, then writes clean Markdown files to your local workspace.

**Import Links** — Paste job posting URLs and Drop the Grind extracts the full page content using Tavily or Firecrawl, cleans out navigation noise and footers, and saves each posting as a readable `.md` file.

**Sources included:**

| Standard | Remote |
|---|---|
| 54 Career Sites | HiringCafe |
| Indeed | We Work Remotely |
| LinkedIn | 4 Day Week |
| YC Startup Jobs | Himalayas |
| Welcome to the Jungle | JustRemote |
| | Remotive |

### Resume builder

Upload your base resume. Drop the Grind parses it, validates it, and can render it to a clean PDF via a bundled Typst engine. The built-in agent (Codex-powered) can tailor your resume to individual job descriptions you've collected.

### Agent Chat

A project-aware chat panel for asking questions, getting help, or generating tailored resumes — with file mentions (`@path`), session forking, streaming responses, and context-window awareness.

### Local workspace

Everything lives under `~/.dropthegrind/workspace/<your-project>/`:

```
workspace/
└── my-job-search/
    ├── profile/          ← your resume and user profile
    ├── hunt_run/         ← Find Jobs results
    │   └── frontend-remote/
    │       ├── results.md        ← summary index
    │       └── jobs/
    │           ├── 001-senior-frontend-acme.md
    │           └── 002-react-engineer-startup.md
    ├── import-links/     ← Import Links results
    ├── resume/           ← tailored resumes
    └── pdf/              ← rendered PDFs
```

No cloud storage. No accounts. Just files on your Mac.

---

## How to use

### 1. Get an Apify API token

Sign up at [apify.com](https://console.apify.com/) and grab your API token. You'll also want free Tavily and Firecrawl keys if you plan to use Import Links.

### 2. Install and launch

Build from source (see below), or grab the `.dmg` from [Releases](https://github.com/cheasovorn4/Drop-the-Grind/releases).

### 3. Create a project

On first launch, name your project (e.g. "My 2026 Job Search"). This creates your local workspace.

### 4. Connect your APIs

Open **Settings** (⚙ at the bottom of the left panel) and connect:
- **Apify** — required for Find Jobs
- **Tavily** — for Import Links web extraction
- **Firecrawl** — alternative extraction backend for Import Links
- **Codex** — for the Agent Chat panel (optional)

### 5. Start hunting

- **Find Jobs tab:** Enter your roles, location, keywords, pick your sources, and hit **Start Hunting**.
- **Import Links tab:** Paste job posting URLs and hit **Extract Jobs**.

Watch the progress dashboard as results come in. When it's done, open `results.md` to browse your job list or dive into individual `jobs/*.md` files.

### 6. Work with results

- Browse files in the left panel tree.
- Preview Markdown files with the built-in viewer (toggle Edit/Preview).
- Right-click any `resume/` file or folder → **Render to PDF**.
- Use the Agent Chat to ask questions or tailor resumes to a specific job.

---

## Install (build from source)

**Requirements:** Node.js, Rust (via [rustup](https://rustup.rs)), and macOS.

```bash
# Clone
git clone https://github.com/cheasovorn4/Drop-the-Grind.git
cd Drop-the-Grind

# Install frontend dependencies
npm install

# Build the macOS app
./build.sh
```

Output:
```
src-tauri/target/release/bundle/macos/Drop the Grind.app
src-tauri/target/release/bundle/dmg/Drop the Grind_0.1.0_aarch64.dmg
```

**Optional — PDF rendering:** Drop the Grind bundles a Typst binary for resume PDF generation. If it's missing:

```bash
./scripts/install-typst-resource.sh
```

If you move the repo and Tauri complains about stale paths:

```bash
rm -rf src-tauri/target
./build.sh
```

---

## Tech stack

| Layer | Tech |
|---|---|
| UI | React 19, TypeScript, Vite |
| Desktop shell | Tauri 2 |
| Backend | Rust |
| Styling | Tailwind CSS + custom dark glass design system |
| Storage | Filesystem + SQLite |
| PDF engine | Typst (bundled) |
| APIs | Apify, Tavily, Firecrawl |

---

## Why this exists

Job hunting is repetitive, high-stress work. Drop the Grind keeps you in control: deterministic scraping, human-reviewed results, plain Markdown files you can read anywhere, and no auto-apply behavior. It's built like a premium Mac utility — dark glass, compact density, quiet feedback — not a SaaS dashboard.

---

## Questions?

[cheasovorn4@gmail.com](mailto:cheasovorn4@gmail.com)
