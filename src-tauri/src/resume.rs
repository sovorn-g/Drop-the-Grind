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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExperienceItem {
    pub title: String,
    pub company: String,
    pub dates: Option<String>,
    pub bullets: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EducationItem {
    pub degree: String,
    pub institution: String,
    pub year: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Resume {
    pub meta: ResumeMeta,
    pub experience: Vec<ExperienceItem>,
    pub skills: Vec<String>,
    pub education: Vec<EducationItem>,
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
    // PDF rendering is restricted to paths under personalized-resume/.
    pub job_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RenderOutput {
    pub pdf_path: String,
    pub compile_errors: Option<String>,
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
        name: "/resume-builder-all",
        description: "Tailor one personalized resume for every job in a selected hunt jobs folder",
        keyword_patterns: &["/resume-builder-all", "resume-builder-all", "resume builder all", "build all resumes", "tailor all resumes", "personalized resumes for all jobs"],
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
        "/resume-builder-all" => Some(SKILL_RESUME_BUILDER_ALL),
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

    Ok(Resume { meta, experience, skills, education })
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

    ResumeMeta { name, email, phone, location, linkedin, summary }
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
                    items.push(ExperienceItem { title, company, dates, bullets: current_bullets });
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
            items.push(ExperienceItem { title, company, dates, bullets: current_bullets });
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
            items.push(EducationItem { degree, institution, year });
        }
    }
    items
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

/// Converts a Resume to a Typst document string.
/// Produces a clean, professional resume layout using bundled Typst.
pub fn resume_to_typst(resume: &Resume) -> String {
    let e = escape_typst_text;
    let mut s = String::new();

    // ── Page & text setup ──
    s.push_str("#set page(paper: \"us-letter\", margin: (x: 0.75in, y: 0.7in))\n");
    s.push_str("#set text(font: (\"Helvetica\", \"Arial\"), size: 10.5pt)\n");
    s.push_str("#set par(leading: 0.55em)\n\n");

    // ── Header: name ──
    let meta = &resume.meta;
    s.push_str(&format!("#align(center, text(size: 22pt, weight: \"bold\")[{}])\n", e(&meta.name)));

    // ── Header: contact line ──
    let mut contact_parts: Vec<String> = Vec::new();
    if !meta.email.is_empty() {
        contact_parts.push(format!("{}", e(&meta.email)));
    }
    if let Some(ref p) = meta.phone {
        contact_parts.push(e(p));
    }
    if let Some(ref l) = meta.location {
        contact_parts.push(e(l));
    }
    if let Some(ref li) = meta.linkedin {
        contact_parts.push(e(li));
    }
    if !contact_parts.is_empty() {
        // Build contact line: each part is escaped, separators are literal
        let contact_line = contact_parts.join(" \\| ");
        s.push_str(&format!("#align(center, text(size: 9pt)[{}])\n\n", contact_line));
    }

    // ── Summary ──
    if let Some(ref summary) = meta.summary {
        if !summary.is_empty() {
            s.push_str(&format!("{}\n\n", e(summary)));
        }
    }

    // ── Separator ──
    s.push_str("#line(length: 100%)\n\n");

    // ── Experience ──
    if !resume.experience.is_empty() {
        s.push_str("= Experience\n\n");
        for exp in &resume.experience {
            s.push_str(&format!("*{}* \\\n", e(&exp.title)));
            s.push_str(&format!("#text(size: 9.5pt, style: \"italic\")[{}]", e(&exp.company)));
            if let Some(ref dates) = exp.dates {
                s.push_str(&format!(" #h(1fr) {}", e(dates)));
            }
            s.push('\n');
            for bullet in &exp.bullets {
                s.push_str(&format!("- {}\n", e(bullet)));
            }
            s.push_str("\n");
        }
    }

    // ── Skills ──
    if !resume.skills.is_empty() {
        s.push_str("= Skills\n\n");
        s.push_str(&format!("{}\n\n", e(&resume.skills.join(", "))));
    }

    // ── Education ──
    if !resume.education.is_empty() {
        s.push_str("= Education\n\n");
        for edu in &resume.education {
            s.push_str(&format!("*{}* \\\n", e(&edu.degree)));
            s.push_str(&format!("#text(size: 9.5pt, style: \"italic\")[{}]", e(&edu.institution)));
            if let Some(ref year) = edu.year {
                s.push_str(&format!(" #h(1fr) {}", e(year)));
            }
            s.push_str("\n\n");
        }
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
        // App.app/Contents/MacOS/executable → Resources/bin/typst
        if let Some(parent) = exe.parent() {
            let p = parent.join("../Resources/bin/typst");
            if p.exists() {
                return Ok(p.canonicalize().map_err(|e| e.to_string())?);
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

    let mut has_personalized_resume = false;
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                if part.to_string_lossy() == "personalized-resume" {
                    has_personalized_resume = true;
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("Resume render path cannot contain parent, root, or prefix components".into());
            }
        }
    }

    if !has_personalized_resume {
        return Err("PDF rendering is only allowed for resume.md files under personalized-resume/ folders. The provided path does not contain personalized-resume/.".into());
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

pub const SKILL_RESUME_BUILDER_ALL: &str = concat!(
    "## /resume-builder-all — Tailor all selected job resumes\n",
    "\n",
    "Use this when the user types /resume-builder-all or asks to build/tailor all resumes for jobs in a hunt folder.\n",
    "\n",
    "### Required inputs\n",
    "- Ask the user for the target jobs folder when it is not explicit. It should look like `hunt_run/<hunt-slug>/jobs-YYYY-MM-DD/`.\n",
    "- Read `profile/RESUME.md` for the base resume structure and resume facts.\n",
    "- Read `profile/USER.md` for user preferences, background, tone, constraints, and positioning.\n",
    "- Read every `.md` job file inside the provided jobs folder. Skip non-Markdown files.\n",
    "\n",
    "### Output location\n",
    "For each job file, write exactly one personalized resume at:\n",
    "`hunt_run/<hunt-slug>/personalized-resume/<job-file-stem>/resume.md`\n",
    "where `<job-file-stem>` is the job file name without `.md` (for example `001-product-engineer-acme`).\n",
    "\n",
    "### Resume.md schema\n",
    "Preserve the same Markdown/frontmatter structure accepted by the app renderer:\n",
    "```\n",
    "---\n",
    "name: \"...\"\n",
    "email: \"...\"\n",
    "phone: \"...\"\n",
    "location: \"...\"\n",
    "linkedin: \"...\"\n",
    "summary: |\n",
    "  A concise tailored summary...\n",
    "---\n",
    "\n",
    "## Experience\n",
    "\n",
    "### Title | Company | Dates\n",
    "- Bullet point tailored to the job.\n",
    "\n",
    "## Skills\n",
    "Skill1, Skill2, Skill3\n",
    "\n",
    "## Education\n",
    "Degree | Institution | Year\n",
    "```\n",
    "\n",
    "### Tailoring rules\n",
    "- Do not invent employers, dates, degrees, certifications, or metrics not supported by profile files.\n",
    "- Reorder and emphasize existing facts to match each job's requirements.\n",
    "- Keep each resume concise and ATS-friendly.\n",
    "- Use the job description keywords naturally when they match true user experience.\n",
    "- Report the list of generated `resume.md` paths when finished.\n",
    "\n",
    "### Rendering PDFs\n",
    "- PDF rendering is allowed only for `personalized-resume/.../resume.md`.\n",
    "- If the user explicitly asks to render, use the `render_resume` tool with the generated resume file path.\n",
    "- Otherwise, generate Markdown first and tell the user they can open any `resume.md` and click Render PDF.\n",
);

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
    "5. Only render files under `personalized-resume/.../resume.md`. After fixing an eligible file, use the `render_resume` tool when the user asked you to render, or tell them to click Render PDF in the UI.\n",
    "\n",
    "### PDF generation\n",
    "PDF rendering is done by the bundled Typst binary (not pdflatex). The render command\n",
    "reads `resume.md`, generates a temporary Typst document, and compiles it to `resume.pdf`.\n",
    "Intermediate `.typ` files are NOT written to the workspace.\n",
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
    "2. Read the user's base resume and context:\n",
    "   - Read `profile/RESUME.md` for the user's complete experience, skills, and style.\n",
    "   - Read `profile/USER.md` for goals, constraints, and preferences.\n",
    "   - Optionally read `profile/RESUME_TEMPLATE.md` for structure reference.\n",
    "\n",
    "3. Tailor the resume:\n",
    "   - Reorder skills to match what the job asks for, in the order they ask.\n",
    "   - Rephrase bullet points using keywords from the job description where the user's real experience supports it.\n",
    "   - Emphasize relevant experience — move matching roles/projects higher.\n",
    "   - Match the positioning line / summary to the target role.\n",
    "   - Preserve the user's voice, wording style, and section structure from the base resume.\n",
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
);
