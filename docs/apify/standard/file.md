# Standard Job Sites — Actor Reference

See `docs/apify/api-input-schema.md` for full input schemas and HuntBrief mapping tables.

| Site | Actor Slug | Key Filters Supported | Post-Filter Only |
|---|---|---|---|
| 54 Career Sites | `fantastic-jobs/career-site-job-listing-api` | title, location, keywords, posted, remote (not wired), seniority (not wired), experience (not wired) | salary |
| Indeed | `borderline/indeed-scraper` | query, country, location, remote/hybrid, seniority level, posted date | experience, salary, exclude keywords |
| LinkedIn | `fantastic-jobs/advanced-linkedin-job-search-api` | title, location, keywords, posted, remote, seniority (not wired) | experience, salary |
| Wellfound | `crawlerbros/wellfound-scraper` | title, keyword, location, remote, seniority, salary | exclude keywords, experience |
| YC Startup Jobs | `memo23/y-combinator-scraper` | keyword, remote (location=remote) | everything else |
| Welcome to the Jungle | `shahidirfan/jungle-job-scraper` | keyword, country code, posted within | everything else |

## URLs

- https://apify.com/fantastic-jobs/career-site-job-listing-api (54 Sites)
- https://apify.com/borderline/indeed-scraper (Indeed)
- https://apify.com/fantastic-jobs/advanced-linkedin-job-search-api (LinkedIn)
- https://apify.com/crawlerbros/wellfound-scraper (Wellfound)
- https://apify.com/memo23/y-combinator-scraper (YC Startup Jobs)
- https://apify.com/shahidirfan/jungle-job-scraper (Welcome to the Jungle)
