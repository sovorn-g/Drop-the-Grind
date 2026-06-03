#!/usr/bin/env python3
"""Generate docs/apify/api-input-schema.md from current actor lists.

Reads:
  docs/apify/standard/file.md
  docs/apify/remote/file.md

Fetches latest Apify actor build input schemas, so running this later refreshes
schema notes when actor lists or actor builds change.
"""
from __future__ import annotations

import json
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent
STANDARD_FILE = ROOT / "standard" / "file.md"
REMOTE_FILE = ROOT / "remote" / "file.md"
OUTPUT_FILE = ROOT / "api-input-schema.md"

TEST_SETTINGS = "roles `AI Engineer`, `AI Automation`; seniority `Entry level`, `Associate`; experience `0–1 year`, `1–3 years`; salary `Any`; include keywords `AI`, `Artificial Intelligence`, `Automation`, `AI Engineer`; avoid keywords `none`; posted within `1 week`; locations `United Kingdom`, `New Zealand`"


def fetch_json(url: str) -> dict:
    req = urllib.request.Request(url, headers={"User-Agent": "DropTheGrindSchemaGenerator/1.0"})
    with urllib.request.urlopen(req, timeout=40) as res:
        payload = json.load(res)
    return payload.get("data", payload)


def actor_slug_from_url(url: str) -> str | None:
    m = re.search(r"https://apify\.com/([^\s)]+)", url)
    if not m:
        return None
    parts = [p for p in m.group(1).split("/") if p and p not in {"reviews", "api", "input-schema"}]
    if len(parts) < 2:
        return None
    return f"{parts[0]}/{parts[1]}"


def parse_actor_list(path: Path, mode: str) -> list[dict]:
    actors: list[dict] = []
    for line in path.read_text().splitlines():
        slug = actor_slug_from_url(line)
        if not slug:
            continue
        label_match = re.search(r"\(([^)]+)\)", line)
        label = label_match.group(1).strip() if label_match else slug.split("/", 1)[1]
        if label == "54 Sites":
            label = "54 Career Sites"
        if label == "HireCafe":
            label = "HiringCafe"
        actors.append({"mode": mode, "label": label, "actorSlug": slug})
    return actors


def parse_schema(raw) -> dict:
    if isinstance(raw, str):
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return {"raw": raw}
    return raw or {}


def compact(value, max_len: int = 150) -> str:
    if value is None:
        return ""
    if isinstance(value, (dict, list)):
        value = json.dumps(value, ensure_ascii=False)
    text = " ".join(str(value).split())
    return text if len(text) <= max_len else text[: max_len - 1] + "…"


def adapter_hint(label: str) -> str:
    hints = {
        "54 Career Sites": "roles → `titleSearch`; locations → `locationSearch`; include/avoid → `descriptionSearch`/exclusions; posted within 1 week → `timeRange: 7d`; max uses `limit` but actor minimum is 10.",
        "LinkedIn": "same Fantastic Jobs style as 54 Career Sites; add `remote` when Remote mode; use `limit` minimum 10 and `timeRange: 7d` for 1 week.",
        "Indeed": "primary role query → `position`; country should be a supported country code such as `GB`/`NZ`; location string → `location`; max → `maxItemsPerSearch`.",
        "Wellfound": "role → `jobTitle`; include keywords → `keyword`; remote mode → `remoteOnly`; seniority maps to `experience`; max → `maxItems`.",
        "YC Startup Jobs": "use `mode: jobs`; roles/keywords → `queries`; remote mode can use `location: remote`; max → `maxItems`.",
        "Welcome to the Jungle": "role/keywords → `keyword`; location → `location`; posted within 1 week → `posted_within: 7d`; max → `results_wanted`.",
        "HiringCafe": "roles/keywords → `keyword`; locations → `location`; remote mode → `workplaceType: Remote`; max → `maxItems`; use `flattenOutput` for easier output.",
        "We Work Remotely": "limited structured filters; use `category: all` unless UI adds category; max → `results_wanted`; post-filter by roles/keywords/location.",
        "4 Day Week": "use required `mode: search`; roles/keywords → `query`; max → `maxItems`; category/jobType optional.",
        "FlexJobs": "API is URL-driven; build FlexJobs search URL(s) in `urls`; max per URL → `max_items_per_url`; use `ignore_url_failures`.",
        "Himalayas": "roles → `keywords`; seniority → `seniority`; country/worldwide maps to `country`/`worldwide`; max → `maxResultsPerKeyword`.",
        "JustRemote": "requires URL input; build JustRemote search URL(s) in `inputUrls`; max → `maxResults`.",
        "Remotive": "roles/keywords → `searchQueries`; max → `maxResultsPerQuery` and `maxResults`; include company info if useful.",
    }
    return hints.get(label, "Map HuntBrief fields to the closest actor fields and post-filter unsupported filters.")


def main() -> int:
    actors = parse_actor_list(STANDARD_FILE, "Standard") + parse_actor_list(REMOTE_FILE, "Remote")
    if not actors:
        print("No actors found in docs/apify/standard/file.md or docs/apify/remote/file.md", file=sys.stderr)
        return 1

    lines: list[str] = []
    lines.append("# Apify Actor API Input Schema Notes\n\n")
    lines.append("Generated by `docs/apify/generate-api-input-schema.py`.\n\n")
    lines.append("Source: Apify public API `GET /v2/acts/{actor}` plus latest build `GET /v2/actor-builds/{buildId}`. These are developer notes for implementing deterministic HuntBrief adapters.\n\n")
    lines.append(f"Test HuntBrief settings used for schema review: {TEST_SETTINGS}.\n\n")

    for actor in actors:
        label = actor["label"]
        slug = actor["actorSlug"]
        print(f"Fetching input schema: {label} ({slug})")
        try:
            act = fetch_json("https://api.apify.com/v2/acts/" + urllib.parse.quote(slug, safe=""))
            build_id = act.get("taggedBuilds", {}).get("latest", {}).get("buildId")
            build = fetch_json(f"https://api.apify.com/v2/actor-builds/{build_id}") if build_id else {}
            schema = parse_schema(build.get("inputSchema"))
            props = schema.get("properties", {}) if isinstance(schema, dict) else {}
            required = schema.get("required", []) if isinstance(schema, dict) else []
        except Exception as exc:  # noqa: BLE001
            lines.append(f"## {label}\n\n- Mode: {actor['mode']}\n- Actor: `{slug}`\n- Error fetching schema: `{exc}`\n\n")
            continue

        lines.append(f"## {label}\n\n")
        lines.append(f"- Mode: {actor['mode']}\n- Actor: `{slug}`\n- Required input fields: {', '.join('`'+x+'`' for x in required) if required else 'none declared'}\n\n")
        lines.append("| Field | Type | Default | Enum / limits | Notes |\n|---|---|---|---|---|\n")
        for name, prop in props.items():
            limits = []
            if prop.get("enum"):
                enum = prop["enum"]
                limits.append("enum: " + ", ".join(map(str, enum[:12])) + ("…" if len(enum) > 12 else ""))
            if prop.get("minimum") is not None:
                limits.append("min " + str(prop["minimum"]))
            if prop.get("maximum") is not None:
                limits.append("max " + str(prop["maximum"]))
            lines.append(f"| `{name}` | {prop.get('type', '')} | `{compact(prop.get('default'), 80)}` | {compact('; '.join(limits), 180)} | {compact(prop.get('description') or prop.get('title'))} |\n")
        lines.append("\n### HuntBrief adapter hints\n\n")
        lines.append(adapter_hint(label) + "\n\n")
        time.sleep(0.15)

    OUTPUT_FILE.write_text("".join(lines))
    print(f"Wrote {OUTPUT_FILE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
