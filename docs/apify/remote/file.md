# Remote Job Sites — Actor Reference

See `docs/apify/api-input-schema.md` for full input schemas and HuntBrief mapping tables.

| Site | Actor Slug | Key Filters Supported | Post-Filter Only |
|---|---|---|---|
| HiringCafe | `memo23/apify-hiring-cafe-scraper` | keyword, location, workplace type | everything else |
| We Work Remotely | `crawlerbros/weworkremotely-job-scraper` | keyword, region, job type, salary | seniority, experience, posted, exclude keywords |
| Himalayas | `inlifeprojects/himalayas-jobs-scraper` | keywords, seniority, location (worldwide/country) | experience, salary, exclude keywords |
| Remotive | `unfenced-group/remotive-scraper` | keyword, categories, location, salary, posted date | seniority, experience, exclude keywords |

## Inactive

### FlexJobs
- Actor: `jupri/flexjobs-scraper`
- Reason inactive: Apify returns `full-permission-actor-not-approved` / 403 until the actor permission is approved at the Apify account level.
- Documentation preserved in `docs/apify/api-input-schema.md` and `docs/apify/api-output-schema.md` for future reactivation.
- HuntBrief adapters and UI entries removed March 2026.

## URLs

- https://apify.com/memo23/apify-hiring-cafe-scraper (HiringCafe)
- https://apify.com/crawlerbros/weworkremotely-job-scraper (We Work Remotely)
- https://apify.com/inlifeprojects/himalayas-jobs-scraper (Himalayas)
- https://apify.com/unfenced-group/remotive-scraper (Remotive)
