use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::process::Command as ProcessCmd;

// ── Data Structures ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResumeMeta {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub linkedin: Option<String>,
    pub summary: Option<String>,
    pub subtitle: Option<String>,
    pub contacts: Vec<ContactItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContactItem {
    pub label: String,
    pub href: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExperienceItem {
    pub title: String,
    pub company: String,
    pub dates: Option<String>,
    pub place: Option<String>,
    pub bullets: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EducationItem {
    pub degree: String,
    pub institution: String,
    pub year: Option<String>,
    pub details: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillItem {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SimpleSectionItem {
    pub title: String,
    pub context: Option<String>,
    pub meta: Option<String>,
    pub bullets: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Resume {
    pub meta: ResumeMeta,
    pub experience: Vec<ExperienceItem>,
    pub skills: Vec<String>,
    pub skill_rows: Vec<SkillItem>,
    pub education: Vec<EducationItem>,
    pub projects: Vec<ExperienceItem>,
    pub awards: Vec<SimpleSectionItem>,
    pub certifications: Vec<SimpleSectionItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
    pub line: Option<usize>,
    pub severity: String, // "error" or "warning"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

// ── Input structs for Tauri commands ───────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResumeInput {
    pub project_slug: String,
    // Workspace-relative directory containing resume.md.
    // PDF rendering is restricted to paths under resume/.
    pub job_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RenderOutput {
    pub pdf_path: String,
    pub compile_errors: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RenderPathInput {
    pub project_slug: String,
    /// Workspace-relative path to a .md file or folder.
    pub path: String,
}

// ── Skills Registry ─────────────────────────────────────────────

/// List of all available agent skills with trigger keywords.
/// Each skill file is embedded as a const string and injected into
/// the agent prompt when its keywords are detected in the user message.
pub struct Skill {
    pub name: &'static str,
    pub description: &'static str,
    pub keyword_patterns: &'static [&'static str],
}

pub const SKILLS: &[Skill] = &[
    Skill {
        name: "/fix-render",
        description: "Debug and fix resume PDF rendering issues (missing fields, format errors, Typst problems)",
        keyword_patterns: &["render", "pdf", "typst", "/fix-render", "fix the resume", "resume is broken", "rendering failed"],
    },
    Skill {
        name: "/resume-builder",
        description: "Tailor the user's base resume to match one or more job descriptions",
        keyword_patterns: &["/resume-builder", "resume-builder", "build resume", "tailor resume", "tailored resume", "build resumes", "tailor resumes", "build tailored resumes"],
    },
];

/// Renders the skill list for inclusion in the agent system prompt.
pub fn skill_registry_prompt() -> String {
    let mut s = String::from("## Available commands\nWhen the user mentions one of these, the full instructions will be injected.\n");
    for skill in SKILLS {
        s.push_str(&format!("- {} — {}\n", skill.name, skill.description));
    }
    s
}

/// Checks if a user message matches any skill's keyword patterns.
pub fn matching_skill(message: &str) -> Option<&'static Skill> {
    let msg = message.to_lowercase();
    for skill in SKILLS {
        for pattern in skill.keyword_patterns {
            if msg.contains(pattern) {
                return Some(skill);
            }
        }
    }
    None
}

/// Returns the full skill instruction text for a given skill name.
pub fn skill_instructions(name: &str) -> Option<&'static str> {
    match name {
        "/fix-render" => Some(SKILL_FIX_RENDER),
        "/resume-builder" => Some(SKILL_RESUME_BUILDER),

        _ => None,
    }
}

// ── Parser ─────────────────────────────────────────────────────

/// Parses a resume.md file into a Resume struct.
/// Handles YAML frontmatter (delimited by `---`) and markdown sections.
pub fn parse_resume(content: &str) -> Result<Resume, String> {
    let content = if content.trim().starts_with("---") {
        content.to_string()
    } else {
        // Wrap in frontmatter if missing
        format!("---\nname: Unknown\nemail: unknown@example.com\n---\n{}", content)
    };

    let parts: Vec<&str> = content.splitn(3, "---\n").collect();
    let frontmatter_str = if parts.len() >= 2 { parts[1] } else { "" };
    let body = if parts.len() >= 3 { parts[2] } else { content.as_str() };

    let meta = parse_frontmatter(frontmatter_str);

    let sections = parse_sections(body);

    let experience = parse_experience(sections.get("experience").map(|s| s.as_str()).unwrap_or(""));
    let skills: Vec<String> = sections.get("skills")
        .map(|s| s.lines()
            .filter(|l| l.starts_with("- ") || l.starts_with("* "))
            .map(|l| l[2..].trim().to_string())
            .collect())
        .unwrap_or_default();
    let education = parse_education(sections.get("education").map(|s| s.as_str()).unwrap_or(""));

    Ok(Resume { meta, experience, skills, skill_rows: Vec::new(), education, projects: Vec::new(), awards: Vec::new(), certifications: Vec::new() })
}

/// Parse frontmatter (simple key: value pairs)
fn parse_frontmatter(s: &str) -> ResumeMeta {
    let mut name = String::new();
    let mut email = String::new();
    let mut phone = None;
    let mut location = None;
    let mut linkedin = None;
    let mut summary = None;

    let mut in_summary = false;
    let mut summary_lines: Vec<String> = Vec::new();

    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_summary { summary_lines.push(String::new()); }
            continue;
        }
        if let Some((key, val)) = trimmed.split_once(':') {
            let k = key.trim().to_lowercase();
            let v = val.trim().to_string();
            match k.as_str() {
                "name" => name = v,
                "email" => email = v,
                "phone" => phone = Some(v),
                "location" => location = Some(v),
                "linkedin" => linkedin = Some(v),
                "summary" => {
                    in_summary = true;
                    if !v.is_empty() { summary_lines.push(v); }
                }
                _ => {
                    if in_summary { summary_lines.push(trimmed.to_string()); }
                }
            }
        } else if in_summary {
            summary_lines.push(trimmed.to_string());
        }
    }

    if !summary_lines.is_empty() {
        summary = Some(summary_lines.join(" ").trim().to_string());
    }

    ResumeMeta { name, email, phone, location, linkedin, summary, subtitle: None, contacts: Vec::new() }
}

/// Parse markdown sections (## Section Name)
fn parse_sections(body: &str) -> std::collections::HashMap<String, String> {
    let mut sections = std::collections::HashMap::new();
    let mut current_section = String::new();
    let mut current_content = String::new();

    for line in body.lines() {
        if line.starts_with("## ") {
            if !current_section.is_empty() {
                sections.insert(current_section.to_lowercase(), current_content.trim().to_string());
            }
            current_section = line[3..].trim().to_string();
            current_content = String::new();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }
    if !current_section.is_empty() {
        sections.insert(current_section.to_lowercase(), current_content.trim().to_string());
    }

    sections
}

/// True if the line is a markdown horizontal rule (---, ***, ___ with optional whitespace).
fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 { return false; }
    let c = trimmed.chars().next().unwrap();
    (c == '-' || c == '*' || c == '_') && trimmed.chars().all(|ch| ch == c || ch == ' ')
}

/// Parse experience items from ## Experience section
fn parse_experience(content: &str) -> Vec<ExperienceItem> {
    let mut items = Vec::new();
    let mut current_line = String::new();
    let mut current_bullets: Vec<String> = Vec::new();
    let mut in_item = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("### ") {
            // Save previous item
            if in_item {
                let (title, company, dates) = parse_item_header(&current_line);
                if !title.is_empty() {
                    items.push(ExperienceItem { title, company, dates, place: None, bullets: current_bullets });
                }
            }
            current_line = trimmed[4..].trim().to_string();
            current_bullets = Vec::new();
            in_item = true;
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            current_bullets.push(trimmed[2..].trim().to_string());
        } else if in_item && !trimmed.is_empty() {
            // Continuation of previous bullet or header info
            if !current_bullets.is_empty() {
                let last = current_bullets.len() - 1;
                current_bullets[last].push_str(&format!(" {}", trimmed));
            }
        }
    }
    if in_item {
        let (title, company, dates) = parse_item_header(&current_line);
        if !title.is_empty() {
            items.push(ExperienceItem { title, company, dates, place: None, bullets: current_bullets });
        }
    }

    items
}

/// Parse "Title | Company | Dates" format
fn parse_item_header(line: &str) -> (String, String, Option<String>) {
    let parts: Vec<&str> = line.split('|').map(|p| p.trim()).collect();
    let title = parts.first().unwrap_or(&"").to_string();
    let company = parts.get(1).unwrap_or(&"").to_string();
    let dates = parts.get(2).map(|s| s.to_string()).filter(|s| !s.is_empty());
    (title, company, dates)
}

/// Parse education items from ## Education section
fn parse_education(content: &str) -> Vec<EducationItem> {
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            let parts: Vec<&str> = text.split('|').map(|p| p.trim()).collect();
            let degree = parts.first().unwrap_or(&"").to_string();
            let institution = parts.get(1).unwrap_or(&"").to_string();
            let year = parts.get(2).map(|s| s.to_string()).filter(|s| !s.is_empty());
            items.push(EducationItem { degree, institution, year, details: Vec::new() });
        }
    }
    items
}

fn strip_inline_markdown(s: &str) -> String {
    s.trim()
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .replace("**", "")
        .replace('*', "")
        .replace('`', "")
        .trim()
        .to_string()
}

fn split_place_dates(line: &str) -> (Option<String>, Option<String>) {
    let parts: Vec<String> = line.split('|').map(strip_inline_markdown).filter(|p| !p.is_empty()).collect();
    match parts.len() {
        0 => (None, None),
        1 => (None, Some(parts[0].clone())),
        _ => (Some(parts[0].clone()), Some(parts[1..].join(" | "))),
    }
}

fn contact_label_and_href(raw: &str) -> ContactItem {
    let clean = strip_inline_markdown(raw);
    if clean.contains('@') {
        return ContactItem { label: clean.clone(), href: Some(format!("mailto:{}", clean)) };
    }
    let lower = clean.to_lowercase();
    if lower.contains("linkedin") {
        return ContactItem { label: "LinkedIn".into(), href: Some(clean) };
    }
    if lower.contains("github") {
        return ContactItem { label: "GitHub".into(), href: Some(clean) };
    }
    if clean.starts_with("http://") || clean.starts_with("https://") {
        return ContactItem { label: "Portfolio".into(), href: Some(clean) };
    }
    ContactItem { label: clean, href: None }
}

fn parse_entry_section(content: &str) -> Vec<ExperienceItem> {
    let mut items = Vec::new();
    let mut org = String::new();
    let mut role = String::new();
    let mut place: Option<String> = None;
    let mut dates: Option<String> = None;
    let mut bullets: Vec<String> = Vec::new();

    let flush = |items: &mut Vec<ExperienceItem>, org: &mut String, role: &mut String, place: &mut Option<String>, dates: &mut Option<String>, bullets: &mut Vec<String>| {
        if !org.is_empty() {
            items.push(ExperienceItem {
                company: org.clone(),
                title: if role.is_empty() { org.clone() } else { role.clone() },
                place: place.clone(),
                dates: dates.clone(),
                bullets: bullets.clone(),
            });
        }
        org.clear();
        role.clear();
        *place = None;
        *dates = None;
        bullets.clear();
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("### ") {
            flush(&mut items, &mut org, &mut role, &mut place, &mut dates, &mut bullets);
            org = strip_inline_markdown(&trimmed[4..]);
        } else if trimmed.starts_with("**") && trimmed.ends_with("**") && role.is_empty() {
            role = strip_inline_markdown(trimmed);
        } else if !trimmed.is_empty() && dates.is_none() && !trimmed.starts_with('-') && !trimmed.starts_with("---") {
            let split = split_place_dates(trimmed);
            place = split.0;
            dates = split.1;
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            bullets.push(strip_inline_markdown(trimmed));
        }
    }
    flush(&mut items, &mut org, &mut role, &mut place, &mut dates, &mut bullets);
    items
}

fn parse_simple_bullet_section(content: &str) -> Vec<SimpleSectionItem> {
    content.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("- ") || l.starts_with("* "))
        .map(|l| {
            let text = l[2..].trim(); // strip bullet prefix
            // Try **Title** - context or **Title**, context pattern
            if text.starts_with("**") {
                if let Some(close_bold) = text[2..].find("**") {
                    let bold_end = close_bold + 2;
                    let title = text[2..bold_end].trim().to_string();
                    let after = text[bold_end+2..].trim();
                    if !after.is_empty() {
                        let context = if after.starts_with("- ") {
                            after[2..].trim().to_string()
                        } else if after.starts_with(", ") {
                            after[2..].trim().to_string()
                        } else {
                            after.to_string()
                        };
                        if !context.is_empty() {
                            return SimpleSectionItem { title, context: Some(context), meta: None, bullets: Vec::new() };
                        }
                    }
                    return SimpleSectionItem { title, context: None, meta: None, bullets: Vec::new() };
                }
            }
            // Fallback: plain bullet
            SimpleSectionItem { title: strip_inline_markdown(l), context: None, meta: None, bullets: Vec::new() }
        })
        .collect()
}

fn parse_resume_for_render(content: &str) -> Result<Resume, String> {
    if content.trim_start().starts_with("---") {
        return parse_resume(content);
    }

    let mut name = String::new();
    let mut subtitle = String::new();
    let mut contact_line = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            break;
        }
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        if trimmed.starts_with("# ") && name.is_empty() {
            name = strip_inline_markdown(&trimmed[2..]);
        } else if name.is_empty() {
            continue;
        } else if subtitle.is_empty() {
            let raw = strip_inline_markdown(trimmed);
            if raw.contains('|') {
                let parts: Vec<String> = raw.split('|').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
                subtitle = parts.join(", ");
            } else {
                subtitle = raw;
            }
        } else if contact_line.is_empty() {
            contact_line = strip_inline_markdown(trimmed);
        }
    }

    if name.is_empty() {
        name = "Unknown".into();
    }

    let contact_parts: Vec<String> = contact_line
        .split('|')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let contacts: Vec<ContactItem> = contact_parts.iter().map(|p| contact_label_and_href(p)).collect();
    let email = contact_parts
        .iter()
        .find(|p| p.contains('@'))
        .cloned()
        .unwrap_or_default();
    let linkedin = contact_parts
        .iter()
        .find(|p| p.to_lowercase().contains("linkedin"))
        .cloned();
    let location = contact_parts
        .iter()
        .rev()
        .find(|p| !p.contains('@') && !p.starts_with("http://") && !p.starts_with("https://"))
        .cloned();

    let sections = parse_sections(content);
    let summary_body = sections
        .get("summary")
        .map(|s| s.lines()
            .map(strip_inline_markdown)
            .filter(|l| !l.is_empty() && !is_horizontal_rule(l))
            .collect::<Vec<_>>()
            .join(" "))
        .unwrap_or_default();
    let summary = match (subtitle.is_empty(), summary_body.is_empty()) {
        (false, false) => Some(format!("{}. {}", subtitle.trim_end_matches('.'), summary_body)),
        (false, true) => Some(subtitle.clone()),
        (true, false) => Some(summary_body),
        (true, true) => None,
    };

    let mut education = Vec::new();
    if let Some(section) = sections.get("education") {
        let mut current_institution = String::new();
        let mut current_degree = String::new();
        let mut current_year: Option<String> = None;
        let mut current_notes: Vec<String> = Vec::new();

        let flush = |items: &mut Vec<EducationItem>, inst: &mut String, degree: &mut String, year: &mut Option<String>, notes: &mut Vec<String>| {
            if !inst.is_empty() || !degree.is_empty() {
                items.push(EducationItem {
                    institution: inst.clone(),
                    degree: degree.clone(),
                    year: year.clone(),
                    details: notes.clone(),
                });
            }
            inst.clear();
            degree.clear();
            *year = None;
            notes.clear();
        };

        for line in section.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("### ") {
                flush(&mut education, &mut current_institution, &mut current_degree, &mut current_year, &mut current_notes);
                current_institution = strip_inline_markdown(&trimmed[4..]);
            } else if trimmed.starts_with("**") && trimmed.ends_with("**") {
                current_degree = strip_inline_markdown(trimmed);
            } else if !trimmed.is_empty() && current_year.is_none() && !trimmed.starts_with('-') {
                current_year = Some(strip_inline_markdown(trimmed));
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                current_notes.push(strip_inline_markdown(trimmed));
            }
        }
        flush(&mut education, &mut current_institution, &mut current_degree, &mut current_year, &mut current_notes);
    }

    let mut skills = Vec::new();
    let mut skill_rows = Vec::new();
    if let Some(section) = sections.get("skills") {
        for line in section.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "---" || trimmed.starts_with("### ") {
                continue;
            }
            let clean = strip_inline_markdown(trimmed);
            if clean.is_empty() {
                continue;
            }
            if let Some((label, value)) = clean.split_once(':') {
                skill_rows.push(SkillItem { label: format!("{}:", label.trim()), value: value.trim().to_string() });
            } else {
                skills.push(clean);
            }
        }
    }

    let experience = sections.get("experience").map(|s| parse_entry_section(s)).unwrap_or_default();
    let projects = sections.get("projects / research")
        .or_else(|| sections.get("projects"))
        .or_else(|| sections.get("research"))
        .map(|s| parse_entry_section(s))
        .unwrap_or_default();
    let awards = sections.get("awards").map(|s| parse_simple_bullet_section(s)).unwrap_or_default();
    let certifications = sections.get("certifications").map(|s| parse_simple_bullet_section(s)).unwrap_or_default();

    Ok(Resume {
        meta: ResumeMeta {
            name,
            email,
            phone: None,
            location,
            linkedin,
            summary,
            subtitle: if subtitle.is_empty() { None } else { Some(subtitle) },
            contacts,
        },
        experience,
        skills,
        skill_rows,
        education,
        projects,
        awards,
        certifications,
    })
}

// ── Validator ───────────────────────────────────────────────────

/// Validates a resume.md file and returns structured issues.
pub fn validate_resume_content(content: &str) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Phase 1: Check frontmatter
    let lines: Vec<&str> = content.lines().collect();
    let mut in_frontmatter = false;
    let mut frontmatter_lines = 0usize;
    let mut frontmatter_range = (0usize, 0usize);

    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
                frontmatter_range.0 = i;
            } else {
                in_frontmatter = false;
                frontmatter_range.1 = i;
                frontmatter_lines = i - frontmatter_range.0 + 1;
                break;
            }
        }
    }

    let fm_content: String = if frontmatter_lines > 0 {
        lines[frontmatter_range.0 + 1..frontmatter_range.1].join("\n")
    } else {
        String::new()
    };

    // Check required frontmatter fields
    let has_name = fm_content.contains("name:");
    let has_email = fm_content.contains("email:");

    if !has_name {
        errors.push(ValidationIssue {
            field: "frontmatter.name".into(),
            message: "Missing required field: name".into(),
            line: Some(1),
            severity: "error".into(),
        });
    }
    if !has_email {
        errors.push(ValidationIssue {
            field: "frontmatter.email".into(),
            message: "Missing required field: email".into(),
            line: Some(1),
            severity: "error".into(),
        });
    }

    // Phase 2: Check body sections
    let body_start = if frontmatter_lines > 0 { frontmatter_range.1 + 1 } else { 0 };
    let body_lines: Vec<(usize, &str)> = lines.iter().enumerate().skip(body_start).map(|(i, l)| (i, *l)).collect();

    let mut has_experience = false;
    let mut has_skills = false;
    let mut has_education = false;
    let mut experience_item_count = 0;

    for (i, line) in &body_lines {
        let trimmed = line.trim();

        if trimmed.starts_with("## ") {
            match trimmed[3..].trim().to_lowercase().as_str() {
                "experience" => has_experience = true,
                "skills" => has_skills = true,
                "education" => has_education = true,
                _ => {}
            }
        }

        if trimmed.starts_with("### ") {
            experience_item_count += 1;
            // Check format: Title | Company | Dates
            let header = &trimmed[4..].trim();
            if !header.contains('|') {
                errors.push(ValidationIssue {
                    field: format!("experience[{}]", experience_item_count - 1),
                    message: "Experience item must use format: Title | Company | Dates".into(),
                    line: Some(*i + 1),
                    severity: "error".into(),
                });
            } else if !header.contains(" | ") {
                warnings.push(ValidationIssue {
                    field: format!("experience[{}]", experience_item_count - 1),
                    message: "Experience item missing company separator (Title | Company)".into(),
                    line: Some(*i + 1),
                    severity: "warning".into(),
                });
            }
        }
    }

    if !has_experience {
        errors.push(ValidationIssue {
            field: "body.section".into(),
            message: "Missing required section: ## Experience".into(),
            line: None,
            severity: "error".into(),
        });
    }

    if !has_skills {
        warnings.push(ValidationIssue {
            field: "body.section".into(),
            message: "Missing recommended section: ## Skills".into(),
            line: None,
            severity: "warning".into(),
        });
    }

    if !has_education {
        warnings.push(ValidationIssue {
            field: "body.section".into(),
            message: "Missing recommended section: ## Education".into(),
            line: None,
            severity: "warning".into(),
        });
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

// ── Typst Escape ────────────────────────────────────────────────

/// Escapes Typst special characters in plain text content.
/// Characters: \\ # $ _ * [ ] { } @ ~
pub fn escape_typst_text(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '#' => result.push_str("\\#"),
            '$' => result.push_str("\\$"),
            '_' => result.push_str("\\_"),
            '*' => result.push_str("\\*"),
            '[' => result.push_str("\\["),
            ']' => result.push_str("\\]"),
            '{' => result.push_str("\\{"),
            '}' => result.push_str("\\}"),
            '@' => result.push_str("\\@"),
            '~' => result.push_str("\\~"),
            _ => result.push(ch),
        }
    }
    result
}

// ── Typst Generator ─────────────────────────────────────────────

/// Converts a Resume to a Typst document string using the resume_2 style.
/// Produces a clean, professional resume layout matching the approved visual design.
pub fn resume_to_typst(resume: &Resume) -> String {
    let e = escape_typst_text;
    let mut s = String::new();

    // ── Helpers matching docs/typst/resume_2_sample.typ ──
    s.push_str("#let muted = rgb(\"666666\")\n");
    s.push_str("#let rule = rgb(\"888888\")\n");
    s.push_str("#let dark = rgb(\"050505\")\n");
    s.push_str("#let pipe = text(fill: muted)[ | ]\n\n");

    s.push_str("#let section(title, body) = {\n");
    s.push_str("  v(0.23em)\n");
    s.push_str("  set par(leading: 0em)\n");
    s.push_str("  text(fill: muted, size: 15pt, weight: \"bold\")[#title]\n");
    s.push_str("  v(-1.15em)\n");
    s.push_str("  line(length: 100%, stroke: 0.5pt + rule)\n");
    s.push_str("  v(-0.25em)\n");
    s.push_str("  set par(leading: 0.46em)\n");
    s.push_str("  body\n");
    s.push_str("}\n\n");

    s.push_str("#let skill-row(label, value) = grid(\n");
    s.push_str("  columns: (1.35in, 1fr),\n");
    s.push_str("  column-gutter: 0.12in,\n");
    s.push_str("  align: top,\n");
    s.push_str("  text(weight: \"regular\", fill: rgb(\"222222\"), size: 10pt)[#label],\n");
    s.push_str("  text(size: 10pt)[#value],\n");
    s.push_str(")\n\n");

    s.push_str("#let entry(org, role, place: none, dates: none, body) = {\n");
    s.push_str("  grid(\n");
    s.push_str("    columns: (1fr, 2.0in),\n");
    s.push_str("    column-gutter: 0.2in,\n");
    s.push_str("    align: (left, right),\n");
    s.push_str("    {\n");
    s.push_str("      text(size: 13pt, weight: \"bold\", fill: dark)[#org]\n");
    s.push_str("      linebreak()\n");
    s.push_str("      role\n");
    s.push_str("    },\n");
    s.push_str("    {\n");
    s.push_str("      if place != none { text(fill: muted)[#place] }\n");
    s.push_str("      if place != none and dates != none { linebreak() }\n");
    s.push_str("      if dates != none { text(fill: muted)[#dates] }\n");
    s.push_str("    },\n");
    s.push_str("  )\n");
    s.push_str("  v(-0.55em)\n");
    s.push_str("  body\n");
    s.push_str("  set par(leading: 0.46em)\n");
    s.push_str("  v(0.27em)\n");
    s.push_str("}\n\n");

    // ── Page & text setup (resume_2 style) ──
    s.push_str("#set page(paper: \"us-letter\", margin: (x: 0.78in, y: 0.62in))\n");
    s.push_str("#set text(font: (\"Avenir Next\", \"Inter\", \"Helvetica Neue\"), size: 10.5pt, fill: rgb(\"111111\"))\n");
    s.push_str("#set par(leading: 0.46em, justify: false)\n");
    s.push_str("#set list(indent: 0pt, body-indent: 0.64em, spacing: 0.38em)\n\n");

    // ── Header: name ──
    let meta = &resume.meta;
    // Split name into first and last for the styled layout
    let name_parts: Vec<&str> = meta.name.splitn(2, ' ').collect();
    let first = name_parts.first().unwrap_or(&"").to_string();
    let rest = if name_parts.len() > 1 { name_parts[1..].join(" ") } else { String::new() };
    s.push_str(&format!(
        "#text(size: 24pt, fill: muted, weight: \"regular\")[{} ]#text(size: 24pt, weight: \"bold\", fill: dark)[{}]\n",
        e(&first), e(&rest)
    ));
    s.push_str("#v(0.15em)\n");

    // ── Header: positioning line ──
    if let Some(ref subtitle) = meta.subtitle {
        if !subtitle.is_empty() {
            s.push_str(&format!("#text(size: 14pt, weight: \"bold\", fill: dark)[{}]\n", e(subtitle)));
            s.push_str("#v(0.15em)\n");
        }
    } else if let Some(ref summary) = meta.summary {
        if !summary.is_empty() {
            let pos_line = summary.split(|c: char| c == '.' || c == '!').next()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .unwrap_or(summary)
                .chars().take(120).collect::<String>();
            s.push_str(&format!("#text(size: 14pt, weight: \"bold\", fill: dark)[{}]\n", e(&pos_line)));
            s.push_str("#v(0.15em)\n");
        }
    }

    // ── Header: contact line ──
    let mut contact_parts: Vec<String> = Vec::new();
    for item in &meta.contacts {
        if let Some(ref href) = item.href {
            contact_parts.push(format!("#link(\"{}\")[#text(size: 10pt, fill: muted)[{}]]", e(href), e(&item.label)));
        } else {
            contact_parts.push(format!("#text(size: 10pt, fill: muted)[{}]", e(&item.label)));
        }
    }
    if contact_parts.is_empty() {
        if !meta.email.is_empty() {
            contact_parts.push(format!("#link(\"mailto:{}\")[#text(size: 10pt, fill: muted)[{}]]", e(&meta.email), e(&meta.email)));
        }
        if let Some(ref l) = meta.location {
            contact_parts.push(format!("#text(size: 10pt, fill: muted)[{}]", e(l)));
        }
        if let Some(ref li) = meta.linkedin {
            contact_parts.push(format!("#link(\"{}\")[#text(size: 10pt, fill: muted)[LinkedIn]]", e(li)));
        }
    }
    if !contact_parts.is_empty() {
        let contact_line = contact_parts.join("#pipe");
        s.push_str(&format!("{}\n", contact_line));
    }

    // ── Summary body text ──
    if let Some(ref summary) = meta.summary {
        if !summary.is_empty() {
            s.push_str(&format!("\n#v(0.5em)\n#text(size: 10.5pt)[{}]\n", e(summary)));
            s.push_str("#v(0.15em)\n");
        }
    }
    s.push_str("\n");

    // ── Education ──
    if !resume.education.is_empty() {
        s.push_str("#section(\"Education\")[\n");
        for edu in &resume.education {
            let (edu_place, edu_dates) = edu.year.as_deref()
                .map(split_place_dates)
                .unwrap_or((None, None));
            s.push_str("  #grid(\n");
            s.push_str("    columns: (1fr, 2.0in),\n");
            s.push_str("    column-gutter: 0.2in,\n");
            s.push_str("    align: (left, right),\n");
            s.push_str("    [\n");
            s.push_str(&format!("      #text(size: 12pt, weight: \"bold\", fill: dark)[{}]\n", e(&edu.institution)));
            s.push_str("      #linebreak()\n");
            s.push_str(&format!("      #text(weight: \"bold\", fill: dark)[{}]\n", e(&edu.degree)));
            s.push_str("    ],\n");
            s.push_str("    [\n");
            if let Some(ref place) = edu_place {
                s.push_str(&format!("      #text(fill: muted)[{}]\n", e(place)));
            }
            if edu_place.is_some() && edu_dates.is_some() {
                s.push_str("      #linebreak()\n");
            }
            if let Some(ref dates) = edu_dates {
                s.push_str(&format!("      #text(fill: muted)[{}]\n", e(dates)));
            }
            s.push_str("    ],\n");
            s.push_str("  )\n");
            if !edu.details.is_empty() {
                s.push_str("  #v(-0.55em)\n");
                for detail in &edu.details {
                    s.push_str(&format!("  - {}\n", e(detail)));
                }
            }
            s.push_str("  #v(0.27em)\n");
        }
        s.push_str("]\n");
    }

    // ── Skills ──
    if !resume.skill_rows.is_empty() || !resume.skills.is_empty() {
        s.push_str("#section(\"Skills\")[\n");
        for (idx, skill) in resume.skill_rows.iter().enumerate() {
            if idx > 0 { s.push_str("  #v(0.35em)\n"); }
            s.push_str(&format!("  #skill-row(\"{}\", [{}])\n", e(&skill.label), e(&skill.value)));
        }
        if !resume.skills.is_empty() {
            if !resume.skill_rows.is_empty() { s.push_str("  #v(0.35em)\n"); }
            s.push_str(&format!("  #skill-row(\"Skills:\", [{}])\n", e(&resume.skills.join(", "))));
        }
        s.push_str("]\n");
    }

    // ── Experience ──
    if !resume.experience.is_empty() {
        s.push_str("#section(\"Experience\")[\n");
        for exp in &resume.experience {
            s.push_str(&format!(
                "  #entry(\n    [{}],\n    [{}],\n    place: [{}],\n    dates: [{}],\n  )[\n",
                e(&exp.company), e(&exp.title), e(exp.place.as_deref().unwrap_or("")),
                e(exp.dates.as_deref().unwrap_or(""))
            ));
            for bullet in &exp.bullets {
                s.push_str(&format!("    - {}\n", e(bullet)));
            }
            s.push_str("  ]\n");
        }
        s.push_str("]\n");
    }

    // ── Projects / Research ──
    if !resume.projects.is_empty() {
        s.push_str("#section(\"Projects / Research\")[\n");
        for project in &resume.projects {
            s.push_str(&format!(
                "  #entry(\n    [{}],\n    [{}],\n    place: [{}],\n    dates: [{}],\n  )[\n",
                e(&project.company), e(&project.title), e(project.place.as_deref().unwrap_or("")),
                e(project.dates.as_deref().unwrap_or(""))
            ));
            for bullet in &project.bullets {
                s.push_str(&format!("    - {}\n", e(bullet)));
            }
            s.push_str("  ]\n");
        }
        s.push_str("]\n");
    }

    // ── Awards ──
    if !resume.awards.is_empty() {
        s.push_str("#section(\"Awards\")[\n");
        for award in &resume.awards {
            if let Some(ref context) = award.context {
                s.push_str(&format!("  - #text(weight: \"bold\")[{}] {}\n", e(&award.title), e(context)));
            } else {
                s.push_str(&format!("  - {}\n", e(&award.title)));
            }
        }
        s.push_str("]\n");
    }

    // ── Certifications ──
    if !resume.certifications.is_empty() {
        s.push_str("#section(\"Certifications\")[\n");
        for cert in &resume.certifications {
            if let Some(ref context) = cert.context {
                s.push_str(&format!("  - #text(weight: \"bold\")[{}] {}\n", e(&cert.title), e(context)));
            } else {
                s.push_str(&format!("  - {}\n", e(&cert.title)));
            }
        }
        s.push_str("]\n");
    }

    s
}

// ── Bundled Typst Binary Resolution ────────────────────────────

/// Resolves the path to the bundled Typst binary.
/// Checks development paths first, then production bundle locations.
fn resolve_typst_binary() -> Result<PathBuf, String> {
    // Dev paths (project-root relative)
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    for rel in &["src-tauri/resources/bin/typst", "resources/bin/typst"] {
        let p = cwd.join(rel);
        if p.exists() {
            return Ok(p.canonicalize().map_err(|e| e.to_string())?);
        }
    }

    // Production bundle paths (macOS .app bundle layout)
    if let Ok(exe) = std::env::current_exe() {
        // App.app/Contents/MacOS/executable → Resources/...
        // Tauri preserves configured resource paths, so resources/bin/typst may
        // be bundled as either Resources/resources/bin/typst or Resources/bin/typst
        // depending on config/version/layout.
        if let Some(parent) = exe.parent() {
            for rel in &["../Resources/resources/bin/typst", "../Resources/bin/typst"] {
                let p = parent.join(rel);
                if p.exists() {
                    return Ok(p.canonicalize().map_err(|e| e.to_string())?);
                }
            }
        }
    }

    Err("Typst binary not found. Run the following to install:\n  ./scripts/install-typst-resource.sh".into())
}

// ── Tauri commands ─────────────────────────────────────────────

fn validate_personalized_resume_job_path(job_path: &str) -> Result<(), String> {
    if job_path.trim().is_empty() {
        return Err("Resume render path cannot be empty".into());
    }

    let path = Path::new(job_path);
    if path.is_absolute() {
        return Err("Resume render path must be workspace-relative".into());
    }

    let mut has_resume_prefix = false;
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                if part.to_string_lossy() == "resume" {
                    has_resume_prefix = true;
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("Resume render path cannot contain parent, root, or prefix components".into());
            }
        }
    }

    if !has_resume_prefix {
        return Err("PDF rendering is only allowed for resume.md files under resume/ folders. The provided path does not contain resume/.".into());
    }

    Ok(())
}

/// Validates a resume.md file at the given project/job path.
#[tauri::command]
pub fn validate_resume(input: ResumeInput) -> Result<ValidationResult, String> {
    let project = super::project_root(&input.project_slug)?;
    let resume_path = project.join(&input.job_path).join("resume.md");

    if !resume_path.exists() {
        return Ok(ValidationResult {
            valid: false,
            errors: vec![ValidationIssue {
                field: "file".into(),
                message: format!("resume.md not found at: {}", resume_path.display()),
                line: None,
                severity: "error".into(),
            }],
            warnings: vec![],
        });
    }

    let content = std::fs::read_to_string(&resume_path)
        .map_err(|e| format!("Failed to read resume.md: {}", e))?;

    Ok(validate_resume_content(&content))
}

/// Renders resume.md to resume.pdf via bundled Typst.
/// Does not write intermediate .typ files to the workspace.
#[tauri::command]
pub fn render_resume_pdf(input: ResumeInput) -> Result<RenderOutput, String> {
    validate_personalized_resume_job_path(&input.job_path)?;
    let project = super::project_root(&input.project_slug)?;
    let job_dir = project.join(&input.job_path);
    let pdf_path = job_dir.join("resume.pdf");

    // Resolve the bundled Typst binary
    let typst_path = resolve_typst_binary()?;

    // Read and parse resume.md
    let resume_path = job_dir.join("resume.md");
    if !resume_path.exists() {
        return Err(format!("resume.md not found at: {}", resume_path.display()));
    }
    let content = std::fs::read_to_string(&resume_path)
        .map_err(|e| format!("Failed to read resume.md: {}", e))?;
    let resume = parse_resume(&content)
        .map_err(|e| format!("Failed to parse resume: {}", e))?;

    // Generate Typst document string
    let typst_content = resume_to_typst(&resume);

    // Write temporary .typ file (outside workspace, cleaned on drop)
    let tmp_file = tempfile::Builder::new()
        .suffix(".typ")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    let tmp_path = tmp_file.path().to_path_buf();
    std::fs::write(&tmp_path, &typst_content)
        .map_err(|e| format!("Failed to write temp typst file: {}", e))?;

    // Ensure output directory exists
    if let Some(parent) = pdf_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Run bundled typst compile
    let output = ProcessCmd::new(&typst_path)
        .arg("compile")
        .arg(&tmp_path)
        .arg(&pdf_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "Typst binary not found at {}. Run scripts/install-typst-resource.sh",
                    typst_path.display()
                )
            } else {
                format!("Failed to run typst: {}", e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut compile_errors = String::new();
        for line in stdout.lines().chain(stderr.lines()) {
            let l = line.trim();
            if l.contains("error:") || l.contains("Error:") || l.contains("panicked") {
                compile_errors.push_str(l);
                compile_errors.push('\n');
            }
        }
        if compile_errors.is_empty() {
            compile_errors = format!("Typst compilation failed (exit code: {:?})", output.status.code());
        }

        return Err(format!("Typst compilation failed:\n{}", compile_errors));
    }

    if !pdf_path.exists() {
        return Err("Typst completed but resume.pdf was not generated".into());
    }

    Ok(RenderOutput {
        pdf_path: pdf_path.to_string_lossy().to_string(),
        compile_errors: None,
    })
}

/// Renders any workspace-relative .md file to PDF via bundled Typst.
/// The path must be workspace-relative, no "..", and must point to a
/// file or folder on disk.
pub fn render_md_file_to_pdf(
    md_path: &std::path::Path,
    output_pdf_path: &std::path::Path,
) -> Result<RenderOutput, String> {
    // Resolve the bundled Typst binary
    let typst_path = resolve_typst_binary()?;

    // Read and parse the .md file
    if !md_path.exists() {
        return Err(format!(".md file not found at: {}", md_path.display()));
    }
    let content = std::fs::read_to_string(md_path)
        .map_err(|e| format!("Failed to read {}: {}", md_path.display(), e))?;
    let resume = parse_resume_for_render(&content)
        .map_err(|e| format!("Failed to parse resume: {}", e))?;

    // Generate Typst document string
    let typst_content = resume_to_typst(&resume);

    // Write temporary .typ file (outside workspace, cleaned on drop)
    let tmp_file = tempfile::Builder::new()
        .suffix(".typ")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    let tmp_path = tmp_file.path().to_path_buf();
    std::fs::write(&tmp_path, &typst_content)
        .map_err(|e| format!("Failed to write temp typst file: {}", e))?;

    // Ensure output directory exists
    if let Some(parent) = output_pdf_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Run bundled typst compile
    let output = ProcessCmd::new(&typst_path)
        .arg("compile")
        .arg(&tmp_path)
        .arg(output_pdf_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "Typst binary not found at {}. Run scripts/install-typst-resource.sh",
                    typst_path.display()
                )
            } else {
                format!("Failed to run typst: {}", e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        let debug_typ_path = output_pdf_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(".debug")
            .join(format!(
                "{}-{}.typ",
                output_pdf_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("resume-render"),
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            ));
        if let Some(parent) = debug_typ_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let debug_write_note = match std::fs::write(&debug_typ_path, &typst_content) {
            Ok(_) => format!("Generated Typst debug file: {}", debug_typ_path.display()),
            Err(e) => format!("Failed to write Typst debug file {}: {}", debug_typ_path.display(), e),
        };

        let stdout_s = stdout.trim();
        let stderr_s = stderr.trim();
        return Err(format!(
            "Typst compilation failed (exit code: {:?})\nInput Markdown: {}\nOutput PDF: {}\nTypst binary: {}\n{}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status.code(),
            md_path.display(),
            output_pdf_path.display(),
            typst_path.display(),
            debug_write_note,
            if stdout_s.is_empty() { "<empty>" } else { stdout_s },
            if stderr_s.is_empty() { "<empty>" } else { stderr_s }
        ));
    }

    if !output_pdf_path.exists() {
        return Err("Typst completed but PDF was not generated".into());
    }

    Ok(RenderOutput {
        pdf_path: output_pdf_path.to_string_lossy().to_string(),
        compile_errors: None,
    })
}

/// Renders a workspace-relative .md file or folder of .md files to PDF.
/// - If `path` is a file: output is `pdf/<stem>-<YYYYMMDD-HHMMSS>.pdf`
/// - If `path` is a folder: output is `pdf/<folder-name>/<stem>.pdf` per file
#[tauri::command]
pub fn render_path_to_pdf(input: RenderPathInput) -> Result<Vec<RenderOutput>, String> {
    if input.path.trim().is_empty() {
        return Err("Render path cannot be empty".into());
    }

    let project = super::project_root(&input.project_slug)?;
    let target = project.join(&input.path);

    // Validate workspace-relative, no ".."
    if input.path.contains("..") || std::path::Path::new(&input.path).is_absolute() {
        return Err("Render path must be workspace-relative and cannot contain ..".into());
    }
    if !target.exists() {
        return Err(format!("Path does not exist: {}", input.path));
    }

    // Restrict rendering to paths under resume/
    let trimmed_path = input.path.trim().trim_start_matches("./");
    if !trimmed_path.starts_with("resume/") && trimmed_path != "resume" {
        return Err("PDF rendering is only allowed for .md files or folders under resume/".into());
    }

    // Ensure pdf/ parent dirs exist
    let pdf_base = project.join("pdf");
    std::fs::create_dir_all(&pdf_base).map_err(|e| e.to_string())?;

    if target.is_file() {
        // Single file mode
        if !target.extension().and_then(|s| s.to_str()).map_or(false, |e| e == "md") {
            return Err(format!("Not a .md file: {}", input.path));
        }
        let stem = target.file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid filename")?;
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let output_pdf = pdf_base.join(format!("{}-{}.pdf", stem, timestamp));
        let rendered = render_md_file_to_pdf(&target, &output_pdf)?;
        Ok(vec![rendered])
    } else {
        // Folder mode: scan for .md files
        let folder_name = target.file_name()
            .and_then(|s| s.to_str())
            .ok_or("Invalid folder name")?;
        let folder_pdf_base = pdf_base.join(folder_name);
        std::fs::create_dir_all(&folder_pdf_base).map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(&target)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let stem = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let output_pdf = folder_pdf_base.join(format!("{}.pdf", stem));
            match render_md_file_to_pdf(&path, &output_pdf) {
                Ok(r) => results.push(r),
                Err(e) => results.push(RenderOutput {
                    pdf_path: format!("{}.md -> ERROR: {}", stem, e),
                    compile_errors: Some(e),
                }),
            }
        }

        if results.is_empty() {
            return Err(format!("No .md files found in folder: {}", input.path));
        }
        Ok(results)
    }
}

/// Validates and renders resume.md to resume.pdf — one-shot pipeline.
#[tauri::command]
pub fn render_resume(input: ResumeInput) -> Result<RenderOutput, String> {
    // Step 1: Validate
    let validation = validate_resume(input.clone())?;
    if !validation.valid {
        let error_msgs: Vec<String> = validation.errors.iter()
            .map(|e| format!("  - {}: {}", e.field, e.message))
            .collect();
        return Err(format!("Resume validation failed:\n{}", error_msgs.join("\n")));
    }

    // Step 2: Render to PDF directly
    render_resume_pdf(input)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
}

/// Returns the list of available agent skills for the UI skills menu.
#[tauri::command]
pub fn list_skills() -> Vec<SkillInfo> {
    SKILLS.iter().map(|s| SkillInfo {
        name: s.name.to_string(),
        description: s.description.to_string(),
    }).collect()
}



pub const SKILL_FIX_RENDER: &str = concat!(
    "## /fix-render — Resume PDF debug skill\n",
    "\n",
    "Use this when the user mentions: render, PDF, fix the resume, typst, or /fix-render.\n",
    "\n",
    "### Resume.md schema\n",
    "```\n",
    "---\n",
    "name: \"...\"                    ← required\n",
    "email: \"...\"                   ← required\n",
    "phone: \"...\"                   ← optional\n",
    "location: \"...\"                ← optional\n",
    "linkedin: \"...\"                ← optional\n",
    "summary: |                       ← optional\n",
    "  A short paragraph...\n",
    "---\n",
    "\n",
    "## Experience\n",
    "\n",
    "### Title | Company | 2022-Present\n",
    "- Bullet point\n",
    "\n",
    "## Skills\n",
    "Skill1, Skill2, Skill3\n",
    "\n",
    "## Education\n",
    "Degree | Institution | Year\n",
    "```\n",
    "\n",
    "### Common validation issues\n",
    "- Missing `name:` or `email:` in frontmatter → read `profile/RESUME.md` for the user's info, fill it in.\n",
    "- Experience item missing company → format should be `### Title | Company | Dates`. If pipes are missing, add them.\n",
    "- Missing `## Experience` section → create one with at least one entry.\n",
    "- Missing `## Skills` section → recommended but not required, add from `profile/RESUME.md` if available.\n",
    "\n",
    "### Workflow\n",
    "1. Read `resume.md` from the job path using `read_file`.\n",
    "2. Check each section against the schema above.\n",
    "3. Fix issues using `write_file` — only edit `resume.md`, never `.typ` or `.pdf`.\n",
    "4. For missing user info (name, email), read `profile/RESUME.md` from the workspace root.\n",
    "5. Only work with Markdown files under `resume/`. PDF rendering is handled by the app: after fixing an eligible file, tell the user to right-click the file or folder and choose \"Render to PDF\" from the context menu.\n",
    "\n",
    "### PDF generation\n",
    "PDF rendering is handled by the app UI via the bundled Typst binary. The agent does not render PDFs.\n",
    "After fixing resume.md, tell the user to right-click the file or folder and choose \"Render to PDF\".\n",
);


pub const SKILL_RESUME_BUILDER: &str = concat!(
    "## /resume-builder\n",
    "\n",
    "Tailors the user's base resume to match one or more job descriptions.\n",
    "\n",
    "### Workflow\n",
    "\n",
    "1. The user will provide either:\n",
    "   - A single job .md file (from hunt_run/ or import-links/)\n",
    "   - A folder containing job .md files (from hunt_run/ or import-links/)\n",
    "\n",
    "2. Read the user's base resume, Markdown example, and context:\n",
    "   - Read `profile/RESUME.md` for the user's complete factual source material: experience, skills, projects, education, achievements, preferences they added, and wording style. The user may edit this file freely, so extract facts flexibly rather than assuming one exact structure.\n",
    "   - Read `profile/RESUME_TEMPLATE.md` as the polished Markdown example and target output structure/style. Use it as the guide for section order, heading shape, contact line shape, entry formatting, and concise professional wording style.\n",
    "   - Do NOT treat `profile/RESUME_TEMPLATE.md` as a content limit. If the template shows one example bullet, project, certification, or experience item, that does not mean the output must have only one. Preserve the relevant amount of real user information from `profile/RESUME.md`.\n",
    "   - Read `profile/USER.md` for goals, constraints, target-role preferences, location/visa context, and personal positioning notes.\n",
    "\n",
    "3. Tailor the resume:\n",
    "   - Tailor to the actual job role first. Do not force the resume toward the user's preferred AI positioning if the job is primarily full-stack, product, mobile, data, or another role.\n",
    "   - Identify the job's primary hiring signal from its title, responsibilities, required stack, and company context, then make the headline, summary, skills order, and bullets serve that signal.\n",
    "   - For full-stack/product engineering roles, emphasize feature ownership, frontend/backend implementation, data models, APIs, dashboards/CMS, UX details, speed, and small-team execution before AI/RAG depth.\n",
    "   - For AI/FDE/LLM roles, emphasize RAG, structured outputs, tool use, workflow state, evaluation/reliability, integrations, dashboards, deployment, and business workflow understanding.\n",
    "   - Reorder skills to match what the job asks for, in the order they ask, but only include skills supported by the base resume/user profile.\n",
    "   - Rephrase bullet points using keywords from the job description where the user's real experience supports it.\n",
    "   - Emphasize relevant experience — move matching roles/projects higher when helpful.\n",
    "   - Match the positioning line / summary to the target role and avoid over-indexing on unrelated strengths.\n",
    "   - Preserve the user's real facts, strongest relevant evidence, and credible wording style from the base resume, but make the finished file look structurally like `profile/RESUME_TEMPLATE.md`.\n",
    "   - Include more or fewer bullets/entries than the template when the user's real experience and the target job justify it. Prefer concise relevance over matching the sample's exact counts.\n",
    "     `# Name`, subtitle line, contact line, then `## Summary`, `## Education`, `## Skills`, `## Experience`, and optional `## Projects / Research`, `## Awards`, `## Certifications`.\n",
    "   - For each Education/Experience/Project entry, use this shape: `### Organization or Project`, next line `**Role/Degree**`, next line `Location or URL | Dates`, then bullet points.\n",
    "   - In Skills, use category lines like `**Category:** comma-separated skills`.\n",
    "   - Do NOT invent any experience, skills, dates, employers, or credentials not in the base resume.\n",
    "   - Do NOT add qualifications the user doesn't have.\n",
    "\n",
    "4. Write output:\n",
    "   - Single file: write to `resume/<source-path-with-parent-prefix>.md`\n",
    "     Example: job file `hunt_run/yc-1/jobs-2026-06-06/001-full-stack-engineer.md`\n",
    "     → Output: `resume/hunt_run/yc-1-001-full-stack-engineer.md`\n",
    "   - Batch folder: write all tailored resumes into `resume/<source-folder>/`\n",
    "     Example: folder `hunt_run/yc-1/jobs-2026-06-06/` (containing 001-*.md, 002-*.md, ...)\n",
    "     → Output: `resume/hunt_run/yc-1/jobs-2026-06-06/001-full-stack-engineer.md`,\n",
    "               `resume/hunt_run/yc-1/jobs-2026-06-06/002-backend-engineer.md`, etc.\n",
    "\n",
    "5. After writing, inform the user what was created and where.\n",
    "\n",
    "### PDF rendering\n",
    "Do NOT attempt to render PDFs yourself. PDF rendering is handled by the app UI.\n",
    "After writing or updating tailored resumes, tell the user to right-click the resume file or folder\n",
    "in the file tree and choose \"Render to PDF\" from the context menu.\n",
);
