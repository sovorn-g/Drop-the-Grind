use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    pub job_path: String, // relative path under hunt_run/<name>/jobs/<job-name>/
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOutput {
    pub tex_path: String,
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
        description: "Debug and fix resume PDF rendering issues (missing fields, format errors, LaTeX problems)",
        keyword_patterns: &["render", "pdf", "latex", "/fix-render", "fix the resume", "resume is broken", "rendering failed"],
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

const LATEX_SPECIAL_CHARS: &[char] = &['%', '$', '&', '_', '#', '{', '}', '~', '^'];

/// Validates a resume.md file and returns structured issues.
pub fn validate_resume_content(content: &str) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut line_num = 0;

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

    // Check for LaTeX special chars in frontmatter
    check_latex_safety(&fm_content, "frontmatter", &mut errors, &mut warnings);

    // Phase 2: Check body sections
    let body_start = if frontmatter_lines > 0 { frontmatter_range.1 + 1 } else { 0 };
    let body_lines: Vec<(usize, &str)> = lines.iter().enumerate().skip(body_start).map(|(i, l)| (i, *l)).collect();

    let mut has_experience = false;
    let mut has_skills = false;
    let mut has_education = false;
    let mut experience_item_count = 0;

    for (i, line) in &body_lines {
        line_num = i + 1;
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

        // Check LaTeX safety in all text
        check_latex_safety_line(trimmed, i + 1, &mut errors, &mut warnings);
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

fn check_latex_safety(content: &str, _field: &str, _errors: &mut Vec<ValidationIssue>, warnings: &mut Vec<ValidationIssue>) {
    for (i, line) in content.lines().enumerate() {
        check_latex_safety_line(line, i + 1, _errors, warnings);
    }
}

fn check_latex_safety_line(line: &str, line_num: usize, _errors: &mut Vec<ValidationIssue>, warnings: &mut Vec<ValidationIssue>) {
    let mut unsafe_chars: Vec<char> = Vec::new();
    for ch in LATEX_SPECIAL_CHARS {
        if line.contains(*ch) {
            unsafe_chars.push(*ch);
        }
    }
    if !unsafe_chars.is_empty() {
        let chars: String = unsafe_chars.iter().collect();
        warnings.push(ValidationIssue {
            field: "body".into(),
            message: format!("Unescaped LaTeX special character(s): '{}' — will be auto-escaped as \\{}", chars, chars),
            line: Some(line_num),
            severity: "warning".into(),
        });
    }
}

// ── LaTeX Escape ───────────────────────────────────────────────

/// Escapes LaTeX special characters in text
pub fn escape_latex(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '%' => result.push_str("\\%"),
            '$' => result.push_str("\\$"),
            '&' => result.push_str("\\&"),
            '_' => result.push_str("\\_"),
            '#' => result.push_str("\\#"),
            '{' => result.push_str("\\{"),
            '}' => result.push_str("\\}"),
            '~' => result.push_str("\\textasciitilde{}"),
            '^' => result.push_str("\\textasciicircum{}"),
            _ => result.push(ch),
        }
    }
    result
}

// ── Converter ──────────────────────────────────────────────────

/// Converts a Resume to LaTeX using a built-in template.
pub fn resume_to_latex(resume: &Resume) -> String {
    let mut latex = String::new();

    // Preamble
    latex.push_str(r#"\documentclass[11pt,a4paper]{article}
\usepackage[utf8]{inputenc}
\usepackage[T1]{fontenc}
\usepackage[margin=0.75in]{geometry}
\usepackage{hyperref}
\usepackage{xcolor}
\usepackage{titlesec}
\pagestyle{empty}

\titleformat{\section}{\large\bfseries\uppercase}{}{0em}{}[\vspace{-0.3em}\rule{\textwidth}{0.5pt}]
\titlespacing*{\section}{0pt}{1.2em}{0.5em}

\titleformat{\subsection}[runin]{\bfseries}{}{0em}{}
\titlespacing*{\subsection}{0pt}{0.5em}{0.3em}

\newcommand{\bulletitem}{\vspace{-0.2em}\item}

\begin{document}

"#);

    // Header
    let meta = &resume.meta;
    latex.push_str(&format!("\\begin{{center}}\n{{\\Huge \\textbf{{{}}}}}\\\\\n", escape_latex(&meta.name)));

    let mut header_items = Vec::new();
    if !meta.email.is_empty() { header_items.push(format!("\\href{{mailto:{}}}{{{}}}", escape_latex(&meta.email), escape_latex(&meta.email))); }
    if let Some(ref p) = meta.phone { header_items.push(escape_latex(p)); }
    if let Some(ref l) = meta.location { header_items.push(escape_latex(l)); }
    if let Some(ref li) = meta.linkedin { header_items.push(format!("\\href{{{}}}{{LinkedIn}}", escape_latex(li))); }

    if !header_items.is_empty() {
        latex.push_str(&header_items.join(" $\\cdot$ "));
        latex.push_str("\\\\\n");
    }

    latex.push_str("\\end{center}\n\n");

    // Summary
    if let Some(ref summary) = meta.summary {
        if !summary.is_empty() {
            latex.push_str(&format!("\n{}\n\n", escape_latex(summary)));
        }
    }

    // Experience
    if !resume.experience.is_empty() {
        latex.push_str("\\section*{Experience}\n\n");
        for exp in &resume.experience {
            latex.push_str(&format!(
                "\\subsection*{{{}}}\n\\textit{{{}}}",
                escape_latex(&exp.title),
                escape_latex(&exp.company)
            ));
            if let Some(ref dates) = exp.dates {
                latex.push_str(&format!(" \\hfill {}", escape_latex(dates)));
            }
            latex.push_str("\n\n\\begin{itemize}\n");
            for bullet in &exp.bullets {
                latex.push_str(&format!("  \\item {}\n", escape_latex(bullet)));
            }
            latex.push_str("\\end{itemize}\n\n");
        }
    }

    // Skills
    if !resume.skills.is_empty() {
        latex.push_str("\\section*{Skills}\n\n");
        latex.push_str(&format!("{}\n\n", escape_latex(&resume.skills.join(", "))));
    }

    // Education
    if !resume.education.is_empty() {
        latex.push_str("\\section*{Education}\n\n");
        for edu in &resume.education {
            latex.push_str(&format!(
                "\\noindent {}$\\hfill$\\textit{{{}}}",
                escape_latex(&edu.degree),
                escape_latex(&edu.institution)
            ));
            if let Some(ref year) = edu.year {
                latex.push_str(&format!(" \\hfill {}", escape_latex(year)));
            }
            latex.push_str("\n\n");
        }
    }

    latex.push_str("\\end{document}\n");
    latex
}

// ── Tauri commands ─────────────────────────────────────────────

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

/// Converts resume.md to .tex and returns the path.
#[tauri::command]
pub fn convert_resume(input: ResumeInput) -> Result<ConvertOutput, String> {
    let project = super::project_root(&input.project_slug)?;
    let job_dir = project.join(&input.job_path);
    let resume_path = job_dir.join("resume.md");
    let tex_path = job_dir.join("resume.tex");

    // Validate first
    let content = std::fs::read_to_string(&resume_path)
        .map_err(|e| format!("Failed to read resume.md: {}", e))?;

    let validation = validate_resume_content(&content);
    if !validation.valid {
        let error_msgs: Vec<String> = validation.errors.iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect();
        return Err(format!("Resume has validation errors:\n{}", error_msgs.join("\n")));
    }

    let resume = parse_resume(&content)?;
    let latex = resume_to_latex(&resume);

    std::fs::write(&tex_path, &latex)
        .map_err(|e| format!("Failed to write resume.tex: {}", e))?;

    Ok(ConvertOutput {
        tex_path: tex_path.to_string_lossy().to_string(),
    })
}

/// Renders resume.tex to resume.pdf via pdflatex.
#[tauri::command]
pub fn render_resume_pdf(input: ResumeInput) -> Result<RenderOutput, String> {
    let project = super::project_root(&input.project_slug)?;
    let job_dir = project.join(&input.job_path);
    let tex_path = job_dir.join("resume.tex");
    let pdf_path = job_dir.join("resume.pdf");

    // Ensure .tex exists — run convert if needed
    if !tex_path.exists() {
        // Try to convert first
        let convert_output = convert_resume(input.clone())?;
        if !tex_path.exists() {
            return Err(format!("resume.tex not found and conversion failed: {}", convert_output.tex_path));
        }
    }

    // Run pdflatex
    let output = ProcessCmd::new("pdflatex")
        .arg("-interaction=nonstopmode")
        .arg("-output-directory")
        .arg(&job_dir)
        .arg(&tex_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "pdflatex not found. Install TeX (e.g., 'brew install basictex' or 'mactex')".to_string()
            } else {
                format!("Failed to run pdflatex: {}", e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract a useful error snippet from the log
        let mut compile_errors = String::new();
        for line in stdout.lines() {
            if line.contains("! ") || line.contains("Error") || line.contains("error") {
                compile_errors.push_str(line);
                compile_errors.push('\n');
            }
        }
        if compile_errors.is_empty() {
            compile_errors = format!("pdflatex failed. Stderr: {}", stderr);
        }

        return Err(format!("LaTeX compilation failed:\n{}", compile_errors));
    }

    if !pdf_path.exists() {
        return Err("pdflatex completed but resume.pdf was not generated (check LaTeX template)".into());
    }

    Ok(RenderOutput {
        pdf_path: pdf_path.to_string_lossy().to_string(),
        compile_errors: None,
    })
}

/// Validates, converts, and renders — one-shot pipeline.
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

    // Step 2: Convert
    let _convert = convert_resume(input.clone())?;

    // Step 3: Render
    let render = render_resume_pdf(input)?;

    Ok(render)
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
    "Use this when the user mentions: render, PDF, fix the resume, latex, or /fix-render.\n",
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
    "- Missing `name:` or `email:` in frontmatter → read `profile/resume.md` for the user's info, fill it in.\n",
    "- Experience item missing company → format should be `### Title | Company | Dates`. If pipes are missing, add them.\n",
    "- Missing `## Experience` section → create one with at least one entry.\n",
    "- LaTeX special characters (`% $ & _ # { } ~ ^`) in text → these are auto-escaped during conversion (warnings only).\n",
    "- Missing `## Skills` section → recommended but not required, add from profile/resume.md if available.\n",
    "\n",
    "### Workflow\n",
    "1. Read `resume.md` from the job path using `read_file`.\n",
    "2. Check each section against the schema above.\n",
    "3. Fix issues using `write_file` — only edit `resume.md`, never `.tex` or `.pdf`.\n",
    "4. For missing user info (name, email), read `profile/resume.md` from the workspace root.\n",
    "5. After fixing, tell the user the .md is clean and ask them to click the Render button in the UI to generate the PDF.\n",
    "\n",
    "### If pdflatex is available\n",
    "If a `.tex` file already exists in the job directory, run:\n",
    "`run_command(\"cd <workspace_root> && pdflatex -interaction=nonstopmode -output-directory <job_dir> <job_dir>/resume.tex\")`\n",
    "Check for the `.tex` file first with `read_file`. If it doesn't exist, the user needs to click Render in the UI.\n",
);
