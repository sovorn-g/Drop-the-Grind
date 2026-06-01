use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

const APP_DIR: &str = ".dropthegrind";
const WORKSPACE_DIR: &str = "workspace";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub root_path: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub children: Option<Vec<FileTreeNode>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFile {
    pub content: String,
    pub version: String,
    pub read_only: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteInput {
    pub project_slug: String,
    pub path: String,
    pub content: String,
}

fn workspace_root() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    Ok(home.join(APP_DIR).join(WORKSPACE_DIR))
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string().chars().take(64).collect::<String>()
}

fn ensure_inside_workspace(path: &Path) -> Result<(), String> {
    let root = workspace_root()?;
    let root_canon = fs::canonicalize(&root).unwrap_or(root.clone());
    let candidate = if path.exists() {
        fs::canonicalize(path).map_err(|e| e.to_string())?
    } else {
        let parent = path.parent().ok_or("Path has no parent")?;
        let parent_canon = fs::canonicalize(parent).map_err(|e| e.to_string())?;
        parent_canon.join(path.file_name().ok_or("Path has no filename")?)
    };
    if candidate.starts_with(&root_canon) { Ok(()) } else { Err("Path escapes Drop the Grind workspace".into()) }
}

fn project_root(project_slug: &str) -> Result<PathBuf, String> {
    if project_slug.contains("..") || project_slug.contains('/') || project_slug.contains('\\') {
        return Err("Invalid project slug".into());
    }
    let path = workspace_root()?.join(project_slug);
    ensure_inside_workspace(&path)?;
    Ok(path)
}

fn safe_project_path(project_slug: &str, rel_path: &str) -> Result<PathBuf, String> {
    if rel_path.contains("..") || Path::new(rel_path).is_absolute() {
        return Err("Unsafe path".into());
    }
    let path = project_root(project_slug)?.join(rel_path);
    ensure_inside_workspace(&path)?;
    Ok(path)
}

fn is_text_editable(path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase().as_str(),
        "md" | "json" | "txt" | "tex" | "toml" | "yaml" | "yml")
}

fn write_if_missing(path: &Path, content: &str) -> Result<(), String> {
    if !path.exists() { fs::write(path, content).map_err(|e| e.to_string())?; }
    Ok(())
}

#[tauri::command]
pub fn create_project(name: String) -> Result<Project, String> {
    let slug = slugify(&name);
    if slug.is_empty() { return Err("Project name must include letters or numbers".into()); }
    let root = workspace_root()?.join(&slug);
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    ensure_inside_workspace(&root)?;

    for dir in ["profile", "sources/imports", "jobs/normalized", "jobs/ranked", "jobs/approved", "applications", "chats"] {
        fs::create_dir_all(root.join(dir)).map_err(|e| e.to_string())?;
    }
    let created_at = chrono::Utc::now().to_rfc3339();
    let project = Project { id: uuid::Uuid::new_v4().to_string(), name, slug: slug.clone(), root_path: root.to_string_lossy().to_string(), created_at: created_at.clone() };
    write_if_missing(&root.join("project.json"), &serde_json::to_string_pretty(&serde_json::json!({"id": project.id, "name": project.name, "slug": project.slug, "schemaVersion": 1, "createdAt": created_at, "folders": {"profile":"profile","sources":"sources","jobs":"jobs","applications":"applications","chats":"chats"}})).unwrap())?;
    write_if_missing(&root.join("profile/resume_extracted.md"), "# Resume Extracted\n\nPaste the markdown/plain-text version of your existing resume here.\n")?;
    write_if_missing(&root.join("profile/user_profile.md"), "# User Profile\n\nAdd stable facts about your background, constraints, and target roles.\n")?;
    write_if_missing(&root.join("profile/preferences.json"), "{\n  \"locations\": [],\n  \"remote\": true,\n  \"salaryMin\": null,\n  \"visa\": null,\n  \"seniority\": []\n}\n")?;
    write_if_missing(&root.join("profile/resume_original.pdf"), "")?;
    write_if_missing(&root.join("sources/apify_sources.json"), "[]\n")?;
    write_if_missing(&root.join("sources/apify_mcp_config.json"), "{\n  \"mcpServers\": {\n    \"apify\": {\n      \"url\": \"https://mcp.apify.com\"\n    }\n  }\n}\n")?;
    write_if_missing(&root.join("sources/run_apify_actor.md"), "# Run Apify Actor via MCP\n\nUse Apify MCP externally with Codex/opencode. Save output JSON into `sources/imports/`.\n")?;
    write_if_missing(&root.join("chats/project-chat.md"), "# Project Chat\n\nLocal notes and task drafts. Model execution is not connected yet.\n")?;
    Ok(project)
}

fn build_tree(root: &Path, current: &Path) -> Result<FileTreeNode, String> {
    let rel = current.strip_prefix(root).unwrap_or(current).to_string_lossy().to_string();
    let name = current.file_name().and_then(|s| s.to_str()).unwrap_or(".").to_string();
    if current.is_dir() {
        let mut children = vec![];
        for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            children.push(build_tree(root, &entry.path())?);
        }
        children.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.cmp(&b.name)));
        Ok(FileTreeNode { name, path: rel, kind: "directory".into(), children: Some(children) })
    } else {
        Ok(FileTreeNode { name, path: rel, kind: "file".into(), children: None })
    }
}

#[tauri::command]
pub fn list_workspace_tree(project_slug: String) -> Result<FileTreeNode, String> {
    let root = project_root(&project_slug)?;
    build_tree(&root, &root)
}

#[tauri::command]
pub fn read_text_file(project_slug: String, path: String) -> Result<TextFile, String> {
    let file = safe_project_path(&project_slug, &path)?;
    if !is_text_editable(&file) {
        return Ok(TextFile { content: "".into(), version: "binary".into(), read_only: true });
    }
    let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let modified = fs::metadata(&file).and_then(|m| m.modified()).ok();
    Ok(TextFile { content, version: format!("{:?}", modified), read_only: false })
}

#[tauri::command]
pub fn write_text_file(input: WriteInput) -> Result<(), String> {
    let file = safe_project_path(&input.project_slug, &input.path)?;
    if !is_text_editable(&file) { return Err("This file type is read-only in Drop the Grind".into()); }
    let tmp = file.with_extension(format!("{}.tmp", file.extension().and_then(|s| s.to_str()).unwrap_or("dtg")));
    fs::write(&tmp, input.content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &file).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![create_project, list_workspace_tree, read_text_file, write_text_file])
        .run(tauri::generate_context!())
        .expect("error while running Drop the Grind");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slugify_project_names() { assert_eq!(slugify("My 2026 Job Search!"), "my-2026-job-search"); }
    #[test]
    fn rejects_traversal_paths() { assert!(safe_project_path("demo", "../secrets.txt").is_err()); }
    #[test]
    fn rejects_invalid_project_slug() { assert!(project_root("../demo").is_err()); }
}
