#!/usr/bin/env python3
"""Generate docs/apify/api-output-schema.md from current actor lists.

Default mode is safe/cheap: fetch actor latest build metadata and README output
sections from Apify. Optional `--sample` starts tiny actor runs where metadata is
missing/unclear; it requires APIFY_TOKEN or ~/.dropthegrind/settings.json.
"""
from __future__ import annotations

import argparse
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
OUTPUT_FILE = ROOT / "api-output-schema.md"
SAMPLE_DIR = ROOT / "samples"

TEST_SETTINGS_MD = """- Roles: AI Engineer, AI Automation
- Seniority: Entry level, Associate
- Experience: 0–1 year, 1–3 years
- Min salary: Any
- Include keywords: AI, Artificial Intelligence, Automation, AI Engineer
- Avoid keywords: none
- Posted within: 1 week
- Location: United Kingdom, New Zealand
- Requested sample size: 2 where needed. Some actors enforce their own minimums or returned 0 rows for that exact search.
"""


def fetch_json(url: str, token: str | None = None, method: str = "GET", body: dict | None = None) -> dict | list:
    data = json.dumps(body).encode() if body is not None else None
    headers = {"User-Agent": "DropTheGrindSchemaGenerator/1.0", "Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request(url, data=data, method=method, headers=headers)
    with urllib.request.urlopen(req, timeout=80) as res:
        payload = json.load(res)
    return payload.get("data", payload) if isinstance(payload, dict) else payload


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


def compact(text, max_len: int = 180) -> str:
    if text is None:
        return ""
    if isinstance(text, (dict, list)):
        text = json.dumps(text, ensure_ascii=False)
    text = " ".join(str(text).split())
    return text if len(text) <= max_len else text[: max_len - 1] + "…"


def actor_api_slug(slug: str) -> str:
    return slug.replace("/", "~")


def read_token() -> str | None:
    import os

    if os.environ.get("APIFY_TOKEN"):
        return os.environ["APIFY_TOKEN"]
    settings = Path.home() / ".dropthegrind" / "settings.json"
    if settings.exists():
        try:
            return json.loads(settings.read_text()).get("apifyApiToken")
        except Exception:  # noqa: BLE001
            return None
    return None


def sample_input(label: str) -> dict:
    q = "AI Engineer OR AI Automation"
    role_list = ["AI Engineer", "AI Automation"]
    return {
        "54 Career Sites": {"timeRange": "7d", "limit": 10, "includeAi": True, "titleSearch": role_list, "locationSearch": ["United Kingdom", "New Zealand"], "descriptionSearch": ["AI", "Artificial Intelligence", "Automation", "AI Engineer"], "descriptionType": "text"},
        "LinkedIn": {"timeRange": "7d", "limit": 10, "includeAi": True, "titleSearch": role_list, "locationSearch": ["United Kingdom", "New Zealand"], "descriptionSearch": ["AI", "Artificial Intelligence", "Automation", "AI Engineer"], "descriptionType": "text"},
        "Indeed": {"position": q, "country": "GB", "location": "United Kingdom", "maxItemsPerSearch": 2, "saveOnlyUniqueItems": True},
        "Wellfound": {"jobTitle": q, "keyword": "AI", "location": "United Kingdom", "remoteOnly": False, "experience": "any", "minSalary": 0, "includeNoSalary": True, "sort": "newest", "maxItems": 2},
        "YC Startup Jobs": {"mode": "jobs", "queries": role_list, "location": "", "scrapeOpenJobs": True, "maxItems": 2},
        "Welcome to the Jungle": {"keyword": q, "location": "United Kingdom", "posted_within": "7d", "results_wanted": 2, "max_pages": 1},
        "HiringCafe": {"keyword": q, "location": "United Kingdom, New Zealand", "workplaceType": "Any", "maxItems": 2, "flattenOutput": True, "enrichDescription": True},
        "We Work Remotely": {"category": "all", "results_wanted": 2},
        "4 Day Week": {"mode": "search", "query": q, "category": "", "jobType": "", "maxItems": 2},
        "FlexJobs": {"urls": ["https://www.flexjobs.com/search?search=AI+Engineer"], "ignore_url_failures": True, "max_items_per_url": 2},
        "Himalayas": {"keywords": role_list, "seniority": ["Entry-level", "Mid-level"], "employmentType": "Full Time", "worldwide": False, "country": "United Kingdom", "sortBy": "recent", "maxResultsPerKeyword": 2, "filterNonTech": False},
        "JustRemote": {"inputUrls": ["https://justremote.co/remote-jobs/search?search=AI%20Engineer"], "scrapeCompanyInfo": True, "maxResults": 2, "enableCache": True},
        "Remotive": {"searchQueries": role_list, "includeCompanyInfo": True, "maxResultsPerQuery": 2, "maxResults": 2},
    }.get(label, {"query": q, "maxItems": 2})


def run_sample(label: str, slug: str, token: str) -> tuple[str, list[str]]:
    SAMPLE_DIR.mkdir(parents=True, exist_ok=True)
    run = fetch_json(f"https://api.apify.com/v2/acts/{actor_api_slug(slug)}/runs", token=token, method="POST", body=sample_input(label))
    run_id = run.get("id")
    dataset_id = run.get("defaultDatasetId")
    status = run.get("status", "UNKNOWN")
    for _ in range(120):
        if status in {"SUCCEEDED", "FAILED", "ABORTED", "TIMED-OUT"}:
            break
        time.sleep(2)
        poll = fetch_json(f"https://api.apify.com/v2/actor-runs/{run_id}", token=token)
        status = poll.get("status", "UNKNOWN")
        dataset_id = dataset_id or poll.get("defaultDatasetId")
    if status != "SUCCEEDED":
        return f"sample run ended with {status}", []
    items = fetch_json(f"https://api.apify.com/v2/datasets/{dataset_id}/items?clean=true&format=json&limit=2", token=token)
    if not isinstance(items, list):
        items = []
    sample_path = SAMPLE_DIR / f"{slug.replace('/', '__')}.json"
    sample_path.write_text(json.dumps(items, indent=2, ensure_ascii=False))
    keys = sorted({key for item in items if isinstance(item, dict) for key in item.keys()})
    return f"sample run returned {len(items)} item(s); saved `{sample_path.relative_to(ROOT)}`", keys


def readme_output_fields(label: str, readme: str) -> list[str]:
    if label == "Wellfound" and "What this actor extracts" in readme:
        return ["type", "jobId", "title", "slug", "jobUrl", "compensation", "remote", "locations", "companyId", "companyName", "companySlug", "companyUrl", "companyLogo", "postedAt", "scrapedAt"]
    return []


def normalizer_map(label: str) -> tuple[str, str, str, str, str, str, str, str] | None:
    maps = {
        "54 Career Sites": ("`title`", "`organization`", "`locations_raw` / derived country/city fields", "`salary_raw` or `ai_salary_*`", "`date_posted`", "`external_apply_url` fallback `url`", "`url`", "`description_text`; requirements from `ai_requirements_summary`, skills from `ai_key_skills`"),
        "LinkedIn": ("`title`", "`organization`", "`locations_raw` / derived fields", "`salary_raw` or `ai_salary_*`", "`date_posted`", "`external_apply_url` fallback `url`", "`url`", "`description_text`; requirements from `ai_requirements_summary`, skills from `ai_key_skills`"),
        "Indeed": ("`positionName`", "`company`", "`location`", "`salary`", "not clearly declared", "`url`", "`url`", "description field if present"),
        "Wellfound": ("`title`", "`companyName`", "`locations`; remote from `remote`", "`compensation`", "`postedAt`", "`jobUrl`", "`jobUrl`", "deep description not guaranteed by actor README"),
        "YC Startup Jobs": ("`title`", "`companyName`", "`location`", "`salaryRange` or min/max/currency", "`datePosted`", "`applyUrl`", "`companyUrl` or `applyUrl`", "`description`; HTML in `descriptionHtml`"),
        "Welcome to the Jungle": ("`title`", "`company`", "`location` / `country`", "`salary`", "`date_posted`", "`url`", "`url`", "description field if present"),
        "HiringCafe": ("`job_information_title` or `v5_processed_job_data_core_job_title`", "`v5_processed_job_data_company_name`", "`v5_processed_job_data_formatted_workplace_location` / countries/states/cities", "listed/yearly/hourly compensation fields", "`v5_processed_job_data_estimated_publish_date`", "`apply_url`", "`source` / `apply_url`", "`job_information_description`; requirements from `v5_processed_job_data_requirements_summary`"),
        "We Work Remotely": ("`title`", "`company`", "`location`", "`salary` / min/max/currency", "`date_posted`", "`apply_url` or `url`", "`url`", "`description` if present"),
        "4 Day Week": ("`title`", "`company`", "`location`", "`salary`", "`postedAt`", "`jobUrl`", "`jobUrl`", "`description` if present"),
        "FlexJobs": ("`title`", "`company`", "`job_locations` / `allowed_candidate_location` / `remote_options`", "salary fields if present", "`posted_date`", "job URL/link field", "job URL/link field", "`description` or `job_summary`"),
        "Himalayas": ("`title`", "`company_name`", "`location`; work mode from `work_mode`", "`salary_min`/`salary_max`/`currency`/`salary_period`", "`posted_at`", "`apply_url`", "`source_url` / `data_source_url`", "`description`; tags from `tags`"),
        "JustRemote": ("`Title`", "`companyInfo.Name` if company info enabled", "not consistently declared", "`Salary and Perks`", "not declared", "`Apply Link`", "`URL`", "`Description`; company details in `companyInfo`"),
        "Remotive": ("schema field for title/name", "schema field for company", "schema field for location/candidate_required_location", "salary field if present", "publication date field", "url/apply_url field", "url field", "description field"),
    }
    return maps.get(label)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sample", action="store_true", help="Run tiny Apify actor samples when output fields are missing/unclear. Requires APIFY_TOKEN or Drop the Grind saved Apify token.")
    args = parser.parse_args()

    actors = parse_actor_list(STANDARD_FILE, "Standard") + parse_actor_list(REMOTE_FILE, "Remote")
    token = read_token() if args.sample else None
    if args.sample and not token:
        print("--sample requires APIFY_TOKEN or ~/.dropthegrind/settings.json apifyApiToken", file=sys.stderr)
        return 1

    lines: list[str] = []
    lines.append("# Apify Actor API Output Schema Notes\n\n")
    lines.append("Generated by `docs/apify/generate-api-output-schema.py`.\n\n")
    lines.append("Source priority: actor latest build `actorDefinition.output`, `actorDefinition.storages.dataset.fields`, dataset view fields, README output examples, and optional small test runs.\n\n")
    lines.append("Important: Apify API endpoints are consistent, but dataset item structure is actor-specific. `results.md` should not store full raw JSON. It should normalize to concise Markdown fields: title, company, location, salary, posted date, apply/source URLs, description/requirements, source actor.\n\n")
    lines.append("## Small test run settings\n\n")
    lines.append(TEST_SETTINGS_MD + "\n")

    for actor in actors:
        label = actor["label"]
        slug = actor["actorSlug"]
        print(f"Fetching output schema: {label} ({slug})")
        try:
            act = fetch_json("https://api.apify.com/v2/acts/" + urllib.parse.quote(slug, safe=""))
            build_id = act.get("taggedBuilds", {}).get("latest", {}).get("buildId")
            build = fetch_json(f"https://api.apify.com/v2/actor-builds/{build_id}") if build_id else {}
            actor_def = build.get("actorDefinition") or {}
            dataset = (actor_def.get("storages") or {}).get("dataset") or {}
            views = dataset.get("views") or {}
            view_fields: list[str] = []
            for view in views.values():
                for field in (view.get("transformation") or {}).get("fields") or []:
                    if field not in view_fields:
                        view_fields.append(field)
            field_schema = dataset.get("fields") or {}
            schema_props = field_schema.get("properties", field_schema) if isinstance(field_schema, dict) else {}
            output_schema = actor_def.get("output") or actor_def.get("outputSchema")
            readme = build.get("readme") or ""
        except Exception as exc:  # noqa: BLE001
            lines.append(f"## {label}\n\n- Mode: {actor['mode']}\n- Actor: `{slug}`\n- Error fetching schema: `{exc}`\n\n---\n\n")
            continue

        fields: list[str] = []
        source = ""
        if schema_props:
            fields = list(schema_props.keys())
            source = "dataset JSON schema fields"
        elif view_fields:
            fields = view_fields
            source = "dataset view fields"
        else:
            fields = readme_output_fields(label, readme)
            if fields:
                source = "README output field list"

        sample_note = ""
        if args.sample and (not fields or label in {"HiringCafe", "YC Startup Jobs", "Himalayas", "JustRemote", "FlexJobs"}):
            try:
                sample_note, sample_fields = run_sample(label, slug, token or "")
                if sample_fields:
                    fields = sample_fields
                    source = "small sample run keys"
            except Exception as exc:  # noqa: BLE001
                sample_note = "sample run failed: " + compact(exc, 220)

        lines.append(f"## {label}\n\n")
        lines.append(f"- Mode: {actor['mode']}\n- Actor: `{slug}`\n")
        lines.append(f"- Declares `actorDefinition.output`: {'yes' if output_schema else 'no'}\n")
        lines.append(f"- Declares dataset JSON schema fields: {'yes' if schema_props else 'no'}\n")
        lines.append(f"- Declares dataset view fields: {len(view_fields)}\n")
        if sample_note:
            lines.append(f"- Sample run: {sample_note}\n")
        lines.append(f"\n**Output field source:** {source or 'not available from actor metadata; run this script with `--sample` or inspect actor README/sample dataset'}\n\n")
        if fields:
            lines.append("Common/declared output fields:\n\n")
            for field in fields[:100]:
                lines.append(f"- `{field}`\n")
            if len(fields) > 100:
                lines.append(f"- … {len(fields) - 100} more fields omitted in this summary\n")
            lines.append("\n")
        mapping = normalizer_map(label)
        if mapping:
            lines.append("| Normalized field | Source field(s) |\n|---|---|\n")
            for normalized, src in zip(["title", "company", "location", "salary", "posted date", "apply URL", "source URL", "description / requirements"], mapping):
                lines.append(f"| {normalized} | {src} |\n")
            lines.append("\n")
        lines.append("---\n\n")
        time.sleep(0.15)

    OUTPUT_FILE.write_text("".join(lines))
    print(f"Wrote {OUTPUT_FILE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
