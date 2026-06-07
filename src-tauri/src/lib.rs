use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::{collections::{HashMap, hash_map::DefaultHasher}, fs, hash::{Hash, Hasher}, io::{BufRead, BufReader, Write}, path::{Path, PathBuf}, process::{Command, Stdio}, sync::{mpsc, Mutex}, thread, time::Duration};
use tauri::{ipc::Channel, AppHandle, Emitter, State};

mod resume;

const APP_DIR: &str = ".dropthegrind";
const WORKSPACE_DIR: &str = "workspace";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Project { pub id: String, pub name: String, pub slug: String, pub root_path: String, pub created_at: String }

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeNode { pub name: String, pub path: String, pub kind: String, pub children: Option<Vec<FileTreeNode>> }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFile { pub content: String, pub version: String, pub read_only: bool }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteInput { pub project_slug: String, pub path: String, pub content: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFileInput { pub project_slug: String, pub parent_path: String, pub name: String, pub content: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderInput { pub project_slug: String, pub parent_path: String, pub name: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePathInput { pub project_slug: String, pub path: String, pub new_name: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyPathToInput { pub project_slug: String, pub path: String, pub target_parent_path: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadFileInput { pub project_slug: String, pub parent_path: String, pub name: String, pub bytes: Vec<u8> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeUploadInput { pub project_slug: String, pub name: String, pub bytes: Vec<u8> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePathInput { pub project_slug: String, pub path: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInput { pub project_slug: String, pub name: String, pub actor_name: String, pub mcp_server_url: Option<String>, pub input_template_json: String }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TavilyExtractInput {
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TavilyExtractResult {
    pub url: String,
    pub title: Option<String>,
    pub raw_content: Option<String>,
    pub content: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TavilyExtractFailure {
    pub url: String,
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TavilyExtractOutput {
    pub results: Vec<TavilyExtractResult>,
    #[serde(default)]
    pub failed_results: Vec<TavilyExtractFailure>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportJobLinksInput {
    pub project_slug: String,
    pub name: String,
    pub urls: Vec<String>,
    #[serde(default)]
    pub run_id: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportLinkFile {
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    pub extracted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportLinkFailure {
    pub url: String,
    pub error: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportJobLinksSummary {
    pub submitted: usize,
    pub extracted: usize,
    pub written: usize,
    pub failed: usize,
    pub folder_path: String,
    pub files: Vec<ImportLinkFile>,
    pub failures: Vec<ImportLinkFailure>,
}



#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatusInput { pub project_slug: String, pub job_id: String, pub status: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketInput { pub project_slug: String, pub job_id: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveChatInput { pub project_slug: String, pub session_id: Option<String>, pub role: String, pub content: String, pub linked_file_path: Option<String>, pub linked_job_id: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkChatSessionInput { pub project_slug: String, pub source_session_id: String, pub up_to_message_id: String, pub title: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRespondInput { pub project_slug: String, pub prompt: String, pub linked_file_path: Option<String> }

#[derive(Default)]
pub struct AgentRunState { pub pids: Mutex<HashMap<String, u32>> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunInput { pub project_slug: String, pub prompt: String, pub linked_file_path: Option<String>, pub model: Option<String>, pub effort: Option<String>, pub run_id: Option<String> }

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunEvent { pub run_id: String, pub kind: String, pub text: String, pub payload: Option<serde_json::Value> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelAgentRunInput { pub run_id: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListChatInput { pub project_slug: String, pub session_id: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChatSessionInput { pub project_slug: String, pub title: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteChatSessionInput { pub project_slug: String, pub session_id: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatTaskInput { pub project_slug: String, pub content: String, pub file_name: Option<String> }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage { pub id: String, pub role: String, pub content: String, pub linked_file_path: Option<String>, pub linked_job_id: Option<String>, pub created_at: String }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession { pub id: String, pub project_slug: String, pub title: String, pub created_at: String, pub updated_at: String }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatus { pub installed: bool, pub connected: bool, pub auth_mode: Option<String>, pub version: Option<String>, pub detail: String }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApifyMcpStatus { pub connected: bool, pub detail: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApifyConnectInput { pub token: String }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TavilyStatus { pub connected: bool, pub detail: String, pub masked_key: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TavilyConnectInput { pub api_key: String }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirecrawlStatus { pub connected: bool, pub detail: String, pub masked_key: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirecrawlConnectInput { pub api_key: String }

// --- Firecrawl data structs for Import Links ---
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirecrawlMetadata {
    #[serde(default)]
    pub title: Option<serde_json::Value>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirecrawlScrapeResult {
    pub success: bool,
    pub data: Option<FirecrawlScrapeData>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirecrawlScrapeData {
    #[serde(default)]
    pub markdown: Option<String>,
    #[serde(default)]
    pub metadata: Option<FirecrawlMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirecrawlBatchStart {
    pub success: bool,
    pub id: Option<String>,
    #[serde(default)]
    pub invalid_urls: Option<Vec<String>>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirecrawlBatchStatus {
    pub status: Option<String>,
    pub total: Option<i64>,
    pub completed: Option<i64>,
    #[serde(default)]
    pub credits_used: Option<i64>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub data: Option<Vec<FirecrawlBatchPage>>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirecrawlBatchPage {
    #[serde(default)]
    pub markdown: Option<String>,
    #[serde(default)]
    pub metadata: Option<FirecrawlMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirecrawlBatchErrors {
    #[serde(default)]
    pub errors: Option<Vec<FirecrawlBatchErrorItem>>,
    #[serde(default)]
    pub robots_blocked: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirecrawlBatchErrorItem {
    pub id: Option<String>,
    pub url: Option<String>,
    pub error: Option<String>,
}

// Normalized internal type for extraction results
#[derive(Debug, Clone)]
struct ImportedExtractPage {
    pub original_url: String,
    pub extract_url: String,
    pub final_url: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HuntRunInput { pub project_slug: String, pub name: String, pub roles: Vec<String>, pub location: String, pub work_mode: String, #[serde(default)] pub seniority: String, #[serde(default)] pub experience: String, #[serde(default)] pub min_salary: String, pub include_keywords: String, #[serde(default)] pub exclude_keywords: String, pub posted_within: String, pub selected_sites: Vec<String>, pub max_items: usize }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunHuntApifyInput { #[serde(flatten)] pub hunt: HuntRunInput, pub results_path: String, pub run_id: String, #[serde(default)] pub is_re_run: bool }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntRunOutput { pub folder_path: String, pub results_path: String }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HuntConfig {
    pub name: String,
    pub slug: String,
    pub created: String,
    pub last_run: Option<String>,
    pub roles: Vec<String>,
    pub location: String,
    pub work_mode: String,
    pub seniority: String,
    pub experience: String,
    pub min_salary: String,
    pub include_keywords: String,
    pub exclude_keywords: String,
    pub posted_within: String,
    pub selected_sites: Vec<String>,
    pub max_items: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HuntRunEntry {
    pub date: String,
    pub new_jobs: usize,
    pub filtered: usize,
    pub duplicates: usize,
    pub sources_failed: Vec<String>,
    pub run_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HuntJobEntry {
    pub title: String,
    pub company: String,
    pub apply_url: String,
    pub source_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HuntResultDB {
    pub runs: Vec<HuntRunEntry>,
    pub jobs: HashMap<String, HuntJobEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntProfileSummary {
    pub name: String,
    pub slug: String,
    pub job_count: usize,
    pub run_count: usize,
    pub last_run: Option<String>,
    pub created: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveHuntConfigInput {
    pub project_slug: String,
    pub hunt_slug: String,
    pub config: HuntConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig { pub id: String, pub project_slug: String, pub name: String, pub actor_name: String, pub mcp_server_url: String, pub input_template_json: String, pub updated_at: String }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord { pub id: String, pub title: String, pub company: String, pub location: Option<String>, pub remote_type: String, pub description: Option<String>, pub salary_range: Option<String>, pub apply_url: String, pub source_url: String, pub source_type: String, pub dedupe_key: String, pub status: String, pub created_at: String }



#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationPacket { pub id: String, pub job_id: String, pub relative_path: String, pub status: String, pub created_at: String }

fn app_root() -> Result<PathBuf, String> { Ok(dirs::home_dir().ok_or("Could not determine home directory")?.join(APP_DIR)) }
fn workspace_root() -> Result<PathBuf, String> { Ok(app_root()?.join(WORKSPACE_DIR)) }
fn db_path() -> Result<PathBuf, String> { Ok(app_root()?.join("drop-the-grind.sqlite")) }
fn settings_path() -> Result<PathBuf, String> { Ok(app_root()?.join("settings.json")) }

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Db { projects: Vec<Project>, source_configs: Vec<SourceConfig>, imports: Vec<Value>, jobs: Vec<StoredJob>, packets: Vec<ApplicationPacket> }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredJob { project_slug: String, #[serde(flatten)] job: JobRecord }

fn conn() -> Result<Connection, String> {
    fs::create_dir_all(app_root()?).map_err(|e| e.to_string())?;
    let c = Connection::open(db_path()?).map_err(|e| e.to_string())?;
    c.execute_batch("PRAGMA foreign_keys=ON;
      CREATE TABLE IF NOT EXISTS projects(id TEXT PRIMARY KEY, name TEXT NOT NULL, slug TEXT UNIQUE NOT NULL, root_path TEXT NOT NULL, schema_version INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS source_configs(id TEXT PRIMARY KEY, project_slug TEXT NOT NULL, type TEXT NOT NULL, name TEXT NOT NULL, actor_name TEXT NOT NULL, mcp_server_url TEXT NOT NULL, input_template_json TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS imports(id TEXT PRIMARY KEY, project_slug TEXT NOT NULL, raw_file_path TEXT NOT NULL, item_count INTEGER NOT NULL, status TEXT NOT NULL, error_message TEXT, created_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS jobs(id TEXT PRIMARY KEY, project_slug TEXT NOT NULL, dedupe_key TEXT NOT NULL, job_json TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE(project_slug, dedupe_key));
      CREATE TABLE IF NOT EXISTS application_packets(id TEXT PRIMARY KEY, project_slug TEXT NOT NULL, job_id TEXT NOT NULL, packet_json TEXT NOT NULL, relative_path TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE(project_slug, job_id));
      CREATE TABLE IF NOT EXISTS chat_sessions(id TEXT PRIMARY KEY, project_slug TEXT NOT NULL, title TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS chat_messages(id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL, linked_file_path TEXT, linked_job_id TEXT, created_at TEXT NOT NULL);").map_err(|e| e.to_string())?;
    Ok(c)
}
fn load_db() -> Result<Db, String> {
    let c = conn()?;
    let mut ps = c.prepare("SELECT id,name,slug,root_path,created_at FROM projects ORDER BY created_at").map_err(|e| e.to_string())?;
    let projects = ps.query_map([], |r| Ok(Project{id:r.get(0)?,name:r.get(1)?,slug:r.get(2)?,root_path:r.get(3)?,created_at:r.get(4)?})).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    let mut ss = c.prepare("SELECT id,project_slug,name,actor_name,mcp_server_url,input_template_json,updated_at FROM source_configs ORDER BY updated_at DESC").map_err(|e| e.to_string())?;
    let source_configs = ss.query_map([], |r| Ok(SourceConfig{id:r.get(0)?,project_slug:r.get(1)?,name:r.get(2)?,actor_name:r.get(3)?,mcp_server_url:r.get(4)?,input_template_json:r.get(5)?,updated_at:r.get(6)?})).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    let mut js = c.prepare("SELECT project_slug,job_json FROM jobs ORDER BY created_at DESC").map_err(|e| e.to_string())?;
    let jobs = js.query_map([], |r| { let project_slug:String=r.get(0)?; let s:String=r.get(1)?; let job:JobRecord=serde_json::from_str(&s).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?; Ok(StoredJob{project_slug,job}) }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    let mut pts = c.prepare("SELECT packet_json FROM application_packets ORDER BY created_at DESC").map_err(|e| e.to_string())?;
    let packets = pts.query_map([], |r| { let s:String=r.get(0)?; serde_json::from_str::<ApplicationPacket>(&s).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e))) }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    Ok(Db{projects,source_configs,imports:vec![],jobs,packets})
}
fn save_db(db: &Db) -> Result<(), String> {
    let mut c = conn()?; let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM projects",[]).map_err(|e|e.to_string())?; tx.execute("DELETE FROM source_configs",[]).map_err(|e|e.to_string())?; tx.execute("DELETE FROM jobs",[]).map_err(|e|e.to_string())?; tx.execute("DELETE FROM application_packets",[]).map_err(|e|e.to_string())?;
    for p in &db.projects { tx.execute("INSERT INTO projects(id,name,slug,root_path,schema_version,created_at,updated_at) VALUES(?1,?2,?3,?4,2,?5,?5)", params![p.id,p.name,p.slug,p.root_path,p.created_at]).map_err(|e|e.to_string())?; }
    for s in &db.source_configs { tx.execute("INSERT INTO source_configs(id,project_slug,type,name,actor_name,mcp_server_url,input_template_json,created_at,updated_at) VALUES(?1,?2,'apify',?3,?4,?5,?6,?7,?7)", params![s.id,s.project_slug,s.name,s.actor_name,s.mcp_server_url,s.input_template_json,s.updated_at]).map_err(|e|e.to_string())?; }
    for j in &db.jobs { tx.execute("INSERT OR IGNORE INTO jobs(id,project_slug,dedupe_key,job_json,status,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?6)", params![j.job.id,j.project_slug,j.job.dedupe_key,serde_json::to_string(&j.job).unwrap(),j.job.status,j.job.created_at]).map_err(|e|e.to_string())?; }
    for p in &db.packets { tx.execute("INSERT OR IGNORE INTO application_packets(id,project_slug,job_id,packet_json,relative_path,status,created_at,updated_at) VALUES(?1,'default',?2,?3,?4,?5,?6,?6)", params![p.id,p.job_id,serde_json::to_string(p).unwrap(),p.relative_path,p.status,p.created_at]).map_err(|e|e.to_string())?; }
    tx.commit().map_err(|e|e.to_string())
}
fn now() -> String { chrono::Utc::now().to_rfc3339() }

fn slugify(input: &str) -> String {
    let mut out = String::new(); let mut last_dash = false;
    for ch in input.to_lowercase().chars() { if ch.is_ascii_alphanumeric() { out.push(ch); last_dash=false; } else if !last_dash { out.push('-'); last_dash=true; } }
    out.trim_matches('-').to_string().chars().take(64).collect()
}
fn short_hash(input: &str) -> String { let mut h=DefaultHasher::new(); input.hash(&mut h); format!("{:x}", h.finish()).chars().take(8).collect() }
fn ensure_inside_workspace(path: &Path) -> Result<(), String> {
    fs::create_dir_all(workspace_root()?).map_err(|e| e.to_string())?;
    let root = workspace_root()?; let root_canon = fs::canonicalize(&root).unwrap_or(root.clone());
    let candidate = if path.exists() { fs::canonicalize(path).map_err(|e| e.to_string())? } else { let parent = path.parent().ok_or("Path has no parent")?; fs::canonicalize(parent).map_err(|e| e.to_string())?.join(path.file_name().ok_or("Path has no filename")?) };
    if candidate.starts_with(&root_canon) { Ok(()) } else { Err("Path escapes Drop the Grind workspace".into()) }
}
pub(crate) fn project_root(project_slug: &str) -> Result<PathBuf, String> { if project_slug.contains("..") || project_slug.contains('/') || project_slug.contains('\\') { return Err("Invalid project slug".into()); } let path=workspace_root()?.join(project_slug); ensure_inside_workspace(&path)?; Ok(path) }
fn safe_project_path(project_slug: &str, rel_path: &str) -> Result<PathBuf, String> { if rel_path.contains("..") || Path::new(rel_path).is_absolute() { return Err("Unsafe path".into()); } let path=project_root(project_slug)?.join(rel_path); ensure_inside_workspace(&path)?; Ok(path) }
fn is_text_editable(path: &Path) -> bool { matches!(path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase().as_str(), "md"|"json"|"txt"|"tex"|"toml"|"yaml"|"yml") }
fn write_if_missing(path: &Path, content: &str) -> Result<(), String> { if !path.exists() { fs::write(path, content).map_err(|e| e.to_string())?; } Ok(()) }

#[tauri::command]
fn create_project(name: String) -> Result<Project, String> {
    let slug=slugify(&name); if slug.is_empty(){return Err("Project name must include letters or numbers".into())}
    let root=workspace_root()?.join(&slug); fs::create_dir_all(&root).map_err(|e| e.to_string())?; ensure_inside_workspace(&root)?;
    let created_at=now(); let project=Project{id:uuid::Uuid::new_v4().to_string(),name,slug:slug.clone(),root_path:root.to_string_lossy().to_string(),created_at:created_at.clone()};
    write_if_missing(&root.join("project.json"), &serde_json::to_string_pretty(&serde_json::json!({"id":project.id,"name":project.name,"slug":project.slug,"schemaVersion":3,"createdAt":created_at,"workspace":"minimal","generatedFolders":"created on demand"})).unwrap())?;
    let profile_root=root.join("profile"); fs::create_dir_all(&profile_root).map_err(|e| e.to_string())?;
    write_if_missing(&root.join("INSTRUCTION.md"), &[
        "# Project Workspace Guide","","This project contains the following files:","","| File | Purpose |","|---|---|","| `profile/RESUME.md` | Your working resume. Edit this to reflect your real experience and skills. |","| `profile/RESUME_TEMPLATE.md` | A structured template with placeholder sections to guide resume formatting. |","| `profile/USER.md` | Personal context — your goals, constraints, preferences, and notes for the agent. |","","You can edit these Markdown files directly in the Editor. The agent reads them to tailor resumes, cover letters, and outreach messages.",""].join("\n"))?;
    write_if_missing(&profile_root.join("RESUME_TEMPLATE.md"), &[
        "# [FULL NAME]","","**[Target Role / Positioning Line]***","[LinkedIn] | [Portfolio] | [GitHub] | [Email] | [Phone] | [Location]","","---","","## Summary","","[2-3 lines. State candidate identity, years or level, strongest technical focus, and 1-2 concrete achievements. Tailor this to the target job description.]","","---","","## Skills","","**Languages:** [Python], [JavaScript], [C++], [SQL], [Java]","**ML / Data:** [PyTorch], [TensorFlow], [scikit-learn], [Pandas], [NumPy]","**Cloud / Infrastructure:** [AWS], [Docker], [Kubernetes], [Linux], [CI/CD]","**Tools / Frameworks:** [Git], [FastAPI], [Node.js], [Spark], [PostgreSQL]","","---","","## Experience","","### [Company / Lab / Institution]","**[Role Title] - [Team or Focus Area]***","[Location] | [Start Date] - [End Date]","","- [Strongest job-relevant achievement with action, method, scale, and result.]","- [Second achievement with metric, technical depth, or production impact.]","- [Third achievement showing collaboration, ownership, or business/research value.]","","### [Company / Lab / Institution]","**[Role Title] - [Team or Focus Area]***","[Location] | [Start Date] - [End Date]","","- [Achievement bullet.]","- [Achievement bullet.]","- [Achievement bullet.]","","---","","## Projects / Research","","### [Project or Research Title]","**[Project Type / Context]***","[Link if available] | [Start Date] - [End Date]","","- [What was built, researched, or shipped.]","- [Technical methods, architecture, model, dataset, or system design.]","- [Result, metric, demo, paper, users, deployment, or measurable impact.]","","### [Project or Research Title]","**[Project Type / Context]***","[Link if available] | [Start Date] - [End Date]","","- [Achievement bullet.]","- [Achievement bullet.]","","---","","## Education","","### [University Name]","**[Degree Name]***","[Location] | [Start Date] - [End Date]","","- [Relevant focus, thesis, concentration, GPA if strong, or notable coursework.]","","---","","## Optional Sections","","Use only sections that add value for the target role.","","### Selected Publications","","- [Author list]. \"[Paper Title].\" *[Venue]*, [Year]. [Status if not published.]","","### Awards","","- **[Award Name]**, [Granting Organization], [Year]. [Brief context.]","","### Teaching / Mentorship","","- [Teaching assistant, mentor, workshop, student supervision, or training impact.]","","### Certifications","","- [Certification Name], [Issuer], [Year].","","### Work Authorization","","- [Authorized to work in X / Sponsorship status if relevant and desired.]",""].join("\n"))?;
    write_if_missing(&profile_root.join("RESUME.md"), &[
        "# Mitchell Bucklew","","**Software Engineer | AI Researcher | 4x Intern, 3x Teaching Assistant**","https://www.linkedin.com/in/mitchell-bucklew | https://mitchellbucklew.dev | https://github.com/mitchellbucklew | mitchell@example.com | Phoenix, AZ","","---","","## Summary","","Software engineer and AI researcher with internship experience across machine learning platforms, data services, and production software teams. Built scalable ML and data-processing systems using Python, Spark, AWS, and cloud deployment workflows, with teaching experience supporting 500+ students in data structures and AI coursework.","","---","","## Skills","","**Languages:** Python, JavaScript, C, C++, Java, C#, SQL, PHP, HTML, CSS","**ML / Data:** Pandas, NumPy, scikit-learn, TensorFlow, PyTorch, Spark, neural networks","**Cloud / Infrastructure:** AWS, EC2, EMR, ECS, Athena, Lambda, Docker, Linux","**Tools / Frameworks:** Node.js, Git, Bash, Jupyter, Elasticsearch, data lakes","","---","","## Experience","","### Intuit","**Machine Learning Intern - Personalization Team**","San Francisco, CA | May 2022 - Aug 2022","","- Built resources that helped data scientists move machine learning projects from development into production workflows.","- Implemented large-scale ETL data processing pipelines using Spark for distributed computing workloads.","- Developed production-grade services and pipelines to make machine learning models available at web scale.","- Presented completed project outcomes to management and technical team members.","","### American Express","**Software Engineering Intern - Data Services Team**","Phoenix, AZ | May 2021 - Aug 2021","","- Researched, designed, and implemented a machine learning application for internal IT support workflows.","- Reduced technical staff support time by an average of 30 minutes per ticket during testing.","- Created a Slack bot to improve developer communication and increase team productivity.","- Deployed a Hugging Face pre-trained model for NLP processing and presented project metrics to executives.","","### Arizona State University","**Teaching Assistant**","Tempe, AZ | Aug 2020 - Dec 2022","","- Taught data structures and AI course material to 500+ students across undergraduate and graduate-level courses.","- Explained challenging computer science concepts through live sessions, office hours, and student support.","- Invited to continue teaching based on performance and department needs.","","---","","## Projects / Research","","### Artificial Intelligence Research","**Master's Thesis**","Tempe, AZ | Aug 2021 - Aug 2022","","- Collaborated on AI research intended for publication in a scientific journal.","- Conducted research at Arizona State University's Cooperative Robotic Systems Lab.","- Created and deployed scalable machine learning models to cloud infrastructure.","","### NASA Rocket Analysis","**Capstone Project**","Tempe, AZ | Aug 2020 - May 2021","","- Collaborated with NASA engineers to convert business requirements into technical specifications.","- Led a four-person development team using agile methodology.","- Delivered data analysis software and received positive sponsor feedback during performance review.","","---","","## Education","","### Arizona State University","**Bachelor of Computer Science & Master of Computer Science**","Tempe, AZ | 2017 - Sep 2022","","- Focus: Artificial Intelligence","","---","","## Awards","","- **Outstanding Teaching Assistant Recognition**, Arizona State University, 2022. Recognized for student support in data structures and AI coursework.","","---","","## Certifications","","- **AWS Certified Cloud Practitioner**, Amazon Web Services, 2022.",""].join("\n"))?;
    write_if_missing(&profile_root.join("USER.md"), &[
        "# User Profile","","This profile helps the agent understand your personal context beyond your resume. Fill in what applies, and feel free to add any other sections that matter to you.","","---","","## Career Goals","","[What kind of role, company, impact, or career direction are you targeting?]","","---","","## Constraints","","[Location restrictions, visa needs, salary minimums, remote requirements, timeline, etc.]","","---","","## Preferences","","[Tech stack, team size, industry, company stage, culture, work style, etc.]","","---","","## Notes for the Agent","","[Anything else you want the agent to know when tailoring resumes, writing outreach, or helping with your job search.]","","---","","*You can add any additional sections below — this template is just a starting point.*",""].join("\n"))?;
    Ok(project)
}

fn build_tree(root:&Path,current:&Path)->Result<FileTreeNode,String>{ let rel=current.strip_prefix(root).unwrap_or(current).to_string_lossy().to_string(); let name=current.file_name().and_then(|s|s.to_str()).unwrap_or(".").to_string(); if current.is_dir(){ let mut children=vec![]; for entry in fs::read_dir(current).map_err(|e|e.to_string())?{ let entry=entry.map_err(|e|e.to_string())?; if entry.file_name().to_str().map(|s| s.starts_with('.')).unwrap_or(false) { continue; } children.push(build_tree(root,&entry.path())?); } children.sort_by(|a,b|a.kind.cmp(&b.kind).then(a.name.cmp(&b.name))); Ok(FileTreeNode{name,path:rel,kind:"directory".into(),children:Some(children)}) } else { Ok(FileTreeNode{name,path:rel,kind:"file".into(),children:None}) } }
#[tauri::command]
fn list_projects() -> Result<Vec<Project>, String> {
    let mut projects = load_db()?.projects;
    if projects.is_empty() {
        let root = workspace_root()?;
        if root.exists() {
            for entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                let manifest = path.join("project.json");
                if manifest.exists() {
                    if let Ok(v) = serde_json::from_str::<Value>(&fs::read_to_string(&manifest).map_err(|e| e.to_string())?) {
                        let slug = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                        projects.push(Project {
                            id: string_alias(&v, &["id"]).unwrap_or_else(|| slug.clone()),
                            name: string_alias(&v, &["name"]).unwrap_or_else(|| slug.clone()),
                            slug,
                            root_path: path.to_string_lossy().to_string(),
                            created_at: string_alias(&v, &["createdAt", "created_at"]).unwrap_or_else(now),
                        });
                    }
                }
            }
        }
    }
    projects.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(projects)
}

#[tauri::command]
fn delete_project(project_slug: String) -> Result<(), String> {
    if project_slug.trim().is_empty() { return Err("Project slug is required".into()); }
    let root = project_root(&project_slug)?;
    let workspace = workspace_root()?.canonicalize().map_err(|e| e.to_string())?;
    if root.exists() {
        let root_canon = root.canonicalize().map_err(|e| e.to_string())?;
        if root_canon == workspace { return Err("Workspace root cannot be deleted".into()); }
        if !root_canon.starts_with(&workspace) { return Err("Project path escapes workspace".into()); }
        fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    let mut db = load_db()?;
    db.projects.retain(|p| p.slug != project_slug);
    db.source_configs.retain(|s| s.project_slug != project_slug);
    db.jobs.retain(|j| j.project_slug != project_slug);
    save_db(&db)?;
    let c = conn()?;
    c.execute("DELETE FROM chat_messages WHERE session_id IN (SELECT id FROM chat_sessions WHERE project_slug=?1)", params![project_slug]).map_err(|e| e.to_string())?;
    c.execute("DELETE FROM chat_sessions WHERE project_slug=?1", params![project_slug]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_project(project_slug: String) -> Result<Project, String> {
    let root = project_root(&project_slug)?;
    let db = load_db()?;
    if let Some(p) = db.projects.into_iter().find(|p| p.slug == project_slug) { return Ok(p); }
    let manifest = root.join("project.json");
    if manifest.exists() {
        let v: Value = serde_json::from_str(&fs::read_to_string(manifest).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        return Ok(Project { id: string_alias(&v, &["id"]).unwrap_or_else(|| project_slug.clone()), name: string_alias(&v, &["name"]).unwrap_or_else(|| project_slug.clone()), slug: project_slug, root_path: root.to_string_lossy().to_string(), created_at: string_alias(&v, &["createdAt", "created_at"]).unwrap_or_else(now) });
    }
    Err("Project not found".into())
}

#[tauri::command] fn list_workspace_tree(project_slug:String)->Result<FileTreeNode,String>{ let root=project_root(&project_slug)?; build_tree(&root,&root) }
#[tauri::command] fn read_text_file(project_slug:String,path:String)->Result<TextFile,String>{ let file=safe_project_path(&project_slug,&path)?; if !is_text_editable(&file){ return Ok(TextFile{content:"".into(),version:"binary".into(),read_only:true}); } let content=fs::read_to_string(&file).map_err(|e|e.to_string())?; let modified=fs::metadata(&file).and_then(|m|m.modified()).ok(); Ok(TextFile{content,version:format!("{:?}",modified),read_only:false}) }
#[tauri::command] fn read_binary_file(project_slug:String,path:String)->Result<Vec<u8>,String>{ let file=safe_project_path(&project_slug,&path)?; fs::read(&file).map_err(|e|e.to_string()) }
#[tauri::command] fn write_text_file(input:WriteInput)->Result<(),String>{ let file=safe_project_path(&input.project_slug,&input.path)?; if !is_text_editable(&file){return Err("This file type is read-only in Drop the Grind".into())} let tmp=file.with_extension(format!("{}.tmp",file.extension().and_then(|s|s.to_str()).unwrap_or("dtg"))); fs::write(&tmp,input.content).map_err(|e|e.to_string())?; fs::rename(&tmp,&file).map_err(|e|e.to_string())?; Ok(()) }

fn validate_child_name(name: &str, kind: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." || trimmed.contains('/') || trimmed.contains('\\') { return Err(format!("Invalid {kind} name")); }
    Ok(trimmed.to_string())
}

fn child_rel(parent_path: &str, name: &str) -> String {
    if parent_path.trim().is_empty() { name.to_string() } else { format!("{}/{}", parent_path.trim_end_matches('/'), name) }
}

#[tauri::command]
fn create_text_file(input: CreateFileInput) -> Result<String, String> {
    let name = validate_child_name(&input.name, "file")?;
    let rel = child_rel(&input.parent_path, &name);
    let path = safe_project_path(&input.project_slug, &rel)?;
    if path.exists() { return Err("File already exists".into()); }
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    fs::write(&path, input.content.unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(rel)
}

#[tauri::command]
fn create_project_folder(input: CreateFolderInput) -> Result<String, String> {
    let name = validate_child_name(&input.name, "folder")?;
    let rel = child_rel(&input.parent_path, &name);
    let path = safe_project_path(&input.project_slug, &rel)?;
    if path.exists() { return Err("Folder already exists".into()); }
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(rel)
}

#[tauri::command]
fn rename_project_path(input: RenamePathInput) -> Result<String, String> {
    if input.path.trim().is_empty() { return Err("Project root cannot be renamed".into()); }
    let new_name = validate_child_name(&input.new_name, "name")?;
    let path = safe_project_path(&input.project_slug, &input.path)?;
    if !path.exists() { return Err("Path does not exist".into()); }
    let parent = path.parent().ok_or("Could not resolve parent folder")?;
    let target = parent.join(&new_name);
    if target.exists() { return Err("A file or folder with that name already exists".into()); }
    let project_root_path = project_root(&input.project_slug)?.canonicalize().map_err(|e| e.to_string())?;
    let parent_canon = parent.canonicalize().map_err(|e| e.to_string())?;
    if !parent_canon.starts_with(&project_root_path) { return Err("Path escapes project workspace".into()); }
    fs::rename(&path, &target).map_err(|e| e.to_string())?;
    // If this is a hunt folder, sync the slug in .hunt_config.json to the new folder name
    if target.is_dir() {
        let config_path = target.join(".hunt_config.json");
        if config_path.exists() {
            if let Ok(json) = fs::read_to_string(&config_path) {
                if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&json) {
                    config["slug"] = serde_json::Value::String(new_name.clone());
                    if let Ok(new_json) = serde_json::to_string_pretty(&config) {
                        let _ = fs::write(&config_path, new_json);
                    }
                }
            }
        }
    }
    let rel_parent = Path::new(&input.path).parent().and_then(|p| p.to_str()).unwrap_or("");
    Ok(child_rel(rel_parent, &new_name))
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() { copy_dir_recursive(&src_path, &dst_path)?; } else { fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

fn copy_name_candidate(name: &str, attempt: usize) -> String {
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let suffix = if attempt == 1 { " copy".to_string() } else { format!(" copy {attempt}") };
    if ext.is_empty() { format!("{stem}{suffix}") } else { format!("{stem}{suffix}.{ext}") }
}

fn copy_project_path_into(project_slug: &str, src_rel: &str, target_parent_rel: &str) -> Result<String, String> {
    if src_rel.trim().is_empty() { return Err("Project root cannot be copied".into()); }
    let src = safe_project_path(project_slug, src_rel)?;
    if !src.exists() { return Err("Path does not exist".into()); }
    let target_parent = safe_project_path(project_slug, target_parent_rel)?;
    if !target_parent.exists() || !target_parent.is_dir() { return Err("Paste destination must be a folder".into()); }
    let src_canon = src.canonicalize().map_err(|e| e.to_string())?;
    let target_parent_canon = target_parent.canonicalize().map_err(|e| e.to_string())?;
    if src.is_dir() && target_parent_canon.starts_with(&src_canon) { return Err("A folder cannot be pasted into itself or one of its subfolders".into()); }
    let name = src.file_name().and_then(|s| s.to_str()).ok_or("Invalid file name")?;
    let mut target = target_parent.join(name);
    let mut rel_name = name.to_string();
    if target.exists() {
        rel_name = copy_name_candidate(name, 1);
        target = target_parent.join(&rel_name);
        for attempt in 2..100 {
            if !target.exists() { break; }
            rel_name = copy_name_candidate(name, attempt);
            target = target_parent.join(&rel_name);
        }
    }
    if target.exists() { return Err("Could not find an available copy name".into()); }
    if src.is_dir() { copy_dir_recursive(&src, &target)?; } else { fs::copy(&src, &target).map_err(|e| e.to_string())?; }
    Ok(child_rel(target_parent_rel, &rel_name))
}

#[tauri::command]
fn copy_project_path(input: FilePathInput) -> Result<String, String> {
    let parent = Path::new(&input.path).parent().and_then(|p| p.to_str()).unwrap_or("");
    copy_project_path_into(&input.project_slug, &input.path, parent)
}

#[tauri::command]
fn copy_project_path_to(input: CopyPathToInput) -> Result<String, String> {
    copy_project_path_into(&input.project_slug, &input.path, &input.target_parent_path)
}

#[tauri::command]
fn upload_project_file(input: UploadFileInput) -> Result<String, String> {
    let name = validate_child_name(&input.name, "file")?;
    let rel = child_rel(&input.parent_path, &name);
    let path = safe_project_path(&input.project_slug, &rel)?;
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    fs::write(&path, input.bytes).map_err(|e| e.to_string())?;
    Ok(rel)
}

#[tauri::command]
fn upload_resume(input: ResumeUploadInput) -> Result<String, String> {
    let ext = Path::new(&input.name).extension().and_then(|s| s.to_str()).unwrap_or("pdf");
    let profile_dir = project_root(&input.project_slug)?.join("profile");
    fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;
    let rel = format!("profile/resume_current.{}", slugify(ext));
    let path = safe_project_path(&input.project_slug, &rel)?;
    fs::write(&path, &input.bytes).map_err(|e| e.to_string())?;
    if ["md","txt"].contains(&ext.to_lowercase().as_str()) {
        if let Ok(text) = String::from_utf8(input.bytes) { fs::write(profile_dir.join("resume_extracted.md"), text).map_err(|e| e.to_string())?; }
    }
    Ok(rel)
}

#[tauri::command]
fn remove_resume(project_slug: String) -> Result<(), String> {
    let root = project_root(&project_slug)?.join("profile");
    if !root.exists() { return Ok(()); }
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? { let path = entry.map_err(|e| e.to_string())?.path(); if path.file_name().and_then(|s|s.to_str()).unwrap_or("").starts_with("resume_current") { let _ = fs::remove_file(path); } }
    Ok(())
}

#[tauri::command]
fn delete_project_file(input: FilePathInput) -> Result<(), String> {
    delete_project_path(input)
}

#[tauri::command]
fn delete_project_path(input: FilePathInput) -> Result<(), String> {
    if input.path.trim().is_empty() { return Err("Project root cannot be deleted".into()); }
    let path = safe_project_path(&input.project_slug, &input.path)?;
    if !path.exists() { return Err("Path does not exist".into()); }
    if path.is_dir() { fs::remove_dir_all(path).map_err(|e| e.to_string()) } else { fs::remove_file(path).map_err(|e| e.to_string()) }
}

#[tauri::command]
fn reveal_project_path(input: FilePathInput) -> Result<(), String> {
    let path = safe_project_path(&input.project_slug, &input.path)?;
    Command::new("open").arg("-R").arg(path).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_project_file(input: FilePathInput) -> Result<(), String> {
    let path = safe_project_path(&input.project_slug, &input.path)?;
    Command::new("open").arg(path).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn save_source_config(input: SourceInput) -> Result<SourceConfig, String> {
    if input.actor_name.trim().is_empty() { return Err("Actor name is required".into()); }
    serde_json::from_str::<Value>(&input.input_template_json).map_err(|e| format!("Input JSON is invalid: {e}"))?;
    let url=input.mcp_server_url.unwrap_or_else(||"https://mcp.apify.com".into()); let updated_at=now();
    let cfg=SourceConfig{id:short_hash(&format!("{}:{}",input.project_slug,input.actor_name)),project_slug:input.project_slug.clone(),name:input.name,actor_name:input.actor_name,mcp_server_url:url,input_template_json:input.input_template_json,updated_at};
    let mut db=load_db()?; db.source_configs.retain(|c| !(c.project_slug==cfg.project_slug && c.id==cfg.id)); db.source_configs.push(cfg.clone()); save_db(&db)?; Ok(cfg)
}
#[tauri::command]
fn get_source_config(project_slug:String)->Result<Option<SourceConfig>,String>{ Ok(load_db()?.source_configs.into_iter().find(|c| c.project_slug==project_slug)) }
#[tauri::command]
fn generate_apify_files(input: SourceInput)->Result<(),String>{ let cfg=save_source_config(input)?; let root=project_root(&cfg.project_slug)?; fs::write(root.join("sources/apify_mcp_config.json"), serde_json::to_string_pretty(&serde_json::json!({"mcpServers":{"apify":{"url":cfg.mcp_server_url}}})).unwrap()).map_err(|e|e.to_string())?; fs::write(root.join("sources/apify_actor_input.json"), cfg.input_template_json).map_err(|e|e.to_string())?; fs::write(root.join("sources/run_apify_actor.md"), format!("# Run Apify Actor via MCP\n\nUse the Apify MCP server to run actor `{}` with the input in `sources/apify_actor_input.json`.\n\nSave the returned dataset items as JSON to `sources/imports/<timestamp>-raw.json`.\n\nDo not tailor resumes or apply to jobs. Only collect job listing data.\n", cfg.actor_name)).map_err(|e|e.to_string())?; Ok(()) }

fn actor_slug_for_site(site: &str) -> &'static str {
    match site {
        "54 Career Sites" => "fantastic-jobs/career-site-job-listing-api",
        "Indeed" => "borderline/indeed-scraper",
        "LinkedIn" => "fantastic-jobs/advanced-linkedin-job-search-api",
        "YC Startup Jobs" => "memo23/y-combinator-scraper",
        "Welcome to the Jungle" => "shahidirfan/jungle-job-scraper",
        "HiringCafe" => "memo23/apify-hiring-cafe-scraper",
        "Himalayas" => "inlifeprojects/himalayas-jobs-scraper",
        _ => "unknown",
    }
}

#[tauri::command]
fn list_hunt_profiles(project_slug: String) -> Result<Vec<HuntProfileSummary>, String> {
    let root = project_root(&project_slug)?;
    let hunt_run_dir = root.join("hunt_run");
    if !hunt_run_dir.exists() { return Ok(vec![]); }
    if let Ok(mut entries) = fs::read_dir(&hunt_run_dir) {
        while let Some(Ok(entry)) = entries.next() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let _ = migrate_old_hunt_format(&entry.path());
            }
        }
    }
    let mut profiles = vec![];
    for entry in fs::read_dir(&hunt_run_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() { continue; }
        let name = entry.file_name().to_string_lossy().to_string();
        let config_path = entry.path().join(".hunt_config.json");
        let result_path = entry.path().join(".hunt_result.json");
        let config: HuntConfig = if config_path.exists() {
            serde_json::from_str(&fs::read_to_string(&config_path).map_err(|e| e.to_string())?).map_err(|e| format!("Malformed .hunt_config.json in {name}: {e}"))?
        } else {
            continue;
        };
        let result: HuntResultDB = if result_path.exists() {
            serde_json::from_str(&fs::read_to_string(&result_path).map_err(|e| e.to_string())?).map_err(|e| format!("Malformed .hunt_result.json in {name}: {e}"))?
        } else {
            HuntResultDB { runs: vec![], jobs: HashMap::new() }
        };
        profiles.push(HuntProfileSummary {
            name: config.name.clone(),
            slug: name.clone(),
            job_count: result.jobs.len(),
            run_count: result.runs.len(),
            last_run: config.last_run.clone(),
            created: config.created.clone(),
        });
    }
    Ok(profiles)
}

#[tauri::command]
fn save_hunt_config(input: SaveHuntConfigInput) -> Result<(), String> {
    let root = project_root(&input.project_slug)?;
    let config_path = root.join("hunt_run").join(&input.hunt_slug).join(".hunt_config.json");
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&input.config).map_err(|e| e.to_string())?;
    fs::write(&config_path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn create_hunt_run(input: HuntRunInput) -> Result<HuntRunOutput, String> {
    let root = project_root(&input.project_slug)?;
    let slug = slugify(&input.name);
    if slug.is_empty() { return Err("Hunt name must include letters or numbers".into()); }
    fs::create_dir_all(root.join("hunt_run")).map_err(|e| e.to_string())?;
    let rel_folder = format!("hunt_run/{slug}");
    let folder = safe_project_path(&input.project_slug, &rel_folder)?;
    let config_path = folder.join(".hunt_config.json");
    let results_rel = format!("{rel_folder}/results.md");
    if config_path.exists() {
        return Ok(HuntRunOutput { folder_path: rel_folder.clone(), results_path: results_rel });
    }
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let effective = effective_hunt_for_sites(&input, &input.selected_sites);
    let now_ts = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let config = HuntConfig {
        name: effective.name.clone(),
        slug: slug.clone(),
        created: now_ts.clone(),
        last_run: None,
        roles: effective.roles.clone(),
        location: effective.location.clone(),
        work_mode: effective.work_mode.clone(),
        seniority: effective.seniority.clone(),
        experience: effective.experience.clone(),
        min_salary: effective.min_salary.clone(),
        include_keywords: effective.include_keywords.clone(),
        exclude_keywords: effective.exclude_keywords.clone(),
        posted_within: effective.posted_within.clone(),
        selected_sites: effective.selected_sites.clone(),
        max_items: effective.max_items,
    };
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&config_path, json).map_err(|e| e.to_string())?;
    let empty_db = HuntResultDB { runs: vec![], jobs: HashMap::new() };
    fs::write(folder.join(".hunt_result.json"), serde_json::to_string_pretty(&empty_db).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let roles_md = effective.roles.iter().filter(|r| !r.trim().is_empty()).map(|r| format!("- {}", r.trim())).collect::<Vec<_>>().join("
");
    let actor_lines = effective.selected_sites.iter().map(|s| format!("- {} -> `{}`", s, actor_slug_for_site(s))).collect::<Vec<_>>().join("
");
    let results = format!("# Hunt Results

Run: {}
Created: {}

## Hunt Settings

- Mode: {}
- Max scrape results: {}
- Location: {}
- Posted within: {}
- Seniority: {}
- Experience: {}
- Minimum salary: {}
- Include keywords: {}
- Avoid keywords: {}

### Roles

{}

### Apify Actors

{}

## Status

This hunt run has been created. Results will appear in date-stamped `jobs-YYYY-MM-DD/` folders after each run.

", effective.name, now_ts, effective.work_mode, effective.max_items, format_hunt_setting_value("location", &effective.location), format_hunt_setting_value("postedWithin", &effective.posted_within), format_hunt_setting_value("seniority", &effective.seniority), format_hunt_setting_value("experience", &effective.experience), format_hunt_setting_value("salary", &effective.min_salary), format_hunt_setting_value("includeKeywords", &effective.include_keywords), format_hunt_setting_value("excludeKeywords", &effective.exclude_keywords), if roles_md.is_empty(){"- Not specified".to_string()}else{roles_md}, if actor_lines.is_empty(){"- None".to_string()}else{actor_lines});
    fs::write(folder.join("results.md"), results).map_err(|e| e.to_string())?;
    Ok(HuntRunOutput { folder_path: rel_folder.clone(), results_path: results_rel })
}
fn emit_event(ch: &Channel<AgentRunEvent>, run_id: &str, kind: &str, text: impl Into<String>) {
    let _ = ch.send(AgentRunEvent{run_id: run_id.to_string(), kind: kind.to_string(), text: text.into(), payload: None});
}
fn emit_event_payload(ch: &Channel<AgentRunEvent>, run_id: &str, kind: &str, text: impl Into<String>, payload: serde_json::Value) {
    let _ = ch.send(AgentRunEvent{run_id: run_id.to_string(), kind: kind.to_string(), text: text.into(), payload: Some(payload)});
}

fn actor_api_slug(slug: &str) -> String { slug.replace('/', "~") }
fn hunt_roles_query(h: &HuntRunInput) -> String {
    let roles = h.roles.iter().map(|r| r.trim()).filter(|r| !r.is_empty()).collect::<Vec<_>>().join(" OR ");
    if !roles.is_empty() { roles } else if !h.include_keywords.trim().is_empty() { h.include_keywords.trim().to_string() } else { "software engineer".into() }
}
fn csv_words(s: &str) -> Vec<Value> { s.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()).map(|x| Value::String(x.to_string())).collect() }
fn location_terms(h: &HuntRunInput) -> Vec<Value> { csv_words(&h.location).into_iter().filter(|v| v.as_str() != Some("Worldwide")).collect() }
fn country_code(location: &str) -> String {
    let l = location.to_lowercase();
    if l.contains("united kingdom") || l.contains("uk") { "GB".into() }
    else if l.contains("new zealand") { "NZ".into() }
    else if l.contains("australia") { "AU".into() }
    else if l.contains("canada") { "CA".into() }
    else if l.contains("india") { "IN".into() }
    else if l.contains("singapore") { "SG".into() }
    else if l.contains("germany") { "DE".into() }
    else if l.contains("france") { "FR".into() }
    else if l.contains("netherlands") { "NL".into() }
    else if l.contains("united states") || l.contains("usa") { "US".into() }
    else { "US".into() }
}
fn posted_time_range(h: &HuntRunInput) -> &'static str { match h.posted_within.as_str() { "1 day" | "24 hours" => "24h", "1 week" => "7d", "3 weeks" | "1 month" => "30d", _ => "any" } }
fn indeed_from_days(h: &HuntRunInput) -> &'static str { match h.posted_within.as_str() { "1 week" => "7", _ => "14" } }
fn fantastic_time_range(h: &HuntRunInput) -> &'static str { if h.posted_within == "1 week" { "7d" } else { "6m" } }
fn keyword_array_with_folded_include(h: &HuntRunInput) -> Vec<String> {
    let mut out = h.roles.iter().map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect::<Vec<_>>();
    out.extend(include_keyword_terms(h));
    out
}

fn include_keyword_terms(h: &HuntRunInput) -> Vec<String> {
    csv_words(&h.include_keywords).iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
}

fn query_with_folded_include(h: &HuntRunInput) -> String {
    let mut q = hunt_roles_query(h);
    let iks = include_keyword_terms(h);
    for ik in iks {
        if !ik.is_empty() {
            q.push_str(&format!(" OR {}", ik));
        }
    }
    q
}
fn primary_role_query(h: &HuntRunInput) -> String {
    h.roles.iter()
        .map(|r| r.trim())
        .find(|r| !r.is_empty())
        .unwrap_or("")
        .to_string()
}
fn yc_role_enum(h: &HuntRunInput) -> &'static str {
    let q = hunt_roles_query(h).to_lowercase();
    if q.contains("design") { "designer" }
    else if q.contains("product") { "product-manager" }
    else if q.contains("operation") { "operations" }
    else if q.contains("marketing") { "marketing" }
    else if q.contains("sales") { "sales-manager" }
    else if q.contains("recruit") || q.contains("hr") { "recruiting-hr" }
    else if q.contains("support") { "support" }
    else if q.contains("science") || q.contains("scientist") { "science" }
    else if q.contains("engineer") || q.contains("developer") || q.contains("backend") || q.contains("frontend") || q.contains("full stack") || q.contains("fullstack") || q.contains("ai") || q.contains("ml") { "software-engineer" }
    else { "" }
}
fn yc_location_enum(h: &HuntRunInput) -> &'static str {
    let l = h.location.to_lowercase();
    if h.work_mode == "Remote" || l.contains("remote") { "remote" }
    else if l.contains("san francisco") || l == "sf" { "san-francisco" }
    else if l.contains("new york") || l == "nyc" { "new-york" }
    else if l.contains("los angeles") || l == "la" { "los-angeles" }
    else if l.contains("seattle") { "seattle" }
    else if l.contains("austin") { "austin" }
    else if l.contains("chicago") { "chicago" }
    else if l.contains("india") { "india" }
    else { "" }
}

// Reusable HuntBrief capability helpers (mirrors frontend ACTOR_CAPABILITIES)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActorFieldCapability { Api, PostFilter, Unsupported }

fn actor_field_capability(site: &str, field: &str) -> ActorFieldCapability {
    let (api, post_filter): (&[&str], &[&str]) = match site {
        "54 Career Sites" | "LinkedIn" | "Indeed" => (&["roles","location","postedWithin","includeKeywords"], &[]),
        "YC Startup Jobs" => (&["roles","location"], &["includeKeywords"]),
        "Welcome to the Jungle" => (&["roles","location","postedWithin"], &["includeKeywords"]),
        "HiringCafe" => (&["roles","location"], &["includeKeywords"]),
        "Himalayas" => (&["roles","location"], &["includeKeywords"]),
        _ => (&[], &[]),
    };
    if api.contains(&field) { ActorFieldCapability::Api }
    else if post_filter.contains(&field) { ActorFieldCapability::PostFilter }
    else { ActorFieldCapability::Unsupported }
}
fn field_is_usable(site: &str, field: &str) -> bool {
    !matches!(actor_field_capability(site, field), ActorFieldCapability::Unsupported)
}
fn field_is_api_supported(site: &str, field: &str) -> bool {
    matches!(actor_field_capability(site, field), ActorFieldCapability::Api)
}
fn neutralize_field(out: &mut HuntRunInput, field: &str) {
    match field {
        "roles" => out.roles = vec![],
        "includeKeywords" => out.include_keywords = String::new(),
        "postedWithin" => out.posted_within = String::new(),
        "location" => out.location = "Worldwide".into(),
        _ => {}
    }
}
fn effective_hunt_for_sites(h: &HuntRunInput, sites: &[String]) -> HuntRunInput {
    if sites.is_empty() { return h.clone(); }
    let mut out = h.clone();
    let fields: &[&str] = &["roles","includeKeywords","postedWithin","location"];
    for &field in fields {
        if !sites.iter().any(|s| field_is_usable(s, field)) {
            neutralize_field(&mut out, field);
        }
    }
    out
}
fn effective_hunt_for_site_for_api(h: &HuntRunInput, site: &str) -> HuntRunInput {
    let mut out = h.clone();
    for &field in &["roles","includeKeywords","postedWithin","location"] {
        if !field_is_api_supported(site, field) {
            neutralize_field(&mut out, field);
        }
    }
    out
}

fn format_hunt_setting_value(key: &str, value: &str) -> String {
    if value.is_empty() || value == "Any" || value == "Worldwide" || value == "Standard" {
        match key {
            "postedWithin" | "includeKeywords" | "seniority" | "experience" | "excludeKeywords" => "Not specified".to_string(),
            "min_salary" | "salary" => "Any".to_string(),
            "location" => "Worldwide".to_string(),
            "work_mode" | "workMode" => "Standard".to_string(),
            _ => value.to_string(),
        }
    } else {
        value.to_string()
    }
}

fn build_actor_input(site: &str, h: &HuntRunInput) -> Value {
    let folded_query = query_with_folded_include(h);
    let include = csv_words(&h.include_keywords);
    let locs = location_terms(h);
    let wttj_country_code = if h.location == "Worldwide" { String::new() } else { country_code(&h.location) };
    let max = h.max_items.max(10);
    match site {
        "54 Career Sites" => serde_json::json!({
            "timeRange": fantastic_time_range(h), "limit": max, "includeAi": true,
            "titleSearch": h.roles.iter().filter(|r| !r.trim().is_empty()).cloned().collect::<Vec<_>>(),
            "locationSearch": locs, "descriptionSearch": include,
            "descriptionType": "text"
        }),
        "LinkedIn" => serde_json::json!({
            "timeRange": fantastic_time_range(h), "limit": max, "includeAi": true,
            "titleSearch": h.roles.iter().filter(|r| !r.trim().is_empty()).cloned().collect::<Vec<_>>(),
            "locationSearch": locs, "descriptionSearch": include,
            "descriptionType": "text", "remote": h.work_mode == "Remote"
        }),
        "Indeed" => {
            let mut input = serde_json::json!({
                "query": folded_query,
                "country": country_code(&h.location).to_lowercase(),
                "location": if h.work_mode == "Remote" { "remote" } else { h.location.as_str() },
                "maxRows": max,
                "sort": "date",
                "fromDays": indeed_from_days(h),
                "enableUniqueJobs": true,
                "includeSimilarJobs": false
            });
            if h.work_mode == "Remote" {
                input["remote"] = serde_json::json!("remote");
            }
            input
        },
        "YC Startup Jobs" => serde_json::json!({"mode":"jobs", "role": yc_role_enum(h), "location": yc_location_enum(h), "maxItems": h.max_items}),
        "Welcome to the Jungle" => serde_json::json!({"keyword": primary_role_query(h), "countryCode": wttj_country_code, "posted_within": posted_time_range(h), "results_wanted": h.max_items, "max_pages": 5}),
        "HiringCafe" => serde_json::json!({"keyword": primary_role_query(h), "location": if h.location == "Worldwide" { "" } else { h.location.as_str() }, "workplaceType": if h.work_mode == "Remote" {"Remote"} else {"Any"}, "maxItems": h.max_items, "flattenOutput": true, "enrichDescription": true}),
        "Himalayas" => serde_json::json!({"keywords": h.roles.iter().map(|r| r.trim()).filter(|r| !r.is_empty()).collect::<Vec<_>>(), "employmentType":"Full Time", "worldwide": h.location.contains("Worldwide"), "country": if h.location.contains("Worldwide") {""} else {h.location.as_str()}, "sortBy":"recent", "maxResultsPerKeyword": h.max_items, "filterNonTech": false}),
        _ => serde_json::json!({"query": folded_query, "maxItems": h.max_items})
    }
}

fn curl_json(method: &str, url: &str, token: &str, body: Option<&Value>) -> Result<Value, String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "-X", method, "-H", &format!("Authorization: Bearer {token}"), "-H", "Content-Type: application/json", url]);
    if let Some(b) = body { cmd.arg("--data").arg(serde_json::to_string(b).map_err(|e| e.to_string())?); }
    let out = cmd.output().map_err(|e| format!("Could not run curl: {e}"))?;
    if !out.status.success() { return Err(String::from_utf8_lossy(&out.stderr).trim().to_string()); }
    let text = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&text).map_err(|e| format!("Apify returned invalid JSON: {e}; {text}"))
}

fn run_actor_api(site: &str, actor_slug: &str, h: &HuntRunInput, token: &str, run_id: &str, ch: &Channel<AgentRunEvent>) -> Result<Vec<Value>, String> {
    let input = build_actor_input(site, h);
    emit_event_payload(ch, run_id, "debug", format!("{site}: actor API input"), input.clone());
    emit_event(ch, run_id, "status", format!("Starting {site} actor"));
    let url = format!("https://api.apify.com/v2/acts/{}/runs", actor_api_slug(actor_slug));
    let started = curl_json("POST", &url, token, Some(&input))?;
    let apify_run_id = started["data"]["id"].as_str().ok_or("Apify did not return a run id")?.to_string();
    emit_event(ch, run_id, "status", format!("{site}: actor run started ({apify_run_id})"));
    let mut dataset_id = started["data"]["defaultDatasetId"].as_str().map(|s| s.to_string());
    for _ in 0..180 {
        thread::sleep(Duration::from_secs(2));
        let poll = curl_json("GET", &format!("https://api.apify.com/v2/actor-runs/{apify_run_id}"), token, None)?;
        let status = poll["data"]["status"].as_str().unwrap_or("UNKNOWN");
        emit_event(ch, run_id, "status", format!("{site}: {status}"));
        if dataset_id.is_none() { dataset_id = poll["data"]["defaultDatasetId"].as_str().map(|s| s.to_string()); }
        match status {
            "SUCCEEDED" => break,
            "FAILED" | "ABORTED" | "TIMED-OUT" => return Err(format!("{site} actor {status}")),
            _ => {}
        }
    }
    let dataset_id = dataset_id.ok_or(format!("{site} run did not expose a dataset id"))?;
    emit_event(ch, run_id, "status", format!("{site}: reading dataset items"));
    let items = curl_json("GET", &format!("https://api.apify.com/v2/datasets/{dataset_id}/items?clean=true&format=json&limit={}", h.max_items), token, None)?;
    Ok(items.as_array().cloned().unwrap_or_default())
}

fn job_detail_markdown(idx: usize, j: &HuntJob) -> String {
    let reqs = if j.requirements.is_empty() { String::new() } else {
        format!("\n## Requirements\n\n{}\n", j.requirements.join("; "))
    };
    let skills = if j.skills.is_empty() { String::new() } else {
        format!("\n## Key skills\n\n{}\n", j.skills.iter().map(|s| format!("- {s}")).collect::<Vec<_>>().join("\n"))
    };
    let wm = if j.work_mode.is_empty() { String::new() } else { format!("\n- Work mode: {}", j.work_mode) };
    let sr = if j.seniority.is_empty() { String::new() } else { format!("\n- Seniority: {}", j.seniority) };
    let ex = if j.experience.is_empty() { String::new() } else { format!("\n- Experience: {}", j.experience) };
    let desc = if j.description.is_empty() { String::new() } else { format!("\n## Description\n\n{}\n", j.description) };
    format!("# {idx}. {title} — {company}\n\n## Job Metadata\n\n- Source: {src}\n- Company: {company}\n- Location: {loc}\n- Salary: {sal}\n- Posted: {pt}\n- Apply: {ap}\n- Original URL: {ourl}{wm}{sr}{ex}{reqs}{skills}{desc}\n## Resume / Outreach Notes\n\nUse this file as the focused job context when tailoring a resume, cover letter, or outreach message. Do not load the whole hunt unless comparing jobs.\n",
        title = j.title, company = j.company, src = j.source_name,
        loc = j.location, sal = j.salary, pt = j.posted_date,
        ap = j.apply_url, ourl = j.source_url)
}

fn job_index_markdown(idx: usize, rel_path: &str, j: &HuntJob) -> String {
    let mut meta = vec![];
    if !j.location.is_empty() { meta.push(format!("Location: {}", j.location)); }
    if !j.work_mode.is_empty() { meta.push(format!("Work mode: {}", j.work_mode)); }
    if !j.salary.is_empty() { meta.push(format!("Salary: {}", j.salary)); }
    if !j.posted_date.is_empty() { meta.push(format!("Posted: {}", j.posted_date)); }
    format!("{}. [{} — {}]({})\n   - Source: {}{}\n", idx, j.title, j.company, rel_path, j.source_name, if meta.is_empty(){String::new()}else{format!("\n   - {}", meta.join(" · "))})
}

fn job_file_name(idx: usize, j: &HuntJob) -> String {
    let title = slugify(&j.title).chars().take(42).collect::<String>();
    let company = slugify(&j.company).chars().take(28).collect::<String>();
    format!("{:03}-{}-{}.md", idx, if title.is_empty(){"job"}else{&title}, if company.is_empty(){"company"}else{&company})
}

// --- Import link helpers ---

fn valid_import_url(url: &str) -> bool {
    let url = url.trim();
    url.starts_with("http://") || url.starts_with("https://")
}

fn dedupe_import_urls(urls: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for url in urls {
        let trimmed = url.trim().to_string();
        if valid_import_url(&trimmed) && seen.insert(trimmed.clone()) {
            result.push(trimmed);
        }
    }
    result
}

#[derive(Debug, Clone)]
struct ImportedJobDraft {
    title: String,
    company: String,
    location: String,
    salary: String,
    posted: String,
    deadline: String,
    description: String,
}

fn clean_import_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}

fn import_title_from_content(url: &str, content: &str) -> String {
    for line in content.lines() {
        let trimmed = clean_import_line(line);
        if !trimmed.is_empty() && trimmed.len() < 200 && !is_import_noise_line(&trimmed) {
            return trimmed.chars().take(120).collect();
        }
    }
    let url_path = url.split('/').filter(|s| !s.is_empty()).last().unwrap_or(url);
    url_path.replace('-', " ").replace('_', " ").chars().take(80).collect()
}

fn domain_from_url(url: &str) -> String {
    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host = without_scheme.split('/').next().unwrap_or("");
    host.trim_start_matches("www.").split('.').next().unwrap_or("").replace('-', " ")
}

fn title_from_page_title(page_title: &str) -> String {
    let base = page_title.split('|').next().unwrap_or(page_title).trim();
    for sep in [" hiring ", " at ", " - ", " | "] {
        if let Some((_, rhs)) = base.split_once(sep) {
            let title = rhs.rsplit_once(" in ").map(|(t, _)| t).unwrap_or(rhs).trim();
            if !title.is_empty() { return title.chars().take(120).collect(); }
        }
    }
    base.chars().take(120).collect()
}

fn imported_content_lines(content: &str) -> Vec<String> {
    content.lines().map(clean_import_line).filter(|l| !l.is_empty()).collect()
}

fn is_import_noise_line(line: &str) -> bool {
    let l = line.to_lowercase();
    [
        "join or sign in", "new to linkedin", "by clicking continue", "user agreement", "privacy policy",
        "cookie policy", "save", "report this job", "see who you know", "sign in to create job alert",
        "linkedin is better on the app", "don't have the app", "open the app", "get the app",
        "find curated posts", "never miss a job alert", "referrals increase your chances",
    ].iter().any(|needle| l.contains(needle))
}

fn is_import_stop_line(line: &str) -> bool {
    let l = line.to_lowercase();
    [
        "similar jobs", "people also viewed", "explore top content", "know when new jobs open up", "get notified about new",
        "where to apply", "share this page", "jobs & opportunities", "useful information", "legal information", "follow us",
    ].iter().any(|needle| l.contains(needle))
}

fn import_label_value(lines: &[String], label: &str) -> String {
    let label_l = label.to_lowercase();
    let known_labels = [
        "organisation/company", "organization/company", "department", "research field", "researcher profile", "positions",
        "application deadline", "country", "type of contract", "job status", "hours per week", "offer description",
        "city", "website", "street", "postal code", "e-mail", "contact", "work location(s)", "company/institute",
    ];
    for (i, line) in lines.iter().enumerate() {
        if line.to_lowercase() == label_l {
            for value in lines.iter().skip(i + 1).take(4) {
                let lower = value.to_lowercase();
                if value.is_empty() || known_labels.contains(&lower.as_str()) { continue; }
                return value.clone();
            }
        }
    }
    String::new()
}

fn extract_relative_posted(line: &str) -> String {
    let words = line.split_whitespace().collect::<Vec<_>>();
    for i in 0..words.len() {
        let w = words[i].trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if ["minute", "minutes", "hour", "hours", "day", "days", "week", "weeks", "month", "months", "year", "years"].contains(&w.as_str()) && i + 1 < words.len() && words[i + 1].eq_ignore_ascii_case("ago") {
            let prev = if i > 0 { words[i - 1].trim_matches(|c: char| !c.is_alphanumeric()) } else { "" };
            if !prev.is_empty() {
                return format!("{} {} ago", prev, words[i].trim_matches(|c: char| !c.is_alphanumeric()));
            }
        }
    }
    String::new()
}

fn extract_salary(lines: &[String]) -> String {
    for line in lines.iter().take(80) {
        let has_currency = line.contains('$') || line.contains('£') || line.contains('€');
        let has_digit = line.chars().any(|c| c.is_ascii_digit());
        if has_currency && has_digit && line.len() <= 180 {
            return line.chars().take(160).collect();
        }
    }
    String::new()
}

fn clean_import_description(lines: &[String]) -> String {
    let priority_markers = ["offer description", "job description", "description", "about ", "the role"];
    let fallback_markers = ["what you", "responsibilities", "requirements", "essential qualifications"];
    let mut start_idx = None;
    for marker in priority_markers {
        if let Some(i) = lines.iter().position(|line| line.to_lowercase().starts_with(marker)) {
            start_idx = Some(i);
            break;
        }
    }
    if start_idx.is_none() {
        for marker in fallback_markers {
            if let Some(i) = lines.iter().position(|line| line.to_lowercase().starts_with(marker)) {
                start_idx = Some(i);
                break;
            }
        }
    }

    let mut out = Vec::new();
    for line in lines.iter().skip(start_idx.unwrap_or(0)) {
        if is_import_stop_line(line) { break; }
        if is_import_noise_line(line) { continue; }
        out.push(line.clone());
    }
    let desc = out.join("\n");
    let desc = desc.chars().take(12000).collect::<String>().trim().to_string();
    if desc.is_empty() { "No extracted job description was returned. Use the original URL for additional context.".into() } else { desc }
}

fn parse_imported_job(_original_url: &str, extract_url: &str, page_title: Option<&str>, content: &str) -> ImportedJobDraft {
    let lines = imported_content_lines(content);
    let mut title = page_title.map(title_from_page_title).unwrap_or_default();
    let mut company = String::new();
    let mut location = String::new();
    let mut posted = String::new();

    if extract_url.contains("linkedin.com") {
        if let Some(pt) = page_title {
            let base = pt.split('|').next().unwrap_or(pt).trim();
            if let Some((co, rest)) = base.split_once(" hiring ") {
                company = co.trim().to_string();
                if let Some((t, loc)) = rest.rsplit_once(" in ") {
                    title = t.trim().to_string();
                    location = loc.trim().to_string();
                }
            }
        }
        if title.is_empty() { title = lines.first().cloned().unwrap_or_default(); }
        if company.is_empty() || location.is_empty() {
            if let Some(line) = lines.get(1) {
                if company.is_empty() {
                    let title_words = title.split_whitespace().count();
                    let parts = line.split_whitespace().collect::<Vec<_>>();
                    if parts.len() > title_words.min(3) {
                        company = parts.first().unwrap_or(&"").to_string();
                    }
                }
                if location.is_empty() && !company.is_empty() {
                    location = line.strip_prefix(&company).unwrap_or(line).trim().to_string();
                }
            }
        }
    }

    if title.is_empty() { title = import_title_from_content(extract_url, content); }

    let labeled_company = import_label_value(&lines, "Organisation/Company");
    if company.is_empty() && !labeled_company.is_empty() { company = labeled_company; }
    let deadline = import_label_value(&lines, "Application Deadline");
    let country = import_label_value(&lines, "Country");
    let city = import_label_value(&lines, "City");
    if location.is_empty() {
        location = match (city.is_empty(), country.is_empty()) {
            (false, false) => format!("{city}, {country}"),
            (false, true) => city,
            (true, false) => country,
            (true, true) => String::new(),
        };
    }
    if company.is_empty() { company = domain_from_url(extract_url); }
    for line in lines.iter().take(40) {
        if posted.is_empty() { posted = extract_relative_posted(line); }
        if location.is_empty() && (line.contains(", ") || line.eq_ignore_ascii_case("remote")) && line.len() < 140 && !line.contains("http") {
            location = line.clone();
        }
        if !posted.is_empty() && !location.is_empty() { break; }
    }

    ImportedJobDraft {
        title: title.trim().chars().take(120).collect(),
        company: company.trim().chars().take(120).collect(),
        location: location.trim().chars().take(160).collect(),
        salary: extract_salary(&lines),
        posted,
        deadline,
        description: clean_import_description(&lines),
    }
}

fn query_param(url: &str, name: &str) -> Option<String> {
    let query = url.split_once('?')?.1.split_once('#').map(|(q, _)| q).unwrap_or_else(|| url.split_once('?').unwrap().1);
    for part in query.split('&') {
        let (key, value) = part.split_once('=').unwrap_or((part, ""));
        if key == name && !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn canonical_import_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches(|c| matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']'));
    if trimmed.contains("linkedin.com/jobs/search") {
        if let Some(id) = query_param(trimmed, "currentJobId") {
            return format!("https://www.linkedin.com/jobs/view/{id}");
        }
    }
    trimmed.to_string()
}

fn import_source_name(url: &str) -> &'static str {
    if url.contains("linkedin.com") { "LinkedIn Import" } else { "Imported Link" }
}

fn import_link_file_name(idx: usize, url: &str) -> String {
    let slug = slugify(url.split('/').filter(|s| !s.is_empty()).last().unwrap_or("link"));
    let safe_slug = if slug.is_empty() { "link".to_string() } else { slug };
    format!("{:03}-{}.md", idx, safe_slug.chars().take(48).collect::<String>())
}

fn import_link_markdown(original_url: &str, extract_url: &str, job: &ImportedJobDraft) -> String {
    let title = if job.title.is_empty() { "Imported job" } else { &job.title };
    let title_line = if job.company.is_empty() { title.to_string() } else { format!("{title} — {}", job.company) };
    let source = import_source_name(extract_url);
    format!("# {title_line}\n\n## Job Metadata\n\n- Source: {source}\n- Company: {company}\n- Location: {location}\n- Salary: {salary}\n- Posted: {posted}\n- Deadline: {deadline}\n- Apply: {extract_url}\n- Original URL: {original_url}\n\n## Description\n\n{description}\n",
        company = job.company,
        location = job.location,
        salary = job.salary,
        posted = job.posted,
        deadline = job.deadline,
        description = job.description)
}

// --- Firecrawl Import Link helpers ---

fn firecrawl_scrape_options_body() -> serde_json::Value {
    serde_json::json!({
        "formats": ["markdown"],
        "onlyMainContent": true,
        "onlyCleanContent": true,
        "timeout": 60000,
        "proxy": "auto",
        "blockAds": true,
        "removeBase64Images": true
    })
}

fn execute_firecrawl_request(method: &str, endpoint: &str, key: &str, body: Option<&serde_json::Value>) -> Result<(serde_json::Value, bool), String> {
    let url = if endpoint.starts_with('/') {
        format!("https://api.firecrawl.dev/v2{}", endpoint)
    } else {
        // endpoint is already a full URL (e.g. for batch pagination "next")
        endpoint.to_string()
    };
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-sS", "--max-time", "90", "--connect-timeout", "10", "-X", method, &url,
        "-H", &format!("Authorization: Bearer {key}"),
        "-H", "Content-Type: application/json"]);
    if let Some(b) = body {
        cmd.arg("--data").arg(serde_json::to_string(b).map_err(|e| e.to_string())?);
    }
    let out = cmd.output().map_err(|e| format!("Firecrawl request failed: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let text = format!("{}{}", stdout, stderr);
    let http_ok = out.status.success();
    let v: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| format!("Firecrawl response parse: {e}; body: {}", text.chars().take(200).collect::<String>()))?;
    Ok((v, http_ok))
}

fn firecrawl_scrape_url(key: &str, url: &str, on_event: &Channel<AgentRunEvent>, run_id: &str) -> Result<ImportedExtractPage, String> {
    let opts = firecrawl_scrape_options_body();
    let body = serde_json::json!({
        "url": url,
        "formats": opts["formats"],
        "onlyMainContent": opts["onlyMainContent"],
        "onlyCleanContent": opts["onlyCleanContent"],
        "timeout": opts["timeout"],
        "proxy": opts["proxy"],
        "blockAds": opts["blockAds"],
        "removeBase64Images": opts["removeBase64Images"]
    });

    let (v, http_ok) = execute_firecrawl_request("POST", "/scrape", key, Some(&body))?;

    let success = v.get("success").and_then(|x| x.as_bool()).unwrap_or(false);
    if !success || !http_ok {
        let err = v.get("error").and_then(|x| x.as_str()).unwrap_or("Unknown Firecrawl scrape error").to_string();
        return Ok(ImportedExtractPage {
            original_url: url.to_string(),
            extract_url: url.to_string(),
            final_url: None,
            title: None,
            content: String::new(),
            error: Some(err),
        });
    }

    let markdown = v.pointer("/data/markdown").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let title = v.pointer("/data/metadata/title").and_then(|x| match x {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(arr) => arr.first().and_then(|v| v.as_str().map(|s| s.to_string())),
        _ => None,
    });
    let final_url = v.pointer("/data/metadata/url").and_then(|x| x.as_str()).map(|s| s.to_string());
    let source_url = v.pointer("/data/metadata/sourceURL").and_then(|x| x.as_str()).map(|s| s.to_string());

    emit_event(on_event, run_id, "debug", format!("Firecrawl scrape OK: url={:?}, title={:?}, content_len={}", final_url.as_deref().unwrap_or(url), title, markdown.len()));

    Ok(ImportedExtractPage {
        original_url: url.to_string(),
        extract_url: source_url.unwrap_or_else(|| url.to_string()),
        final_url,
        title,
        content: markdown,
        error: None,
    })
}

fn firecrawl_start_batch_scrape(key: &str, urls: &[String], on_event: &Channel<AgentRunEvent>, run_id: &str) -> Result<String, String> {
    let opts = firecrawl_scrape_options_body();
    let body = serde_json::json!({
        "urls": urls,
        "formats": opts["formats"],
        "onlyMainContent": opts["onlyMainContent"],
        "onlyCleanContent": opts["onlyCleanContent"],
        "timeout": opts["timeout"],
        "proxy": opts["proxy"],
        "blockAds": opts["blockAds"],
        "removeBase64Images": opts["removeBase64Images"],
        "ignoreInvalidURLs": true
    });

    let (v, http_ok) = execute_firecrawl_request("POST", "/batch/scrape", key, Some(&body))?;

    if !http_ok {
        let err = v.get("error").and_then(|x| x.as_str()).unwrap_or("Unknown batch scrape error").to_string();
        return Err(format!("Firecrawl batch scrape failed: {err}"));
    }

    let success = v.get("success").and_then(|x| x.as_bool()).unwrap_or(false);
    if !success {
        let err = v.get("error").and_then(|x| x.as_str()).unwrap_or("Unknown batch scrape error").to_string();
        return Err(format!("Firecrawl batch scrape not successful: {err}"));
    }

    let batch_id = v.get("id").and_then(|x| x.as_str()).ok_or("Firecrawl batch scrape did not return an id")?.to_string();

    // Emit invalid URLs if any
    if let Some(invalid) = v.get("invalidURLs").and_then(|x| x.as_array()) {
        if !invalid.is_empty() {
            let invalid_strs: Vec<String> = invalid.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
            emit_event(on_event, run_id, "debug", format!("Firecrawl invalid URLs: {}", invalid_strs.join(", ")));
        }
    }

    Ok(batch_id)
}

fn firecrawl_poll_batch_scrape(key: &str, batch_id: &str, url_mappings: &[(String, String)], on_event: &Channel<AgentRunEvent>, run_id: &str) -> Result<Vec<ImportedExtractPage>, String> {
    let endpoint = format!("/batch/scrape/{}", batch_id);
    let max_polls = 60;
    let mut all_pages: Vec<ImportedExtractPage> = Vec::new();

    for poll in 0..max_polls {
        thread::sleep(Duration::from_secs(2));

        let (v, http_ok) = execute_firecrawl_request("GET", &endpoint, key, None)?;

        if !http_ok {
            let err = v.get("error").and_then(|x| x.as_str()).unwrap_or("Unknown batch status error").to_string();
            emit_event(on_event, run_id, "debug", format!("Firecrawl batch status HTTP error: {err}"));
            if poll >= 5 {
                return Err(format!("Firecrawl batch status failed after {} polls: {err}", poll + 1));
            }
            continue;
        }

        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("unknown").to_string();
        let completed = v.get("completed").and_then(|x| x.as_i64()).unwrap_or(0);
        let total = v.get("total").and_then(|x| x.as_i64()).unwrap_or(0);

        emit_event(on_event, run_id, "status", format!("Firecrawl batch: {status} ({completed}/{total})"));

        // Collect data from current response (including "next" pages)
        all_pages.append(&mut firecrawl_collect_batch_data(&v, url_mappings));

        // Follow next pages if present
        if let Some(next_url) = v.get("next").and_then(|x| x.as_str()).map(|s| s.to_string()) {
            emit_event(on_event, run_id, "debug", "Firecrawl batch fetching next page");
            let next_result = firecrawl_fetch_batch_next(key, &next_url, url_mappings);
            if let Ok(mut more_pages) = next_result {
                all_pages.append(&mut more_pages);
            }
        }

        if status == "completed" {
            if let Some(credits) = v.get("creditsUsed").and_then(|x| x.as_i64()) {
                emit_event(on_event, run_id, "debug", format!("Firecrawl batch completed: {completed}/{total} pages, credits used: {credits}"));
            } else {
                emit_event(on_event, run_id, "debug", format!("Firecrawl batch completed: {completed}/{total} pages"));
            }
            break;
        }

        if status == "failed" {
            let err = v.get("error").and_then(|x| x.as_str()).unwrap_or("Batch scrape failed").to_string();
            emit_event(on_event, run_id, "debug", format!("Firecrawl batch failed: {err}"));
            // Try to get batch errors for diagnostics
            if let Ok(errors_result) = firecrawl_get_batch_errors(key, batch_id) {
                for err_item in errors_result {
                    emit_event(on_event, run_id, "debug", format!("Firecrawl batch error detail: url={:?}, error={:?}", err_item.url, err_item.error));
                }
            }
            return Err(err);
        }
    }

    Ok(all_pages)
}

fn firecrawl_collect_batch_data(v: &serde_json::Value, url_mappings: &[(String, String)]) -> Vec<ImportedExtractPage> {
    let mut pages = Vec::new();
    if let Some(data) = v.get("data").and_then(|x| x.as_array()) {
        for item in data {
            let markdown = item.get("markdown").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let page_title = item.pointer("/metadata/title").and_then(|x| match x {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Array(arr) => arr.first().and_then(|v| v.as_str().map(|s| s.to_string())),
                _ => None,
            });
            let final_url = item.pointer("/metadata/url").and_then(|x| x.as_str()).map(|s| s.to_string());
            let source_url = item.pointer("/metadata/sourceURL").and_then(|x| x.as_str()).map(|s| s.to_string());
            let page_error = item.pointer("/metadata/error").and_then(|x| x.as_str()).map(|s| s.to_string());

            let extract_url = source_url.clone().unwrap_or_default();

            // Find original URL from mapping
            let original_url = if !extract_url.is_empty() {
                url_mappings.iter()
                    .find(|(_, c)| c == &extract_url)
                    .map(|(o, _)| o.clone())
                    .or_else(|| {
                        final_url.as_ref().and_then(|fu| {
                            url_mappings.iter()
                                .find(|(_, c)| c == fu)
                                .map(|(o, _)| o.clone())
                        })
                    })
                    .unwrap_or_else(|| extract_url.clone())
            } else {
                String::new()
            };

            pages.push(ImportedExtractPage {
                original_url,
                extract_url: source_url.unwrap_or_default(),
                final_url,
                title: page_title,
                content: markdown,
                error: page_error,
            });
        }
    }
    pages
}

fn firecrawl_fetch_batch_next(key: &str, next_url: &str, url_mappings: &[(String, String)]) -> Result<Vec<ImportedExtractPage>, String> {
    let (v, _http_ok) = execute_firecrawl_request("GET", next_url, key, None)?;
    Ok(firecrawl_collect_batch_data(&v, url_mappings))
}

fn firecrawl_get_batch_errors(key: &str, batch_id: &str) -> Result<Vec<FirecrawlBatchErrorItem>, String> {
    let endpoint = format!("/batch/scrape/{}/errors", batch_id);
    let (v, _http_ok) = execute_firecrawl_request("GET", &endpoint, key, None)?;

    let mut items = Vec::new();
    if let Some(errors) = v.get("errors").and_then(|x| x.as_array()) {
        for item in errors {
            items.push(FirecrawlBatchErrorItem {
                id: item.get("id").and_then(|x| x.as_str()).map(|s| s.to_string()),
                url: item.get("url").and_then(|x| x.as_str()).map(|s| s.to_string()),
                error: item.get("error").and_then(|x| x.as_str()).map(|s| s.to_string()),
            });
        }
    }
    if let Some(robots_blocked) = v.get("robotsBlocked").and_then(|x| x.as_array()) {
        for url_val in robots_blocked {
            if let Some(u) = url_val.as_str() {
                items.push(FirecrawlBatchErrorItem {
                    id: None,
                    url: Some(u.to_string()),
                    error: Some("Blocked by robots.txt".to_string()),
                });
            }
        }
    }
    Ok(items)
}

#[tauri::command]
fn import_job_links(input: ImportJobLinksInput, on_event: Channel<AgentRunEvent>) -> Result<String, String> {
    if input.urls.is_empty() { return Err("At least one URL is required".into()); }
    let total = input.urls.len();
    let run_id = if input.run_id.is_empty() { format!("import-{}", uuid::Uuid::new_v4()) } else { input.run_id.clone() };
    let return_run_id = run_id.clone();
    let folder_name = slugify(&input.name);
    if folder_name.is_empty() { return Err("Import name must include letters or numbers".into()); }
    let import_parent = match safe_project_path(&input.project_slug, "import-links") { Ok(p) => p, Err(e) => { return Err(e); } };
    fs::create_dir_all(&import_parent).map_err(|e| format!("Could not create import-links folder: {e}"))?;
    let import_base = format!("import-links/{}", folder_name);
    let folder_path = match safe_project_path(&input.project_slug, &import_base) { Ok(p) => p, Err(e) => { return Err(e); } };

    // Read Firecrawl API key early
    let firecrawl_key = match read_firecrawl_key() {
        Ok(Some(key)) => key,
        Ok(None) => return Err("Firecrawl API key is missing. Add firecrawlApiKey to settings.".into()),
        Err(e) => return Err(format!("Could not read Firecrawl API key: {e}")),
    };

    thread::spawn(move || {
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            emit_event(&on_event, &run_id, "started", format!("Starting import: {} URLs", total));
            if let Err(e) = fs::create_dir_all(&folder_path) {
                emit_event(&on_event, &run_id, "failed", format!("Could not create import folder: {e}"));
                return;
            }
            emit_event(&on_event, &run_id, "status", format!("Created import folder: {import_base}"));

            // Canonicalize all URLs first and build mapping
            let mut url_mappings: Vec<(String, String)> = Vec::new();
            for url in &input.urls {
                let original_url = url.trim().to_string();
                if !valid_import_url(&original_url) {
                    emit_event(&on_event, &run_id, "status", format!("Skipping invalid URL: {original_url}"));
                    continue;
                }
                let canonical = canonical_import_url(&original_url);
                if canonical != original_url {
                    emit_event(&on_event, &run_id, "debug", format!("Normalized import URL: {original_url} -> {canonical}"));
                }
                url_mappings.push((original_url, canonical));
            }

            let total_valid = url_mappings.len();
            if total_valid == 0 {
                emit_event(&on_event, &run_id, "failed", "No valid URLs to import");
                return;
            }

            let mut files = Vec::new();
            let mut failures = Vec::new();
            let mut extracted_count = 0usize;
            let mut written_count = 0usize;

            if total_valid == 1 {
                // Single URL: use Firecrawl /v2/scrape
                let (original_url, extract_url) = &url_mappings[0];
                emit_event(&on_event, &run_id, "debug", "Using Firecrawl single scrape");
                emit_event_payload(&on_event, &run_id, "status", format!("Extracting: {}", extract_url.chars().take(80).collect::<String>()),
                    serde_json::json!({"index": 1, "total": 1, "url": extract_url, "originalUrl": original_url}));

                match firecrawl_scrape_url(&firecrawl_key, extract_url, &on_event, &run_id) {
                    Ok(page) => {
                        extracted_count += 1;
                        if let Some(err) = &page.error {
                            emit_event(&on_event, &run_id, "debug", format!("Firecrawl failed to extract {extract_url}: {err}"));
                            failures.push(ImportLinkFailure { url: original_url.clone(), error: err.clone() });
                            files.push(ImportLinkFile { url: original_url.clone(), title: String::new(), file_path: None, extracted: false, error: Some(err.clone()) });
                        } else {
                            let final_url = page.final_url.as_deref().unwrap_or(&page.extract_url);
                            let job = parse_imported_job(original_url, final_url, page.title.as_deref(), &page.content);
                            let title = job.title.clone();
                            let file_name = import_link_file_name(1, final_url);
                            let file_rel = format!("{import_base}/{file_name}");
                            let file_path = match safe_project_path(&input.project_slug, &file_rel) {
                                Ok(p) => p,
                                Err(e) => {
                                    emit_event(&on_event, &run_id, "debug", format!("Could not resolve file path: {e}"));
                                    files.push(ImportLinkFile { url: original_url.clone(), title, file_path: None, extracted: true, error: Some(format!("File write error: {e}")) });
                                    return;
                                }
                            };
                            let md = import_link_markdown(original_url, final_url, &job);
                            if let Err(e) = fs::write(&file_path, md) {
                                emit_event(&on_event, &run_id, "debug", format!("Write failed: {e}"));
                                files.push(ImportLinkFile { url: original_url.clone(), title, file_path: None, extracted: true, error: Some(e.to_string()) });
                            } else {
                                written_count += 1;
                                emit_event(&on_event, &run_id, "debug", format!("Wrote {file_rel}"));
                                files.push(ImportLinkFile { url: original_url.clone(), title, file_path: Some(file_rel), extracted: true, error: None });
                            }
                        }
                    },
                    Err(e) => {
                        emit_event(&on_event, &run_id, "debug", format!("Firecrawl scrape error for {extract_url}: {e}"));
                        let err_str = e.to_string();
                        failures.push(ImportLinkFailure { url: original_url.clone(), error: err_str.clone() });
                        files.push(ImportLinkFile { url: original_url.clone(), title: String::new(), file_path: None, extracted: false, error: Some(format!("Extraction error: {err_str}")) });
                    }
                }
            } else {
                // Multi-URL: use Firecrawl /v2/batch/scrape in chunks of 10
                emit_event(&on_event, &run_id, "debug", format!("Using Firecrawl batch scrape: {} URLs", total_valid));
                let num_chunks = (total_valid + 9) / 10;
                for chunk_idx in 0..num_chunks {
                    let chunk_start = chunk_idx * 10;
                    let chunk_end = std::cmp::min(chunk_start + 10, total_valid);
                    if chunk_start >= chunk_end { break; }

                    let chunk_urls: Vec<String> = url_mappings[chunk_start..chunk_end].iter().map(|(_, c)| c.clone()).collect();
                    let chunk_mappings: Vec<(String, String)> = url_mappings[chunk_start..chunk_end].to_vec();

                    emit_event(&on_event, &run_id, "debug", format!("Batch chunk {}/{}: {} URLs", chunk_idx + 1, num_chunks, chunk_urls.len()));

                    let batch_id = match firecrawl_start_batch_scrape(&firecrawl_key, &chunk_urls, &on_event, &run_id) {
                        Ok(id) => id,
                        Err(e) => {
                            emit_event(&on_event, &run_id, "debug", format!("Firecrawl batch start error: {e}"));
                            for (original_url, _) in &chunk_mappings {
                                failures.push(ImportLinkFailure { url: original_url.clone(), error: format!("Batch start error: {e}") });
                                files.push(ImportLinkFile { url: original_url.clone(), title: String::new(), file_path: None, extracted: false, error: Some(format!("Batch error: {e}")) });
                            }
                            continue;
                        }
                    };

                    emit_event(&on_event, &run_id, "debug", format!("Firecrawl batch id: {batch_id}"));

                    let batch_pages = match firecrawl_poll_batch_scrape(&firecrawl_key, &batch_id, &chunk_mappings, &on_event, &run_id) {
                        Ok(pages) => pages,
                        Err(e) => {
                            emit_event(&on_event, &run_id, "debug", format!("Firecrawl batch poll error: {e}"));
                            for (original_url, _) in &chunk_mappings {
                                failures.push(ImportLinkFailure { url: original_url.clone(), error: format!("Batch poll error: {e}") });
                                files.push(ImportLinkFile { url: original_url.clone(), title: String::new(), file_path: None, extracted: false, error: Some(format!("Batch error: {e}")) });
                            }
                            continue;
                        }
                    };

                    for page in &batch_pages {
                        extracted_count += 1;
                        if let Some(err) = &page.error {
                            emit_event(&on_event, &run_id, "debug", format!("Firecrawl batch failed for {}: {err}", page.extract_url));
                            failures.push(ImportLinkFailure { url: page.original_url.clone(), error: err.clone() });
                            files.push(ImportLinkFile { url: page.original_url.clone(), title: String::new(), file_path: None, extracted: false, error: Some(err.clone()) });
                        } else {
                            let final_url = page.final_url.as_deref().unwrap_or(&page.extract_url);
                            let job = parse_imported_job(&page.original_url, final_url, page.title.as_deref(), &page.content);
                            let title = job.title.clone();
                            let file_name = import_link_file_name(extracted_count, final_url);
                            let file_rel = format!("{import_base}/{file_name}");
                            let file_path = match safe_project_path(&input.project_slug, &file_rel) {
                                Ok(p) => p,
                                Err(e) => {
                                    emit_event(&on_event, &run_id, "debug", format!("Could not resolve file path: {e}"));
                                    files.push(ImportLinkFile { url: page.original_url.clone(), title, file_path: None, extracted: true, error: Some(format!("File write error: {e}")) });
                                    continue;
                                }
                            };
                            let md = import_link_markdown(&page.original_url, final_url, &job);
                            if let Err(e) = fs::write(&file_path, md) {
                                emit_event(&on_event, &run_id, "debug", format!("Write failed: {e}"));
                                files.push(ImportLinkFile { url: page.original_url.clone(), title, file_path: None, extracted: true, error: Some(e.to_string()) });
                            } else {
                                written_count += 1;
                                emit_event(&on_event, &run_id, "debug", format!("Wrote {file_rel}"));
                                files.push(ImportLinkFile { url: page.original_url.clone(), title, file_path: Some(file_rel), extracted: true, error: None });
                            }
                        }
                    }
                }
            }

            let summary = ImportJobLinksSummary {
                submitted: total,
                extracted: extracted_count,
                written: written_count,
                failed: failures.len(),
                folder_path: import_base.clone(),
                files,
                failures,
            };
            let summary_text = format!("Import complete: {written_count} written, {} failed out of {total}", summary.failed);
            emit_event_payload(&on_event, &run_id, "completed", summary_text,
                serde_json::to_value(&summary).unwrap_or_default());
        }));
        if let Err(panic) = panic_result {
            let msg = if let Some(s) = panic.downcast_ref::<&str>() { s.to_string() }
                      else if let Some(s) = panic.downcast_ref::<String>() { s.clone() }
                      else { "Unexpected panic in import thread".to_string() };
            emit_event(&on_event, &run_id, "failed", format!("Import thread panicked: {msg}"));
        }
    });
    Ok(return_run_id)
}

#[tauri::command]
fn start_hunt_apify(mut input: RunHuntApifyInput, on_event: Channel<AgentRunEvent>) -> Result<String, String> {
    let run_id = input.run_id.clone();
    let return_run_id = run_id.clone();
    let token = read_apify_key()?.ok_or("Connect Apify API in Settings first")?;
    thread::spawn(move || {
        emit_event(&on_event, &run_id, "started", "Starting Apify Actor API hunt");
        let results_path = match safe_project_path(&input.hunt.project_slug, &input.results_path) { Ok(p) => p, Err(e) => { emit_event(&on_event, &run_id, "failed", e); return; } };
        let hunt_folder = match results_path.parent() { Some(p) => p.to_path_buf(), None => { emit_event(&on_event, &run_id, "failed", "Could not resolve hunt directory"); return; } };
        let _ = migrate_old_hunt_format(&hunt_folder);
        // If re-running, load settings from stored .hunt_config.json
        if input.is_re_run {
            let config_path = hunt_folder.join(".hunt_config.json");
            if let Ok(config) = serde_json::from_str::<HuntConfig>(&fs::read_to_string(&config_path).unwrap_or_default()) {
                input.hunt = HuntRunInput {
                    project_slug: input.hunt.project_slug.clone(),
                    name: config.name,
                    roles: config.roles,
                    location: config.location,
                    work_mode: config.work_mode,
                    seniority: config.seniority,
                    experience: config.experience,
                    min_salary: config.min_salary,
                    include_keywords: config.include_keywords,
                    exclude_keywords: config.exclude_keywords,
                    posted_within: config.posted_within,
                    selected_sites: config.selected_sites,
                    max_items: config.max_items,
                };
            }
        }
        // Compute run-level effective settings after any re-run config load.
        let effective = effective_hunt_for_sites(&input.hunt, &input.hunt.selected_sites);
        let result_db_path = hunt_folder.join(".hunt_result.json");
        let mut result_db: HuntResultDB = if result_db_path.exists() {
            match serde_json::from_str(&fs::read_to_string(&result_db_path).unwrap_or_default()) {
                Ok(db) => db,
                Err(_) => HuntResultDB { runs: vec![], jobs: HashMap::new() }
            }
        } else {
            HuntResultDB { runs: vec![], jobs: HashMap::new() }
        };
        let before_count = result_db.jobs.len();
        // Run actors with per-site effective settings
        let mut all = Vec::<(String, Value)>::new();
        let mut failures = Vec::<String>::new();
        for site in &input.hunt.selected_sites {
            let actor = actor_slug_for_site(site);
            if actor == "unknown" { failures.push(format!("{site}: unknown actor")); continue; }
            // Compute per-site API-effective settings; post-filter still uses run-level effective settings.
            let api_hunt = effective_hunt_for_site_for_api(&effective, site);
            let mut api_fields = Vec::<&str>::new();
            let mut post_filter_fields = Vec::<&str>::new();
            let mut unsupported_fields = Vec::<&str>::new();
            for f in &["roles","includeKeywords","postedWithin","location"] {
                match actor_field_capability(site, f) {
                    ActorFieldCapability::Api => api_fields.push(f),
                    ActorFieldCapability::PostFilter => post_filter_fields.push(f),
                    ActorFieldCapability::Unsupported => unsupported_fields.push(f),
                }
            }
            let debug_payload = serde_json::json!({
                "site": site,
                "actorSlug": actor,
                "apiEffectiveFields": {
                    "roles": api_hunt.roles,
                    "includeKeywords": api_hunt.include_keywords,
                    "postedWithin": api_hunt.posted_within,
                    "location": api_hunt.location,
                    "workMode": api_hunt.work_mode
                },
                "postFilterEffectiveFields": {
                    "includeKeywords": effective.include_keywords
                },
                "apiFields": api_fields,
                "postFilterFields": post_filter_fields,
                "ignoredFields": unsupported_fields
            });
            emit_event_payload(&on_event, &run_id, "debug", format!("{site}: capability-adjusted fields"), debug_payload);
            match run_actor_api(site, actor, &api_hunt, &token, &run_id, &on_event) {
                Ok(items) => { emit_event(&on_event, &run_id, "status", format!("{site}: fetched {} items", items.len())); for item in items { all.push((site.clone(), item)); } }
                Err(e) => { emit_event(&on_event, &run_id, "status", format!("{site} failed: {e}")); failures.push(format!("{site}: {e}")); }
            }
        }
        let raw_found = all.len();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let mut new_jobs = Vec::<(String, HuntJob)>::new();
        let mut filtered_reasons: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut filtered_total = 0usize;
        let mut intra_duplicate = 0usize;
        let mut already_seen = 0usize;
        // Track per-source filter counts and normalize samples
        let mut source_filtered: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut source_filter_examples: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let mut source_raw_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut source_normalized_samples: std::collections::HashMap<String, Vec<serde_json::Value>> = std::collections::HashMap::new();
        for (source, item) in &all {
            *source_raw_counts.entry(source.clone()).or_insert(0) += 1;
            let norm = normalize_hunt_job(source, item);
            // Capture first 3 normalized samples per source for debug
            let samples = source_normalized_samples.entry(source.clone()).or_default();
            if samples.len() < 3 {
                samples.push(serde_json::json!({
                    "title": norm.title, "company": norm.company,
                    "posted_date": norm.posted_date, "seniority": norm.seniority,
                    "experience": norm.experience, "salary": norm.salary,
                    "location": norm.location, "work_mode": norm.work_mode,
                    "apply_url": norm.apply_url.chars().take(80).collect::<String>()
                }));
            }
            // Use per-site post-filter-effective settings
            let site_filter_hunt = effective.clone();
            if let Some(reason) = post_filter_reason(&site_filter_hunt, &norm) {
                filtered_total += 1;
                *filtered_reasons.entry(reason.clone()).or_insert(0) += 1;
                *source_filtered.entry(source.clone()).or_insert(0) += 1;
                let examples = source_filter_examples.entry(source.clone()).or_default();
                if examples.len() < 3 {
                    let include_terms = filter_terms(&site_filter_hunt.include_keywords).join(", ");
                    let preview = job_search_text(&norm).chars().take(180).collect::<String>();
                    examples.push(format!("{} @ {} — {} | includeTerms=[{}] | textPreview={}", norm.title, norm.company, reason, include_terms, preview));
                }
                continue;
            }
            let dedup_key = hunt_job_dedup_key(&norm);
            if result_db.jobs.contains_key(&dedup_key) { already_seen += 1; continue; }
            if new_jobs.iter().any(|(_, j)| hunt_job_dedup_key(j) == dedup_key) { intra_duplicate += 1; continue; }
            new_jobs.push((source.clone(), norm));
            if new_jobs.len() >= input.hunt.max_items { break; }
        }
        // Emit normalize samples per source
        let normalize_debug: std::collections::HashMap<String, serde_json::Value> = source_normalized_samples.iter().map(|(s, samples)| {
            (s.clone(), serde_json::json!({"count": source_raw_counts.get(s).copied().unwrap_or(0), "samples": samples}))
        }).collect();
        emit_event_payload(&on_event, &run_id, "debug", "Normalized job samples (first 3 per source)", serde_json::to_value(&normalize_debug).unwrap_or_default());
        // Emit post-filter debug event
        let filter_debug = serde_json::json!({
            "rawCount": raw_found,
            "keptNew": new_jobs.len(),
            "duplicateCount": already_seen + intra_duplicate,
            "filteredTotal": filtered_total,
            "reasons": filtered_reasons,
            "perSourceFiltered": source_filtered,
            "includeTerms": filter_terms(&effective.include_keywords),
            "examples": source_filter_examples
        });
        emit_event_payload(&on_event, &run_id, "debug", format!("Post-filter: {filtered_total} filtered, {} kept, {} duplicate", new_jobs.len(), already_seen + intra_duplicate), filter_debug);
        // Warn if any source had >80% of its results filtered
        for source in &input.hunt.selected_sites {
            let raw = source_raw_counts.get(source).copied().unwrap_or(0);
            let filtered = source_filtered.get(source).copied().unwrap_or(0);
            if raw > 0 && filtered as f64 / raw as f64 > 0.8 {
                emit_event(&on_event, &run_id, "status", format!("⚠ {source}: {filtered}/{raw} filtered ({:.0}%) — possible field mapping or filter issue", (filtered as f64 / raw as f64) * 100.0));
            }
        }
        emit_event(&on_event, &run_id, "status", format!("Filtered {filtered_total}, already seen {already_seen}, intra-run duplicates {intra_duplicate}"));
        // Write new jobs to date-stamped folder
        let job_dir_name = format!("jobs-{today}");
        let jobs_dir = hunt_folder.join(&job_dir_name);
        if !new_jobs.is_empty() {
            if let Err(e) = fs::create_dir_all(&jobs_dir) { emit_event(&on_event, &run_id, "failed", e.to_string()); return; }
        }
        new_jobs.sort_by(|(_, a), (_, b)| {
            a.title.to_lowercase().cmp(&b.title.to_lowercase())
                .then_with(|| a.company.to_lowercase().cmp(&b.company.to_lowercase()))
        });
        let mut job_links = Vec::<(String, HuntJob)>::new();
        for (i, (_source, norm)) in new_jobs.iter().enumerate() {
            let file_name = job_file_name(i+1, norm);
            let detail_path = jobs_dir.join(&file_name);
            if let Err(e) = fs::write(&detail_path, job_detail_markdown(i+1, norm)) { emit_event(&on_event, &run_id, "failed", e.to_string()); return; }
            let key = hunt_job_dedup_key(norm);
            result_db.jobs.insert(key, HuntJobEntry {
                title: norm.title.clone(),
                company: norm.company.clone(),
                apply_url: norm.apply_url.clone(),
                source_name: norm.source_name.clone(),
                first_seen: today.clone(),
                last_seen: today.clone(),
                file_path: format!("{job_dir_name}/{file_name}"),
            });
            job_links.push((format!("{job_dir_name}/{file_name}"), norm.clone()));
        }
        // Add run entry
        let now_ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        result_db.runs.push(HuntRunEntry {
            date: today.clone(),
            new_jobs: new_jobs.len(),
            filtered: filtered_total,
            duplicates: already_seen + intra_duplicate,
            sources_failed: failures.clone(),
            run_at: now_ts,
        });
        // Update last_run and settings in config with effective settings
        let config_path = hunt_folder.join(".hunt_config.json");
        if config_path.exists() {
            if let Ok(mut config) = serde_json::from_str::<HuntConfig>(&fs::read_to_string(&config_path).unwrap_or_default()) {
                config.last_run = Some(today.clone());
                config.roles = effective.roles.clone();
                config.location = effective.location.clone();
                config.work_mode = effective.work_mode.clone();
                config.seniority = effective.seniority.clone();
                config.experience = effective.experience.clone();
                config.min_salary = effective.min_salary.clone();
                config.include_keywords = effective.include_keywords.clone();
                config.exclude_keywords = effective.exclude_keywords.clone();
                config.posted_within = effective.posted_within.clone();
                config.max_items = effective.max_items;
                if let Ok(json) = serde_json::to_string_pretty(&config) {
                    let _ = fs::write(&config_path, json);
                }
            }
        }
        // Write updated .hunt_result.json
        if let Ok(db_json) = serde_json::to_string_pretty(&result_db) {
            if let Err(e) = fs::write(&result_db_path, db_json) { emit_event(&on_event, &run_id, "failed", e.to_string()); return; }
        }
        // Regenerate results.md using effective settings
        let new_count = result_db.jobs.len() - before_count;
        let total_jobs = result_db.jobs.len();
        let total_runs = result_db.runs.len();
        let roles_md = effective.roles.iter().filter(|r| !r.trim().is_empty()).map(|r| format!("- {}", r.trim())).collect::<Vec<_>>().join("\n");
        let mut md = format!("# Hunt Results\n\nRun: {}\nMode: {}\nSources: {}\nTotal runs: {}\nTotal distinct jobs: {}\nLast refreshed: {}\n\n## Hunt Settings\n\n- Max scrape results: {}\n- Location: {}\n- Posted within: {}\n- Seniority: {}\n- Experience: {}\n- Minimum salary: {}\n- Include keywords: {}\n- Avoid keywords: {}\n\n### Roles\n\n{}\n\n",
            effective.name, effective.work_mode, effective.selected_sites.join(", "), total_runs, total_jobs, today,
            effective.max_items, format_hunt_setting_value("location", &effective.location), format_hunt_setting_value("postedWithin", &effective.posted_within), format_hunt_setting_value("seniority", &effective.seniority), format_hunt_setting_value("experience", &effective.experience), format_hunt_setting_value("salary", &effective.min_salary), format_hunt_setting_value("includeKeywords", &effective.include_keywords), format_hunt_setting_value("excludeKeywords", &effective.exclude_keywords),
            if roles_md.is_empty(){"- Not specified".to_string()}else{roles_md});
        if !failures.is_empty() { md.push_str("## Source Failures\n\n"); for f in &failures { md.push_str(&format!("- {f}\n")); } md.push('\n'); }
        for (ri, run) in result_db.runs.iter().enumerate() {
            let label = if ri == result_db.runs.len() - 1 && new_count > 0 { "Latest Run".to_string() } else { format!("Run {}", ri + 1) };
            let new_info = if ri == result_db.runs.len() - 1 && new_count > 0 { format!(" ({} new)", new_count) } else { String::new() };
            md.push_str(&format!("## {} --- {}{}\n\n", label, run.date, new_info));
            if !run.sources_failed.is_empty() {
                for sf in &run.sources_failed { md.push_str(&format!("- Source failed: {sf}\n")); }
            }
            md.push_str(&format!("- Jobs found: {}\n", run.new_jobs + run.filtered + run.duplicates));
            md.push_str(&format!("- New: {} . Filtered: {} . Duplicates: {}\n\n", run.new_jobs, run.filtered, run.duplicates));
        }
        md.push_str("## All Jobs\n\n");
        let mut all_job_entries: Vec<(&String, &HuntJobEntry)> = result_db.jobs.iter().collect();
        all_job_entries.sort_by(|(_, a), (_, b)| b.last_seen.cmp(&a.last_seen).then_with(|| a.title.cmp(&b.title)));
        for (i, (_, entry)) in all_job_entries.iter().enumerate() {
            let mut meta_parts = Vec::new();
            if !entry.source_name.is_empty() { meta_parts.push(format!("Source: {}", entry.source_name)); }
            meta_parts.push(format!("First seen: {}", entry.first_seen));
            let meta_str = meta_parts.join(" . ");
            md.push_str(&format!("{}. [{} --- {}]({})\n   - {}\n", i+1, entry.title, entry.company, entry.file_path, meta_str));
        }
        if let Err(e) = fs::write(&results_path, md) { emit_event(&on_event, &run_id, "failed", e.to_string()); return; }
        let summary = if new_count == 0 { "No new jobs found --- all up to date.".to_string() } else { format!("Hunt complete. Added {} new job{} to {}/", new_count, if new_count == 1 { "" } else { "s" }, job_dir_name) };
        let payload = serde_json::json!({
            "rawFound": raw_found,
            "newJobs": new_count,
            "filtered": filtered_total,
            "duplicates": already_seen + intra_duplicate,
            "totalDistinctJobs": total_jobs,
            "totalRuns": total_runs,
            "jobDirName": job_dir_name,
            "resultsPath": input.results_path,
            "sourceFailures": failures
        });
        emit_event_payload(&on_event, &run_id, "completed", summary, payload);
    });
    Ok(return_run_id)
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HuntJob {
    pub title: String, pub company: String, pub location: String,
    pub work_mode: String, pub seniority: String, pub experience: String,
    pub salary: String, pub posted_date: String,
    pub apply_url: String, pub source_url: String,
    pub source_name: String, pub actor_slug: String,
    pub description: String,
    pub requirements: Vec<String>, pub skills: Vec<String>,
}

fn alias(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(val) = v.get(*k) {
            if let Some(s) = val.as_str() { let t = s.trim(); if !t.is_empty() { return t.to_string(); } }
            else if let Some(n) = val.as_i64() { return n.to_string(); }
            else if let Some(n) = val.as_f64() { return format!("{:.0}", n); }
            else if let Some(b) = val.as_bool() { return if b { "Yes".into() } else { "No".into() }; }
            else if let Some(arr) = val.as_array() {
                let joined = arr.iter().filter_map(|x| {
                    if let Some(s)=x.as_str(){Some(s.trim().to_string())}
                    else if x.is_number() || x.is_boolean(){Some(x.to_string())}
                    else {None}
                }).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ");
                if !joined.is_empty() { return joined; }
            }
        }
    }
    String::new()
}
fn alias_alt(v: &Value, primary: &[&str], fallback: &[&str]) -> String {
    let r = alias(v, primary);
    if !r.is_empty() { return r; }
    alias(v, fallback)
}
fn alias_bool(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(default)
}
fn listify(v: &Value, keys: &[&str]) -> Vec<String> {
    for k in keys {
        if let Some(val) = v.get(*k) {
            if let Some(arr) = val.as_array() {
                let items: Vec<String> = arr.iter().filter_map(|x| x.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                if !items.is_empty() { return items; }
            }
            if let Some(s) = val.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() { return vec![trimmed.to_string()]; }
            }
        }
    }
    vec![]
}
fn concise(text: String, max_chars: usize) -> String {
    let cleaned = text.replace("\r", " ").replace("\t", " ").lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
    if cleaned.chars().count() <= max_chars { cleaned } else { format!("{}…", cleaned.chars().take(max_chars).collect::<String>()) }
}

fn normalize_54_career_sites(v: &Value) -> HuntJob {
    HuntJob {
        title: alias(v, &["title"]), company: alias(v, &["organization"]),
        location: alias_alt(v, &["locations_raw"], &["locations_derived","cities_derived","countries_derived"]),
        work_mode: alias(v, &["ai_work_arrangement","remote_derived"]),
        seniority: alias(v, &["ai_experience_level"]),
        experience: String::new(),
        salary: alias_alt(v, &["ai_salary_minvalue"], &["ai_salary_maxvalue","ai_salary_currency","ai_salary_unittext","salary_raw"]),
        posted_date: alias(v, &["date_posted","date_created"]),
        apply_url: alias(v, &["external_apply_url","url"]),
        source_url: alias(v, &["url","external_apply_url"]),
        source_name: "54 Career Sites".into(), actor_slug: "fantastic-jobs/career-site-job-listing-api".into(),
        description: concise(alias(v, &["description_text"]), 1800),
        requirements: listify(v, &["ai_requirements_summary"]),
        skills: listify(v, &["ai_key_skills"]),
    }
}
fn normalize_linkedin(v: &Value) -> HuntJob {
    let mut j = normalize_54_career_sites(v);
    j.source_name = "LinkedIn".into(); j.actor_slug = "fantastic-jobs/advanced-linkedin-job-search-api".into();
    j
}
fn normalize_indeed(v: &Value) -> HuntJob {
    let loc = if let Some(o) = v.get("location").and_then(|x| x.as_object()) {
        ["city", "state", "country", "countryCode"]
            .iter()
            .filter_map(|k| o.get(*k).and_then(|x| x.as_str()).map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    } else { alias(v, &["location", "formattedLocation", "jobLocation"]) };
    HuntJob {
        title: alias(v, &["title", "jobTitle", "positionName", "position"]), company: alias(v, &["company", "companyName", "source"]),
        location: loc,
        work_mode: alias(v, &["isRemote", "workingSystem", "remote", "workMode", "workArrangement", "jobType"]), seniority: alias(v, &["level", "jobLevel"]), experience: String::new(),
        salary: alias(v, &["salary", "salaryText", "salaryRange", "estimatedSalary"]),
        posted_date: alias(v, &["datePublished", "postedToday", "postedAt", "postedDate", "date", "publishedAt", "formattedRelativeTime"]),
        apply_url: alias(v, &["url", "jobUrl", "jobURL", "applyUrl", "apply_url"]),
        source_url: alias(v, &["url", "jobUrl", "jobURL", "applyUrl", "apply_url"]),
        source_name: "Indeed".into(), actor_slug: "borderline/indeed-scraper".into(),
        description: concise(alias(v, &["descriptionText", "descriptionHtml", "description", "jobDescription", "snippet", "text"]), 1800),
        requirements: vec![], skills: listify(v, &["skills"]),
    }
}
fn normalize_yc(v: &Value) -> HuntJob {
    HuntJob {
        title: alias(v, &["title"]), company: alias(v, &["companyName"]),
        location: alias(v, &["location"]),
        work_mode: String::new(),
        seniority: String::new(),
        experience: alias(v, &["experience"]),
        salary: alias_alt(v, &["salaryRange"], &["salaryMin","salaryMax","salaryCurrency"]),
        posted_date: alias(v, &["postedAgo", "datePosted"]),
        apply_url: alias(v, &["applyUrl"]),
        source_url: alias(v, &["companyUrl","applyUrl"]),
        source_name: "YC Startup Jobs".into(), actor_slug: "memo23/y-combinator-scraper".into(),
        description: concise(alias(v, &["description"]), 1800),
        requirements: vec![], skills: vec![],
    }
}
fn normalize_welcome_jungle(v: &Value) -> HuntJob {
    HuntJob {
        title: alias(v, &["title"]), company: alias(v, &["company"]),
        location: alias_alt(v, &["location"], &["country"]),
        work_mode: if alias_bool(v, "remote",false) || alias_bool(v, "has_remote", false) { "Remote".into() } else { String::new() },
        seniority: alias(v, &["experience_level_minimum"]), experience: alias(v, &["experience_level_minimum"]),
        salary: alias_alt(v, &["salary"], &["salary_yearly_minimum", "salary_minimum", "salary_maximum", "salary_currency", "salary_period"]),
        posted_date: alias(v, &["date_posted", "published_at_timestamp"]),
        apply_url: alias(v, &["url"]),
        source_url: alias(v, &["url"]),
        source_name: "Welcome to the Jungle".into(), actor_slug: "shahidirfan/jungle-job-scraper".into(),
        description: concise(alias(v, &["description","text"]), 1800),
        requirements: vec![], skills: vec![],
    }
}
fn normalize_hiring_cafe(v: &Value) -> HuntJob {
    HuntJob {
        title: alias_alt(v, &["job_information_title"], &["v5_processed_job_data_core_job_title","v5_processed_job_data_job_category"]),
        company: alias(v, &["v5_processed_job_data_company_name","enriched_company_data_name"]),
        location: alias(v, &["v5_processed_job_data_formatted_workplace_location","v5_processed_job_data_workplace_countries"]),
        work_mode: alias(v, &["v5_processed_job_data_workplace_type"]),
        seniority: alias(v, &["v5_processed_job_data_seniority_level"]),
        experience: alias(v, &["v5_processed_job_data_min_industry_and_role_yoe"]),
        salary: alias_alt(v, &["v5_processed_job_data_yearly_min_compensation","v5_processed_job_data_yearly_max_compensation"], &["v5_processed_job_data_listed_compensation_currency"]),
        posted_date: alias(v, &["v5_processed_job_data_estimated_publish_date"]),
        apply_url: alias(v, &["apply_url"]),
        source_url: alias(v, &["source","apply_url"]),
        source_name: "HiringCafe".into(), actor_slug: "memo23/apify-hiring-cafe-scraper".into(),
        description: concise(alias(v, &["job_information_description"]), 1800),
        requirements: listify(v, &["v5_processed_job_data_requirements_summary"]),
        skills: listify(v, &["v5_processed_job_data_technical_tools"]),
    }
}
fn normalize_himalayas(v: &Value) -> HuntJob {
    HuntJob {
        title: alias(v, &["title"]), company: alias(v, &["company_name"]),
        location: alias(v, &["location"]),
        work_mode: alias(v, &["work_mode"]),
        seniority: alias(v, &["experience_level"]),
        experience: String::new(),
        salary: alias_alt(v, &["salary_min"], &["salary_max","salary_currency","salary_period"]),
        posted_date: alias(v, &["posted_at"]),
        apply_url: alias(v, &["apply_url"]),
        source_url: alias(v, &["source_url","data_source_url"]),
        source_name: "Himalayas".into(), actor_slug: "inlifeprojects/himalayas-jobs-scraper".into(),
        description: concise(alias(v, &["description"]), 1800),
        requirements: vec![], skills: listify(v, &["tags"]),
    }
}
fn normalize_generic(v: &Value) -> HuntJob {
    HuntJob {
        title: alias(v, &["title","jobTitle","position","name"]),
        company: alias(v, &["company","companyName","employer","organization","company_name"]),
        location: alias(v, &["location","jobLocation","city","country","locations"]),
        work_mode: alias(v, &["remoteType","remote","workplaceType","work_mode"]),
        seniority: alias(v, &["seniority","experience_level","seniorityLevel"]),
        experience: alias(v, &["experience","yearsExperience"]),
        salary: alias(v, &["salary","salaryRange","compensation","salary_range","min_salary"]),
        posted_date: alias(v, &["postedAt","postedDate","datePosted","createdAt","publishedAt","posted_at","date_posted"]),
        apply_url: alias(v, &["applyUrl","applyURL","apply_url","url","jobUrl","jobURL","link"]),
        source_url: alias(v, &["sourceUrl","sourceURL","postingUrl","jobUrl","url","link"]),
        source_name: String::new(), actor_slug: String::new(),
        description: concise(alias(v, &["description","jobDescription","text","summary","details"]), 1800),
        requirements: vec![], skills: vec![],
    }
}
fn normalize_hunt_job(site: &str, v: &Value) -> HuntJob {
    let mut j = match site {
        "54 Career Sites" => normalize_54_career_sites(v),
        "LinkedIn" => normalize_linkedin(v),
        "Indeed" => normalize_indeed(v),
        "YC Startup Jobs" => normalize_yc(v),
        "Welcome to the Jungle" => normalize_welcome_jungle(v),
        "HiringCafe" => normalize_hiring_cafe(v),
        "Himalayas" => normalize_himalayas(v),
        _ => normalize_generic(v),
    };
    if j.source_name.is_empty() { j.source_name = site.to_string(); }
    j
}


fn filter_terms(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && !matches!(s.as_str(), "none"|"no"|"n/a"|"na"|"any"))
        .collect()
}
fn job_search_text(j: &HuntJob) -> String {
    format!("{} {} {} {} {} {} {} {}", j.title, j.company, j.location, j.description, j.requirements.join(" "), j.skills.join(" "), j.seniority, j.experience).to_lowercase()
}
fn contains_any_term(text: &str, terms: &[String]) -> bool { terms.iter().any(|t| !t.is_empty() && text.contains(t)) }
fn location_aliases(term: &str) -> Vec<String> {
    let t = term.trim().to_lowercase();
    match t.as_str() {
        "united kingdom" | "uk" | "great britain" => vec!["united kingdom","uk","great britain","england","scotland","wales","northern ireland","london"].into_iter().map(String::from).collect(),
        "new zealand" | "nz" => vec!["new zealand","nz","auckland","wellington"].into_iter().map(String::from).collect(),
        "united states" | "usa" | "us" => vec!["united states","usa","us","america"].into_iter().map(String::from).collect(),
        _ => vec![t]
    }
}
fn selected_location_terms(h: &HuntRunInput) -> Vec<String> {
    h.location.split(',')
        .flat_map(|s| location_aliases(s))
        .filter(|s| !s.is_empty() && s != "worldwide")
        .collect()
}
fn extract_numbers(text: &str) -> Vec<i64> {
    let mut nums=vec![]; let mut cur=String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() { cur.push(ch); }
        else if !cur.is_empty() { if let Ok(n)=cur.parse::<i64>() { nums.push(n); } cur.clear(); }
    }
    if !cur.is_empty() { if let Ok(n)=cur.parse::<i64>() { nums.push(n); } }
    nums
}
fn posted_days_limit(h: &HuntRunInput) -> Option<i64> {
    match h.posted_within.as_str() { "1 week" => Some(7), "3 weeks" => Some(21), "1 month" => Some(31), "3 months" => Some(93), _ => None }
}
fn relative_posted_within(s: &str, limit: i64) -> Option<bool> {
    let t=s.to_lowercase();
    if t.contains("today") || t.contains("just now") { return Some(true); }
    if t.contains("yesterday") { return Some(1 <= limit); }
    let nums=extract_numbers(&t); let n=*nums.first()?;
    if t.contains("day") { Some(n <= limit) }
    else if t.contains("week") { Some(n*7 <= limit) }
    else if t.contains("month") { Some(n*31 <= limit) }
    else { None }
}
fn post_filter_reason(h: &HuntRunInput, j: &HuntJob) -> Option<String> {
    if j.title.trim().is_empty() || j.company.trim().is_empty() { return Some("missing title/company".into()); }
    let include = filter_terms(&h.include_keywords);
    if !include.is_empty() && !contains_any_term(&job_search_text(j), &include) { return Some("missing include keyword".into()); }
    None
}


fn migrate_old_hunt_format(hunt_dir: &Path) -> Result<(), String> {
    let old_jobs_dir = hunt_dir.join("jobs");
    if !old_jobs_dir.exists() { return Ok(()); }
    let has_date_jobs = fs::read_dir(hunt_dir)
        .map_err(|e| e.to_string())?
        .any(|e| e.ok().map_or(false, |e| {
            e.file_name().to_string_lossy().starts_with("jobs-")
        }));
    if has_date_jobs { return Ok(()); }
    let date = if let Ok(config) = serde_json::from_str::<HuntConfig>(&fs::read_to_string(hunt_dir.join(".hunt_config.json")).unwrap_or_default()) {
        config.created.chars().take(10).collect::<String>()
    } else {
        let mtime = old_jobs_dir.metadata().ok().and_then(|m| m.modified().ok())
            .and_then(|t| Some(chrono::DateTime::<chrono::Utc>::from(t).format("%Y-%m-%d").to_string()))
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
        mtime
    };
    let new_name = format!("jobs-{date}");
    let new_jobs_dir = hunt_dir.join(&new_name);
    if new_jobs_dir.exists() {
        fs::remove_dir_all(&old_jobs_dir).map_err(|e| e.to_string())?;
        return Ok(());
    }
    fs::rename(&old_jobs_dir, &new_jobs_dir).map_err(|e| e.to_string())?;
    let result_db_path = hunt_dir.join(".hunt_result.json");
    let mut result_db: HuntResultDB = if result_db_path.exists() {
        serde_json::from_str(&fs::read_to_string(&result_db_path).unwrap_or_default()).unwrap_or(HuntResultDB { runs: vec![], jobs: HashMap::new() })
    } else {
        HuntResultDB { runs: vec![], jobs: HashMap::new() }
    };
    if result_db.jobs.is_empty() && new_jobs_dir.exists() {
        let mut entries: Vec<_> = fs::read_dir(&new_jobs_dir).map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in &entries {
            let path = entry.path();
            if let Ok(content) = fs::read_to_string(&path) {
                let title = content.lines().find(|l| l.starts_with("# ") && l.contains(" --- "))
                    .and_then(|l| l.splitn(2, ' ').last())
                    .and_then(|l| l.rsplitn(2, " --- ").last())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| entry.file_name().to_string_lossy().replace(".md", ""));
                let company = content.lines().find(|l| l.starts_with("- Company: "))
                    .and_then(|l| l.strip_prefix("- Company: "))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                let apply_url = content.lines().find(|l| l.starts_with("- Apply: "))
                    .and_then(|l| l.strip_prefix("- Apply: "))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                let source_name = content.lines().find(|l| l.starts_with("- Source: "))
                    .and_then(|l| l.strip_prefix("- Source: "))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                let file_path = format!("{new_name}/{}", entry.file_name().to_string_lossy());
                let key = format!("{}:{}:{}:{}", title.to_lowercase(), company.to_lowercase(), apply_url.to_lowercase(), source_name.to_lowercase());
                if !result_db.jobs.contains_key(&key) {
                    result_db.jobs.insert(key, HuntJobEntry {
                        title: title.clone(),
                        company: company.clone(),
                        apply_url: apply_url.clone(),
                        source_name: source_name.clone(),
                        first_seen: date.clone(),
                        last_seen: date.clone(),
                        file_path: file_path.clone(),
                    });
                }
            }
        }
        if result_db.runs.is_empty() {
            result_db.runs.push(HuntRunEntry {
                date: date.clone(),
                new_jobs: result_db.jobs.len(),
                filtered: 0,
                duplicates: 0,
                sources_failed: vec![],
                run_at: format!("{} 00:00:00", date),
            });
        }
    }
    if let Ok(json) = serde_json::to_string_pretty(&result_db) {
        let _ = fs::write(&result_db_path, json);
    }
    let config_path = hunt_dir.join(".hunt_config.json");
    if !config_path.exists() {
        let slug = hunt_dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let config = HuntConfig {
            name: slug.clone(),
            slug: slug.clone(),
            created: format!("{} 00:00", date),
            last_run: Some(date.clone()),
            roles: vec![],
            location: String::new(),
            work_mode: String::new(),
            seniority: String::new(),
            experience: String::new(),
            min_salary: String::new(),
            include_keywords: String::new(),
            exclude_keywords: String::new(),
            posted_within: String::new(),
            selected_sites: vec![],
            max_items: 100,
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&config_path, json);
        }
    }
    Ok(())
}

fn string_alias(v:&Value, keys:&[&str])->Option<String>{ for k in keys { if let Some(x)=v.get(*k) { if let Some(s)=x.as_str(){ if !s.trim().is_empty(){ return Some(s.trim().to_string()) } } else if !x.is_null() && (x.is_number() || x.is_boolean()) { return Some(x.to_string()) } } } None }
fn array_items(raw: Value)->Vec<Value>{ if let Some(a)=raw.as_array(){ return a.clone(); } for k in ["items","data","datasetItems","results"] { if let Some(a)=raw.get(k).and_then(|v|v.as_array()){ return a.clone(); } } vec![raw] }
fn dedupe_key(company:&str,title:&str,apply:&str,source:&str)->String{ format!("{}:{}:{}:{}",company.to_lowercase(),title.to_lowercase(),apply.to_lowercase(),source.to_lowercase()) }
fn normalize_text(s: &str) -> String {
    let trimmed = s.trim().to_lowercase();
    let mut res = String::with_capacity(trimmed.len());
    let mut in_space = false;
    for c in trimmed.chars() {
        if c.is_whitespace() {
            if !in_space { res.push(' '); in_space = true; }
        } else {
            res.push(c); in_space = false;
        }
    }
    let trimmed_res = res.trim().trim_end_matches(|c: char| !c.is_alphanumeric() && c != ' ').to_string();
    if trimmed_res.is_empty() { s.trim().to_lowercase() } else { trimmed_res }
}

fn normalize_url(url: &str) -> String {
    let url = url.trim().to_lowercase();
    if let Some(q_pos) = url.find('?') {
        let base = &url[..q_pos];
        let query = &url[q_pos+1..];
        let mut clean_params: Vec<&str> = Vec::new();
        for param in query.split('&') {
            let pl = param.to_lowercase();
            if pl.starts_with("utm_") || pl.starts_with("ref=") || pl.starts_with("source=") {
                continue;
            }
            clean_params.push(param);
        }
        if clean_params.is_empty() { base.to_string() }
        else { format!("{}?{}", base, clean_params.join("&")) }
    } else { url }
}

fn hunt_job_dedup_key(j: &HuntJob) -> String {
    let apply = j.apply_url.trim();
    if !apply.is_empty() { normalize_url(apply) }
    else { format!("{}:{}:{}",
        normalize_text(&j.title),
        normalize_text(&j.company),
        j.source_name.to_lowercase())
    }
}
fn normalize_job(v:&Value)->Result<JobRecord,String>{ let title=string_alias(v,&["title","jobTitle","position","name"]).ok_or("missing title")?; let company=string_alias(v,&["company","companyName","employer","organization"]).ok_or("missing company")?; let apply_url=string_alias(v,&["applyUrl","applyURL","url","jobUrl","jobURL","link"]).ok_or("missing applyUrl/sourceUrl")?; let source_url=string_alias(v,&["sourceUrl","sourceURL","postingUrl","jobUrl","url","link"]).unwrap_or_else(||apply_url.clone()); let remote=string_alias(v,&["remoteType","remote","workplaceType"]).unwrap_or_else(||"unknown".into()).to_lowercase(); let remote_type=if remote.contains("hybrid"){"hybrid"}else if remote.contains("remote")||remote=="true"{"remote"}else if remote.contains("onsite")||remote.contains("office"){"onsite"}else{"unknown"}.to_string(); let key=dedupe_key(&company,&title,&apply_url,&source_url); Ok(JobRecord{id:short_hash(&key),title,company,location:string_alias(v,&["location","jobLocation","city"]),remote_type,description:string_alias(v,&["description","jobDescription","text","summary"]),salary_range:string_alias(v,&["salary","salaryRange","compensation"]),apply_url,source_url,source_type:"apify".into(),dedupe_key:key,status:"new".into(),created_at:now()}) }
#[tauri::command]
fn list_jobs(project_slug:String,status:Option<String>)->Result<Vec<JobRecord>,String>{ let mut jobs:Vec<_>=load_db()?.jobs.into_iter().filter(|j|j.project_slug==project_slug).map(|j|j.job).filter(|j|status.as_ref().map(|s|s=="all"||&j.status==s).unwrap_or(true)).collect(); jobs.sort_by(|a,b|b.created_at.cmp(&a.created_at)); Ok(jobs) }
#[tauri::command]
fn update_job_status(input:JobStatusInput)->Result<JobRecord,String>{ let mut db=load_db()?; let mut out=None; for stored in &mut db.jobs{ if stored.project_slug==input.project_slug && stored.job.id==input.job_id{ stored.job.status=input.status.clone(); out=Some(stored.job.clone()); break; } } save_db(&db)?; out.ok_or("Job not found".into()) }

#[tauri::command]
fn generate_application_packet(input:PacketInput)->Result<ApplicationPacket,String>{ let mut db=load_db()?; if let Some(p)=db.packets.iter().find(|p|p.job_id==input.job_id).cloned(){return Ok(p)}; let stored=db.jobs.iter_mut().find(|j|j.project_slug==input.project_slug && j.job.id==input.job_id).ok_or("Job not found")?; let job=stored.job.clone(); let rel=format!("applications/{}-{}-{}",slugify(&job.company),slugify(&job.title),job.id); let root=project_root(&input.project_slug)?; let packet_root=root.join(&rel); for dir in ["input","tasks","templates","output"]{fs::create_dir_all(packet_root.join(dir)).map_err(|e|e.to_string())?;} write_if_missing(&packet_root.join("input/job_posting.json"),&serde_json::to_string_pretty(&job).unwrap())?; if root.join("profile/resume_extracted.md").exists(){ let _=fs::copy(root.join("profile/resume_extracted.md"),packet_root.join("input/resume_extracted.md")); } if root.join("profile/preferences.json").exists(){ let _=fs::copy(root.join("profile/preferences.json"),packet_root.join("input/user_preferences.json")); } write_if_missing(&packet_root.join("tasks/tailor_resume.md"),"# Tailor Resume\n\nUse `input/resume_extracted.md` and `input/job_posting.json`. Tailor truthfully; do not invent experience.\n")?; write_if_missing(&packet_root.join("tasks/verify_resume.md"),"# Verify Resume\n\nCheck tailored materials against the source resume and job posting. Flag unsupported claims.\n")?; write_if_missing(&packet_root.join("tasks/generate_outreach.md"),"# Generate Outreach\n\nDraft concise outreach grounded in the job posting and user profile.\n")?; write_if_missing(&packet_root.join("templates/resume_template.tex"),"% Resume template placeholder\n")?; stored.job.status="packet_created".into(); let packet=ApplicationPacket{id:short_hash(&format!("{}:{}",input.project_slug,input.job_id)),job_id:input.job_id,relative_path:rel,status:"ready".into(),created_at:now()}; db.packets.push(packet.clone()); save_db(&db)?; Ok(packet) }
#[tauri::command]
fn list_packets(project_slug:String)->Result<Vec<ApplicationPacket>,String>{ let job_ids:Vec<String>=load_db()?.jobs.into_iter().filter(|j|j.project_slug==project_slug).map(|j|j.job.id).collect(); Ok(load_db()?.packets.into_iter().filter(|p|job_ids.contains(&p.job_id)).collect()) }

fn default_chat_session(c: &Connection, project_slug: &str) -> Result<String, String> {
    let mut stmt = c.prepare("SELECT id FROM chat_sessions WHERE project_slug=?1 ORDER BY created_at LIMIT 1").map_err(|e| e.to_string())?;
    let existing: Result<String, _> = stmt.query_row(params![project_slug], |r| r.get(0));
    if let Ok(id) = existing { return Ok(id); }
    let id = short_hash(&format!("chat:{}", project_slug)); let t = now();
    c.execute("INSERT INTO chat_sessions(id,project_slug,title,created_at,updated_at) VALUES(?1,?2,'Project Chat',?3,?3)", params![id, project_slug, t]).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn list_chat_sessions(project_slug: String) -> Result<Vec<ChatSession>, String> {
    let c = conn()?; let _ = default_chat_session(&c, &project_slug)?;
    let mut stmt = c.prepare("SELECT id,project_slug,title,created_at,updated_at FROM chat_sessions WHERE project_slug=?1 ORDER BY updated_at DESC").map_err(|e| e.to_string())?;
    let sessions = stmt.query_map(params![project_slug], |r| Ok(ChatSession{id:r.get(0)?,project_slug:r.get(1)?,title:r.get(2)?,created_at:r.get(3)?,updated_at:r.get(4)?})).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    Ok(sessions)
}

#[tauri::command]
fn create_chat_session(input: CreateChatSessionInput) -> Result<ChatSession, String> {
    let c = conn()?; let t = now(); let title = input.title.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d-%H-%M").to_string());
    let session = ChatSession { id: uuid::Uuid::new_v4().to_string(), project_slug: input.project_slug, title, created_at: t.clone(), updated_at: t };
    c.execute("INSERT INTO chat_sessions(id,project_slug,title,created_at,updated_at) VALUES(?1,?2,?3,?4,?5)", params![session.id, session.project_slug, session.title, session.created_at, session.updated_at]).map_err(|e| e.to_string())?;
    Ok(session)
}

#[tauri::command]
fn delete_chat_session(input: DeleteChatSessionInput) -> Result<(), String> {
    let c = conn()?;
    let count: i64 = c.query_row("SELECT COUNT(*) FROM chat_sessions WHERE project_slug=?1", params![input.project_slug], |r| r.get(0)).map_err(|e| e.to_string())?;
    if count <= 1 { return Err("Keep at least one agent session".into()); }
    c.execute("DELETE FROM chat_messages WHERE session_id=?1", params![input.session_id]).map_err(|e| e.to_string())?;
    c.execute("DELETE FROM chat_sessions WHERE id=?1", params![input.session_id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn list_chat_messages(input: ListChatInput) -> Result<Vec<ChatMessage>, String> {
    let c = conn()?; let session_id = match input.session_id { Some(id) => id, None => default_chat_session(&c, &input.project_slug)? };
    let mut stmt = c.prepare("SELECT id,role,content,linked_file_path,linked_job_id,created_at FROM chat_messages WHERE session_id=?1 ORDER BY created_at").map_err(|e| e.to_string())?;
    let messages = stmt.query_map(params![session_id], |r| Ok(ChatMessage{id:r.get(0)?,role:r.get(1)?,content:r.get(2)?,linked_file_path:r.get(3)?,linked_job_id:r.get(4)?,created_at:r.get(5)?})).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    Ok(messages)
}

#[tauri::command]
fn save_chat_message(input: SaveChatInput) -> Result<ChatMessage, String> {
    if input.content.trim().is_empty() { return Err("Message cannot be empty".into()); }
    let c = conn()?; let session_id = match input.session_id.clone() { Some(id) => id, None => default_chat_session(&c, &input.project_slug)? }; let created_at = now();
    let msg = ChatMessage { id: uuid::Uuid::new_v4().to_string(), role: input.role, content: input.content, linked_file_path: input.linked_file_path, linked_job_id: input.linked_job_id, created_at };
    c.execute("INSERT INTO chat_messages(id,session_id,role,content,linked_file_path,linked_job_id,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![msg.id, session_id, msg.role, msg.content, msg.linked_file_path, msg.linked_job_id, msg.created_at]).map_err(|e| e.to_string())?;
    c.execute("UPDATE chat_sessions SET updated_at=?1 WHERE id=?2", params![msg.created_at, session_id]).map_err(|e| e.to_string())?;
    let transcript = project_root(&input.project_slug)?.join("chats/project-chat.md");
    if let Some(parent) = transcript.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    let line = format!("\n\n## {} · {}\n\n{}\n", msg.role, msg.created_at, msg.content);
    use std::io::Write; fs::OpenOptions::new().create(true).append(true).open(transcript).and_then(|mut f| f.write_all(line.as_bytes())).map_err(|e| e.to_string())?;
    Ok(msg)
}

#[tauri::command]
fn fork_chat_session(input: ForkChatSessionInput) -> Result<ChatSession, String> {
    let c = conn()?;
    let t = now();
    let source_title = c.query_row("SELECT title FROM chat_sessions WHERE id=?1", params![input.source_session_id], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
    let title = input.title.unwrap_or_else(|| format!("Fork: {}", source_title));
    let session = ChatSession { id: uuid::Uuid::new_v4().to_string(), project_slug: input.project_slug.clone(), title, created_at: t.clone(), updated_at: t.clone() };
    c.execute("INSERT INTO chat_sessions(id,project_slug,title,created_at,updated_at) VALUES(?1,?2,?3,?4,?5)", params![session.id, session.project_slug, session.title, session.created_at, session.updated_at]).map_err(|e| e.to_string())?;
    let mut stmt = c.prepare("SELECT id,role,content,linked_file_path,linked_job_id,created_at FROM chat_messages WHERE session_id=?1 AND created_at<=(SELECT created_at FROM chat_messages WHERE id=?2) ORDER BY created_at").map_err(|e| e.to_string())?;
    let messages = stmt.query_map(params![input.source_session_id, input.up_to_message_id], |r| Ok(ChatMessage{id:uuid::Uuid::new_v4().to_string(),role:r.get(1)?,content:r.get(2)?,linked_file_path:r.get(3)?,linked_job_id:r.get(4)?,created_at:r.get(5)?})).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    let mut latest_ts = t.clone();
    for msg in &messages {
        c.execute("INSERT INTO chat_messages(id,session_id,role,content,linked_file_path,linked_job_id,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![msg.id, session.id, msg.role, msg.content, msg.linked_file_path, msg.linked_job_id, msg.created_at]).map_err(|e| e.to_string())?;
        latest_ts = msg.created_at.clone();
    }
    c.execute("UPDATE chat_sessions SET updated_at=?1 WHERE id=?2", params![latest_ts, session.id]).map_err(|e| e.to_string())?;
    Ok(session)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameChatSessionInput { session_id: String, title: String }

#[tauri::command]
fn rename_chat_session(input: RenameChatSessionInput) -> Result<ChatSession, String> {
    let c = conn()?;
    c.execute("UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3", params![input.title, now(), input.session_id]).map_err(|e| e.to_string())?;
    let s = c.query_row("SELECT id, project_slug, title, created_at, updated_at FROM chat_sessions WHERE id = ?1", params![input.session_id], |row| {
        Ok(ChatSession { id: row.get(0)?, project_slug: row.get(1)?, title: row.get(2)?, created_at: row.get(3)?, updated_at: row.get(4)? })
    }).map_err(|e| e.to_string())?;
    Ok(s)
}

#[tauri::command]
fn create_task_file_from_chat(input: ChatTaskInput) -> Result<String, String> {
    if input.content.trim().is_empty() { return Err("Task content cannot be empty".into()); }
    let name = input.file_name.unwrap_or_else(|| format!("chat-task-{}.md", chrono::Utc::now().format("%Y%m%d-%H%M%S")));
    let safe = slugify(name.trim_end_matches(".md"));
    let rel = format!("chats/{}.md", if safe.is_empty() { "chat-task".into() } else { safe });
    let path = safe_project_path(&input.project_slug, &rel)?;
    write_if_missing(&path, &format!("# Chat Task\n\n{}\n", input.content))?;
    Ok(rel)
}

fn find_bin(name: &str) -> String {
    for dir in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        let p = format!("{}/{}", dir, name);
        if Path::new(&p).exists() { return p; }
    }
    name.into()
}

fn codex_bin() -> String { find_bin("codex") }
fn codex_connection_path() -> Result<PathBuf, String> { Ok(app_root()?.join("codex-connection.json")) }
fn codex_app_enabled() -> bool { codex_connection_path().ok().and_then(|p| fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str::<Value>(&s).ok()).and_then(|v| v["enabled"].as_bool()).unwrap_or(false) }
fn set_codex_app_enabled(enabled: bool) -> Result<(), String> { let p=codex_connection_path()?; if let Some(parent)=p.parent(){fs::create_dir_all(parent).map_err(|e| e.to_string())?;} fs::write(p, serde_json::json!({"enabled":enabled,"updatedAt":now()}).to_string()).map_err(|e| e.to_string()) }

#[tauri::command]
fn agent_respond(input: AgentRespondInput) -> Result<String, String> {
    let root = project_root(&input.project_slug)?;
    let mut prompt = format!("You are the local Drop the Grind job-search agent. Answer concisely and use the project workspace as context when relevant.\n\nUser message:\n{}", input.prompt);
    if let Some(p) = input.linked_file_path { prompt.push_str(&format!("\n\nCurrently selected file: {}", p)); }
    let out = Command::new(codex_bin())
        .env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin")
        .args(["exec", "--cd", root.to_str().ok_or("Invalid project path")?, "--skip-git-repo-check", "--sandbox", "workspace-write", "--"])
        .arg(prompt)
        .output()
        .map_err(|e| format!("Could not run Codex: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if !out.status.success() { return Err(format!("Codex failed: {}", if stderr.is_empty(){stdout}else{stderr})); }
    if stdout.is_empty() { return Err(if stderr.is_empty(){"Codex returned no response".into()}else{stderr}); }
    Ok(stdout)
}

fn tavily_extract_urls(urls: &[String]) -> Result<TavilyExtractOutput, String> {
    let key = read_tavily_key()?.ok_or("Tavily API key not configured")?;
    let body = serde_json::json!({
        "urls": urls,
        "extract_depth": "advanced",
        "include_images": false,
        "format": "text"
    });
    let result = execute_tavily_extract(&key, &body);
    let result = match result {
        Ok(v) => v,
        Err(e) => {
            // Retry once with minimal body if Tavily rejected the request shape
            if e.contains("422") || e.contains("400") || e.contains("unexpected") || e.contains("Invalid") {
                let minimal = serde_json::json!({"urls": urls});
                execute_tavily_extract(&key, &minimal)?
            } else {
                return Err(e);
            }
        }
    };
    Ok(result)
}

fn execute_tavily_extract(key: &str, body: &Value) -> Result<TavilyExtractOutput, String> {
    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let out = Command::new("curl")
        .args(["-sS", "--max-time", "30", "--connect-timeout", "10", "-X", "POST", "https://api.tavily.com/extract",
            "-H", &format!("Authorization: Bearer {key}"),
            "-H", "Content-Type: application/json",
            "-d", &body_str])
        .output().map_err(|e| format!("Tavily extract request failed: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let text = format!("{}{}", stdout, stderr);
    if !out.status.success() {
        return Err(format!("Tavily extract failed: {}", text.chars().take(300).collect::<String>()));
    }
    let v: Value = serde_json::from_str(&stdout).map_err(|e| format!("Tavily extract response parse: {e}"))?;
    // Tolerant parsing: accept multiple response schemas
    let mut results = Vec::new();
    let mut failed_results = Vec::new();
    // Check for results array (primary field)
    if let Some(arr) = v.get("results").and_then(|x| x.as_array()) {
        for item in arr {
            let url = item["url"].as_str().unwrap_or("").to_string();
            let raw_content = item.get("raw_content").and_then(|x| x.as_str()).map(|s| s.to_string());
            let content = item.get("content").and_then(|x| x.as_str()).map(|s| s.to_string());
            let text = item.get("text").and_then(|x| x.as_str()).map(|s| s.to_string());
            if url.is_empty() {
                // If a result lacks a URL, treat it as a potential failure
                let err = item["error"].as_str().unwrap_or("No URL in result").to_string();
                failed_results.push(TavilyExtractFailure { url: String::new(), error: err });
            } else {
                let title = item.get("title").and_then(|x| x.as_str()).map(|s| s.to_string());
                results.push(TavilyExtractResult { url, title, raw_content, content, text });
            }
        }
    }
    // Check for failed_results array
    if let Some(fails) = v.get("failed_results").and_then(|x| x.as_array()) {
        for item in fails {
            let url = item["url"].as_str().unwrap_or("").to_string();
            let error = item["error"].as_str().unwrap_or("Unknown failure").to_string();
            failed_results.push(TavilyExtractFailure { url, error });
        }
    }
    // If no recognized structure, return a clear error
    if results.is_empty() && failed_results.is_empty() {
        // Check if the response itself is an error object
        if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
            return Err(format!("Tavily Extract error: {err}"));
        }
        if let Some(detail) = v.get("detail").and_then(|x| x.as_str()) {
            return Err(format!("Tavily Extract error: {detail}"));
        }
        return Err("Tavily Extract response schema not recognized — expected results[] and/or failed_results[]".into());
    }
    Ok(TavilyExtractOutput { results, failed_results })
}

fn search_tavily(query: &str) -> Result<String, String> {
    let key = read_tavily_key()?.ok_or("Tavily API key not configured")?;
    let body = serde_json::json!({"query": query, "search_depth": "advanced", "max_results": 6}).to_string();
    let out = Command::new("curl")
        .args(["-sS", "-X", "POST", "https://api.tavily.com/search", "-H", &format!("Authorization: Bearer {key}"), "-H", "Content-Type: application/json", "-d", &body])
        .output().map_err(|e| format!("Tavily request failed: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if !out.status.success() { return Err(format!("Tavily search failed: {text}")); }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Tavily response parse: {e}"))?;
    let mut result = String::new();
    if let Some(answer) = v["answer"].as_str() { result.push_str(&format!("Answer: {answer}\n\n")); }
    if let Some(results) = v["results"].as_array() {
        for (i, r) in results.iter().enumerate() {
            let title = r["title"].as_str().unwrap_or("Untitled");
            let url = r["url"].as_str().unwrap_or("");
            let content = r["content"].as_str().unwrap_or("");
            result.push_str(&format!("{}. {title}\n   {url}\n   {content}\n\n", i + 1));
        }
    }
    if result.is_empty() { result = v["answer"].as_str().unwrap_or("No results found").into(); }
    Ok(result)
}

fn execute_tool(tool_name: &str, args: &Value, project_root: &std::path::Path, project_slug: &str) -> Result<String, String> {
    match tool_name {
        "search_web" | "web_search" | "tavily_search" => {
            let query = args["query"].as_str().or_else(|| args["q"].as_str()).ok_or("search_web requires a 'query' argument")?;
            search_tavily(query)
        }
        "read_file" | "read" => {
            let path_str = args["path"].as_str().or_else(|| args["file"].as_str()).ok_or("read_file requires a 'path' argument")?;
            let resolved = resolve_workspace_path(project_root, path_str)?;
            if !resolved.exists() { return Err(format!("File not found: {path_str}")); }
            fs::read_to_string(&resolved).map_err(|e| format!("Read error: {e}"))
        }
        "write_file" | "write" => {
            let path_str = args["path"].as_str().or_else(|| args["file"].as_str()).ok_or("write_file requires a 'path' argument")?;
            let content = args["content"].as_str().or_else(|| args["text"].as_str()).ok_or("write_file requires 'content'")?;
            let resolved = resolve_workspace_path(project_root, path_str)?;
            if let Some(parent) = resolved.parent() { fs::create_dir_all(parent).map_err(|e| format!("Create dir: {e}"))?; }
            fs::write(&resolved, content).map_err(|e| format!("Write error: {e}"))?;
            Ok(format!("Wrote {} bytes to {path_str}", content.len()))
        }
        "render_resume" | "render_resume_pdf" | "render_pdf" => {
            let path_str = args["path"].as_str().or_else(|| args["file"].as_str()).ok_or("render_resume requires a 'path' or 'file' argument")?;
            let trimmed = path_str.trim().trim_start_matches("./");
            if trimmed.is_empty() { return Err("render_resume path cannot be empty".into()); }
            if std::path::Path::new(trimmed).is_absolute() || trimmed.contains("..") {
                return Err("render_resume path must be workspace-relative and cannot contain ..".into());
            }
            if !trimmed.ends_with("/resume.md") || !trimmed.split('/').any(|part| part == "personalized-resume") {
                return Err("render_resume only accepts personalized-resume/.../resume.md files".into());
            }
            let resolved = resolve_workspace_path(project_root, trimmed)?;
            if !resolved.exists() { return Err(format!("resume.md not found: {trimmed}")); }
            let job_path = trimmed.strip_suffix("/resume.md").ok_or("render_resume path must end with /resume.md")?.to_string();
            let rendered = resume::render_resume(resume::ResumeInput { project_slug: project_slug.to_string(), job_path })?;
            Ok(format!("Rendered PDF: {}", rendered.pdf_path))
        }
        "run_command" | "shell" | "exec" => {
            let cmd_str = args["command"].as_str().or_else(|| args["cmd"].as_str()).ok_or("run_command requires 'command'")?;
            let cwd = args["cwd"].as_str().map(|s| s.to_string()).unwrap_or_else(|| project_root.to_string_lossy().to_string());
            // Whitelist check — exact basename match
            let parts: Vec<&str> = cmd_str.split_whitespace().collect();
            if parts.is_empty() { return Err("Empty command".into()); }
            let basename = std::path::Path::new(parts[0])
                .file_name().and_then(|s| s.to_str()).unwrap_or(parts[0]);
            let allowed = ["pandoc", "pdflatex", "xelatex", "lualatex", "python3", "node", "git", "grep", "wc", "cat", "head", "tail", "ls", "echo", "sed", "awk", "tr", "sort", "uniq", "find"];
            if !allowed.contains(&basename) { return Err(format!("Command not allowed: {basename}. Allowed: {}", allowed.join(", "))); }
            let output = std::process::Command::new("sh")
                .arg("-c").arg(cmd_str)
                .current_dir(&cwd)
                .output().map_err(|e| format!("Command error: {e}"))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let mut result = if stdout.is_empty() { String::new() } else { stdout };
            if !stderr.is_empty() { result.push_str(&format!("\n[stderr]\n{stderr}")); }
            if !output.status.success() { result.push_str(&format!("\n[exit code: {}]", output.status.code().unwrap_or(-1))); }
            Ok(if result.is_empty() { "Command completed with no output".into() } else { result })
        }
        _ => Err(format!("Unknown tool: {tool_name}"))
    }
}

fn resolve_workspace_path(project_root: &std::path::Path, path_str: &str) -> Result<std::path::PathBuf, String> {
    let cleaned = path_str.trim_start_matches('/').trim_start_matches("./");
    if cleaned.contains("..") { return Err("Path traversal not allowed".into()); }
    let resolved = project_root.join(cleaned);
    let canonical_root = project_root.canonicalize().map_err(|e| format!("Root resolve: {e}"))?;
    match resolved.canonicalize() {
        Ok(p) if p.starts_with(&canonical_root) => Ok(p),
        Ok(_) => Err("Path escapes project workspace".into()),
        Err(_) if resolved.parent().map_or(false, |p| p.starts_with(&canonical_root)) => Ok(resolved),
        Err(_) => Err(format!("Invalid path: {path_str}"))
    }
}

#[tauri::command]
fn start_agent_run(app: AppHandle, state: State<AgentRunState>, input: AgentRunInput, on_event: Channel<AgentRunEvent>) -> Result<String, String> {
    let root = project_root(&input.project_slug)?;
    let run_id = input.run_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut prompt = format!("You are Drop the Grind's local job-search agent inside a macOS app. Help the user move from hunt intent to scraped jobs, tailored application packets, resumes, outreach, and follow-up tasks. Be concise, concrete, and workspace-aware. When useful, inspect or reference files under this project workspace: profile/RESUME.md, profile/USER.md, profile/resume_current.*, resources/, hunt_run/<name>/results.md (master job index), hunt_run/<name>/jobs-*/ (read-only scraped listings), hunt_run/<name>/personalized-resume/ (tailored resume outputs), applications/, and visible user files. Prefer actionable next steps over generic chat. If the user asks for work on a file, mention what file you need or what you will create. Available local tools include read_file, write_file, run_command, search_web, and render_resume. Use render_resume only for personalized-resume/.../resume.md files.\n\n## Available commands\n{}\n\nUser message:\n{}", resume::skill_registry_prompt(), input.prompt);
    if let Some(p) = input.linked_file_path { prompt.push_str(&format!("\n\nCurrently selected file: {}", p)); }

    // Inject skill instructions if the user's message matches a skill keyword
    if let Some(skill) = resume::matching_skill(&input.prompt) {
        let instructions = resume::skill_instructions(skill.name).unwrap_or("");
        prompt = format!("{}\n\n---\n### {}\n{}\n\n---\nUser message:\n{}", prompt, skill.name, instructions, input.prompt);
    }
    let model = input.model.unwrap_or_else(|| "gpt-5.5".into());
    let effort = input.effort.unwrap_or_else(|| "low".into());
    let root_str = root.to_str().ok_or("Invalid project path")?.to_string();
    let project_slug_for_tools = input.project_slug.clone();
    let apify_token = read_apify_key().ok().flatten();
    let mut child_cmd = Command::new(codex_bin());
    child_cmd.env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin");
    if let Some(token) = apify_token { child_cmd.env("APIFY_TOKEN", token); }
    let mut child = child_cmd
        .arg("app-server")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start Codex app-server: {e}"))?;
    let pid = child.id();
    state.pids.lock().map_err(|_| "Agent state lock failed")?.insert(run_id.clone(), pid);
    let _ = on_event.send(AgentRunEvent{payload:None,run_id:run_id.clone(),kind:"started".into(),text:format!("Starting Codex app-server · pid {pid}")});
    if let Some(stderr) = child.stderr.take() {
        let event_err = on_event.clone(); let id = run_id.clone();
        thread::spawn(move || { for line in BufReader::new(stderr).lines().map_while(Result::ok) { let _ = event_err.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"stderr".into(),text:line}); } });
    }
    let mut stdin = child.stdin.take().ok_or("Codex stdin unavailable")?;
    let stdout = child.stdout.take().ok_or("Codex stdout unavailable")?;
    let event_stream = on_event.clone(); let id = run_id.clone();
    thread::spawn(move || {
        let send = |stdin: &mut std::process::ChildStdin, v: Value| -> Result<(), String> {
            writeln!(stdin, "{}", serde_json::to_string(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            stdin.flush().map_err(|e| e.to_string())
        };
        let init = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"drop-the-grind","version":"0.1.0"},"capabilities":{"experimentalApi":true}}});
        let thread_start = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"thread/start","params":{"cwd":root_str,"model":model,"approvalPolicy":"on-failure","sandbox":"workspace-write","ephemeral":true}});
        let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:"Waiting for app-server initialize response".into()});
        if let Err(e)=send(&mut stdin, init).and_then(|_| send(&mut stdin, thread_start)) { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"failed".into(),text:e}); return; }
        let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:"Waiting for thread/start response".into()});
        let mut thread_id = String::new();
        let (tx, rx) = mpsc::channel::<String>();
        thread::spawn(move || { for line in BufReader::new(stdout).lines().map_while(Result::ok) { if tx.send(line).is_err(){break;} } });
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"failed".into(),text:"Timed out waiting for Codex thread/start response".into()}); return; }
            let Ok(line)=rx.recv_timeout(remaining.min(Duration::from_millis(500))) else { continue; };
            let Ok(v): Result<Value,_> = serde_json::from_str(line.trim()) else { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:format!("Non-JSON app-server output: {}", line.chars().take(80).collect::<String>())}); continue; };
            if v["id"] == 1 { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:"App-server initialized".into()}); }
            if v["id"] == 2 { if let Some(err)=v["error"]["message"].as_str() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"failed".into(),text:err.into()}); return; } if let Some(t)=v["result"]["thread"]["id"].as_str() { thread_id=t.to_string(); break; } }
            if let Some(method)=v["method"].as_str() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:method.into()}); }
        }
        if thread_id.is_empty() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"failed".into(),text:"Codex app-server did not create a thread".into()}); return; }
        let turn = serde_json::json!({"jsonrpc":"2.0","id":3,"method":"turn/start","params":{"threadId":thread_id,"input":[{"type":"text","text":prompt}],"model":model,"effort":effort}});
        let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:format!("Thread created · starting turn · model {model} · thinking {effort}")});
        if let Err(e)=send(&mut stdin, turn) { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"failed".into(),text:e}); return; }
        let started_at = std::time::Instant::now();
        let mut last_activity = std::time::Instant::now();
        let max_total = Duration::from_secs(20 * 60);
        let max_idle = Duration::from_secs(5 * 60);
        loop {
            if started_at.elapsed() > max_total { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"failed".into(),text:"Timed out waiting for Codex turn to finish after 20 minutes".into()}); break; }
            if last_activity.elapsed() > max_idle { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"failed".into(),text:"Timed out waiting for Codex activity for 5 minutes".into()}); break; }
            let Ok(line)=rx.recv_timeout(Duration::from_millis(500)) else { continue; };
            last_activity = std::time::Instant::now();
            let Ok(v): Result<Value,_> = serde_json::from_str(line.trim()) else { continue; };
            if v["id"] == 3 { if let Some(err)=v["error"]["message"].as_str() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"failed".into(),text:err.into()}); return; } }
            let Some(method)=v["method"].as_str() else { continue; };
            let mut handled_tool = false;
            if method == "item/toolCall" || method == "item/toolUse" || method == "item/tool_call" || method == "item/tool_use" {
                let Some(item) = v["params"]["item"].as_object() else { continue; };
                let tool_call_id = item.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let tool_name = item.get("name").and_then(|x| x.as_str()).unwrap_or("unknown").to_string();
                let args = item.get("arguments").unwrap_or(&Value::Null);
                let target = args.get("path").or_else(|| args.get("file")).or_else(|| args.get("query")).or_else(|| args.get("command")).or_else(|| args.get("cmd")).and_then(|x| x.as_str()).map(|s| s.to_string()).unwrap_or_else(|| tool_name.clone());
                let _ = event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"tool-call".into(),text:serde_json::json!({"id":tool_call_id,"name":tool_name,"target":target,"status":"pending"}).to_string()});
                let result = execute_tool(&tool_name, args, std::path::Path::new(&root_str), &project_slug_for_tools);
                let status = if result.is_ok() { "done" } else { "error" };
                let preview = match &result { Ok(s) => s.chars().take(200).collect::<String>(), Err(e) => e.clone() };
                let _ = event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"tool-result".into(),text:serde_json::json!({"id":tool_call_id,"name":tool_name,"status":status,"preview":preview}).to_string()});
                let tool_result = serde_json::json!({"jsonrpc":"2.0","method":"tool/result","params":{"toolCallId":tool_call_id,"result":result.ok().unwrap_or_else(|| "Error".into())}});
                if let Err(e) = send(&mut stdin, tool_result) { let _ = event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"stderr".into(),text:format!("Failed to send tool result: {e}")}); }
                handled_tool = true;
            }
            if !handled_tool && method == "item/started" {
                let item_type = v["params"]["item"]["type"].as_str().unwrap_or("");
                let _ = event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:format!("Started {item_type}")});
                if item_type == "tool_use" || item_type == "tool_call" || item_type == "toolUse" || item_type == "toolCall" {
                    let item = &v["params"]["item"];
                    let tool_call_id = item["id"].as_str().unwrap_or("").to_string();
                    let tool_name = item["name"].as_str().unwrap_or("unknown").to_string();
                    let args = &item["arguments"];
                    let target = args.get("path").or_else(|| args.get("file")).or_else(|| args.get("query")).or_else(|| args.get("command")).or_else(|| args.get("cmd")).and_then(|x| x.as_str()).map(|s| s.to_string()).unwrap_or_else(|| tool_name.clone());
                    let _ = event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"tool-call".into(),text:serde_json::json!({"id":tool_call_id,"name":tool_name,"target":target,"status":"pending"}).to_string()});
                    let result = execute_tool(&tool_name, args, std::path::Path::new(&root_str), &project_slug_for_tools);
                    let status = if result.is_ok() { "done" } else { "error" };
                    let preview = match &result { Ok(s) => s.chars().take(200).collect::<String>(), Err(e) => e.clone() };
                    let _ = event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"tool-result".into(),text:serde_json::json!({"id":tool_call_id,"name":tool_name,"status":status,"preview":preview}).to_string()});
                    let tool_result = serde_json::json!({"jsonrpc":"2.0","method":"tool/result","params":{"toolCallId":tool_call_id,"result":result.ok().unwrap_or_else(|| "Error".into())}});
                    if let Err(e) = send(&mut stdin, tool_result) { let _ = event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"stderr".into(),text:format!("Failed to send tool result: {e}")}); }
                }
            }
            if handled_tool { continue; }
            match method {
                "item/agentMessage/delta" => if let Some(delta)=v["params"]["delta"].as_str() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"delta".into(),text:delta.into()}); },
                "item/started" => if let Some(t)=v["params"]["item"]["type"].as_str() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:format!("Started {t}")}); },
                "item/completed" => if let Some(t)=v["params"]["item"]["type"].as_str() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:format!("Completed {t}")}); },
                "turn/completed" => { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"completed".into(),text:"Done".into()}); break; },
                "thread/status/changed" => if let Some(t)=v["params"]["status"]["type"].as_str() { let _=event_stream.send(AgentRunEvent{payload:None,run_id:id.clone(),kind:"status".into(),text:format!("Thread {t}")}); },
                _ => {}
            }
        }
        let _ = child.kill();
    });
    Ok(run_id)
}

#[tauri::command]
fn cancel_agent_run(app: AppHandle, state: State<AgentRunState>, input: CancelAgentRunInput) -> Result<(), String> {
    if let Some(pid) = state.pids.lock().map_err(|_| "Agent state lock failed")?.remove(&input.run_id) {
        let _ = Command::new("/bin/kill").arg("-TERM").arg(pid.to_string()).output();
        let _ = app.emit("agent-run-event", AgentRunEvent{payload:None,run_id:input.run_id,kind:"cancelled".into(),text:"Cancelled".into()});
    }
    Ok(())
}

#[tauri::command]
fn codex_status() -> CodexStatus {
    let bin = codex_bin();
    if !codex_app_enabled() {
        let version = Command::new(&bin).env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin").arg("--version").output().ok().and_then(|o| if o.status.success(){Some(String::from_utf8_lossy(&o.stdout).trim().to_string())}else{None});
        return CodexStatus{installed:version.is_some(),connected:false,auth_mode:None,version,detail:"Not connected in Drop the Grind".into()};
    }
    let version = Command::new(&bin).env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin").arg("--version").output().ok().and_then(|o| if o.status.success(){Some(String::from_utf8_lossy(&o.stdout).trim().to_string())}else{None});
    let auth_path = dirs::home_dir().map(|h| h.join(".codex/auth.json"));
    let fallback = || {
        if let Some(path) = &auth_path {
            if let Ok(raw) = fs::read_to_string(path) {
                if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                    let mode = v["auth_mode"].as_str().map(|s| s.to_string());
                    let has_chatgpt = mode.as_deref() == Some("chatgpt") && v["tokens"]["access_token"].as_str().is_some();
                    if has_chatgpt { return Some(CodexStatus{installed:true,connected:true,auth_mode:mode,version:version.clone(),detail:"ChatGPT tokens found in ~/.codex/auth.json".into()}); }
                }
            }
        }
        None
    };
    if version.is_none() { return fallback().unwrap_or(CodexStatus{installed:false,connected:false,auth_mode:None,version:None,detail:format!("Codex CLI could not execute. Looked for {bin}. Bundled app may need PATH for node.")}); }
    let doctor = Command::new(&bin).env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin").args(["doctor", "--json"]).output();
    let Ok(out) = doctor else { return fallback().unwrap_or(CodexStatus{installed:true,connected:false,auth_mode:None,version,detail:"Could not run codex doctor".into()}); };
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let auth = &v["checks"]["auth.credentials"];
    let connected = auth["status"].as_str() == Some("ok");
    let auth_mode = auth["details"]["stored auth mode"].as_str().map(|s| s.to_string());
    let tokens = auth["details"]["stored ChatGPT tokens"].as_str().unwrap_or("unknown");
    if connected { CodexStatus{installed:true,connected,auth_mode,version,detail:format!("ChatGPT tokens: {tokens}")} } else { fallback().unwrap_or(CodexStatus{installed:true,connected:false,auth_mode,version,detail:"Codex auth not configured".into()}) }
}

fn codex_detect_auth() -> CodexStatus {
    let bin = codex_bin();
    let version = Command::new(&bin).env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin").arg("--version").output().ok().and_then(|o| if o.status.success(){Some(String::from_utf8_lossy(&o.stdout).trim().to_string())}else{None});
    let auth_path = dirs::home_dir().map(|h| h.join(".codex/auth.json"));
    let fallback = || {
        if let Some(path) = &auth_path {
            if let Ok(raw) = fs::read_to_string(path) {
                if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                    let mode = v["auth_mode"].as_str().map(|s| s.to_string());
                    let has_chatgpt = mode.as_deref() == Some("chatgpt") && v["tokens"]["access_token"].as_str().is_some();
                    if has_chatgpt { return Some(CodexStatus{installed:true,connected:true,auth_mode:mode,version:version.clone(),detail:format!("Found Codex CLI at {bin}. Using existing local ChatGPT Codex credentials from ~/.codex/auth.json")}); }
                }
            }
        }
        None
    };
    if version.is_none() { return fallback().unwrap_or(CodexStatus{installed:false,connected:false,auth_mode:None,version:None,detail:format!("Codex CLI could not execute. Looked for {bin}. Bundled app may need PATH for node.")}); }
    let doctor = Command::new(&bin).env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin").args(["doctor", "--json"]).output();
    let Ok(out) = doctor else { return fallback().unwrap_or(CodexStatus{installed:true,connected:false,auth_mode:None,version,detail:"Could not run codex doctor".into()}); };
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let auth = &v["checks"]["auth.credentials"];
    let connected = auth["status"].as_str() == Some("ok");
    let auth_mode = auth["details"]["stored auth mode"].as_str().map(|s| s.to_string());
    let tokens = auth["details"]["stored ChatGPT tokens"].as_str().unwrap_or("unknown");
    if connected { CodexStatus{installed:true,connected,auth_mode,version,detail:format!("Found Codex CLI at {bin}. Using existing local Codex credentials. ChatGPT tokens: {tokens}")} } else { fallback().unwrap_or(CodexStatus{installed:true,connected:false,auth_mode,version,detail:format!("Found Codex CLI at {bin}, but auth is not configured")}) }
}

#[tauri::command]
fn codex_connect() -> Result<CodexStatus, String> {
    let s = codex_detect_auth();
    if s.connected { set_codex_app_enabled(true)?; return Ok(s); }
    if !s.installed { return Ok(s); }
    Command::new(codex_bin()).env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin").arg("login").spawn().map_err(|e| e.to_string())?;
    Ok(CodexStatus{installed:true,connected:false,auth_mode:None,version:s.version,detail:"No existing Codex auth found. Started Codex login. Finish sign-in, then click Connect again.".into()})
}

#[tauri::command]
fn codex_disconnect() -> Result<CodexStatus, String> {
    set_codex_app_enabled(false)?;
    let version = Command::new(codex_bin()).env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin").arg("--version").output().ok().and_then(|o| if o.status.success(){Some(String::from_utf8_lossy(&o.stdout).trim().to_string())}else{None});
    Ok(CodexStatus{installed:version.is_some(),connected:false,auth_mode:None,version,detail:"Disconnected from Drop the Grind only. Did not modify ~/.codex/auth.json or global Codex login.".into()})
}

#[tauri::command]
fn test_apify_token(token: &str) -> Result<(), String> {
    let out = Command::new("curl")
        .args(["-sS", "-H", &format!("Authorization: Bearer {token}"), "https://api.apify.com/v2/users/me"])
        .output()
        .map_err(|e| format!("Could not reach Apify API: {e}"))?;
    if !out.status.success() { return Err(format!("Apify API check failed: {}", String::from_utf8_lossy(&out.stderr).trim())); }
    let text = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&text).map_err(|_| "Apify API returned a non-JSON response".to_string())?;
    if v["data"]["id"].as_str().is_some() { Ok(()) } else { Err(v["error"]["message"].as_str().unwrap_or("Apify token was rejected").to_string()) }
}

#[tauri::command]
fn apify_mcp_status() -> ApifyMcpStatus {
    match read_apify_key() {
        Ok(Some(_)) => ApifyMcpStatus{connected:true,detail:"Apify API token saved for Drop the Grind".into()},
        Ok(None) => ApifyMcpStatus{connected:false,detail:"Needs Apify API token".into()},
        Err(e) => ApifyMcpStatus{connected:false,detail:e},
    }
}

#[tauri::command]
fn apify_mcp_connect(input: ApifyConnectInput) -> Result<ApifyMcpStatus, String> {
    let token = input.token.trim();
    if token.is_empty() { return Err("Apify API token is required".into()); }
    if !token.starts_with("apify_api_") { return Err("Token should look like apify_api_...".into()); }
    test_apify_token(token)?;
    write_setting_key("apifyApiToken", Some(token))?;
    Ok(ApifyMcpStatus{connected:true,detail:"Apify API connected · token tested and saved locally".into()})
}

#[tauri::command]
fn apify_mcp_disconnect() -> Result<ApifyMcpStatus, String> {
    write_setting_key("apifyApiToken", None)?;
    Ok(ApifyMcpStatus{connected:false,detail:"Disconnected Apify API from Drop the Grind".into()})
}

fn read_setting_key(name: &str) -> Result<Option<String>, String> {
    let path = settings_path()?;
    if !path.exists() { return Ok(None); }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    Ok(v[name].as_str().map(|s| s.to_string()).filter(|s| !s.trim().is_empty()))
}
fn write_setting_key(name: &str, value: Option<&str>) -> Result<(), String> {
    fs::create_dir_all(app_root()?).map_err(|e| e.to_string())?;
    let path = settings_path()?;
    let mut settings = if path.exists() { fs::read_to_string(&path).ok().and_then(|r| serde_json::from_str::<Value>(&r).ok()).unwrap_or(serde_json::json!({})) } else { serde_json::json!({}) };
    if let Some(obj) = settings.as_object_mut() { match value { Some(v) => { obj.insert(name.into(), Value::String(v.into())); }, None => { obj.remove(name); } } }
    fs::write(path, serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn read_apify_key() -> Result<Option<String>, String> { read_setting_key("apifyApiToken") }
fn read_tavily_key() -> Result<Option<String>, String> { read_setting_key("tavilyApiKey") }

fn mask_key(key: &str) -> String {
    if key.len() <= 10 { return "••••".into(); }
    format!("{}••••{}", &key[..5], &key[key.len().saturating_sub(4)..])
}

#[tauri::command]
fn tavily_status() -> TavilyStatus {
    match read_tavily_key() {
        Ok(Some(key)) => TavilyStatus{connected:true,detail:"API key saved".into(),masked_key:Some(mask_key(&key))},
        Ok(None) => TavilyStatus{connected:false,detail:"Needs Tavily API key".into(),masked_key:None},
        Err(e) => TavilyStatus{connected:false,detail:e,masked_key:None},
    }
}

#[tauri::command]
fn tavily_connect(input: TavilyConnectInput) -> Result<TavilyStatus, String> {
    let key = input.api_key.trim();
    if key.is_empty() { return Err("Tavily API key is required".into()); }
    if !key.starts_with("tvly-") { return Err("Tavily API key should look like tvly-...".into()); }
    let body = serde_json::json!({"query":"Drop the Grind Tavily connection test","max_results":1}).to_string();
    let out = Command::new("curl")
        .args(["-sS", "-X", "POST", "https://api.tavily.com/search", "-H", &format!("Authorization: Bearer {key}"), "-H", "Content-Type: application/json", "-d", &body])
        .output().map_err(|e| format!("Could not run curl to test Tavily: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if !out.status.success() { return Err(format!("Tavily test failed: {text}")); }
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if v.get("error").is_some() || v.get("detail").is_some() { return Err(format!("Tavily rejected the key: {text}")); }
    write_setting_key("tavilyApiKey", Some(key))?;
    Ok(TavilyStatus{connected:true,detail:"Tavily search test passed".into(),masked_key:Some(mask_key(key))})
}

#[tauri::command]
fn tavily_extract(input: TavilyExtractInput) -> Result<TavilyExtractOutput, String> {
    if input.urls.is_empty() { return Err("At least one URL is required".into()); }
    for url in &input.urls {
        if !valid_import_url(url) { return Err(format!("Invalid URL: {url}")); }
    }
    tavily_extract_urls(&input.urls)
}

#[tauri::command]
fn tavily_disconnect() -> Result<TavilyStatus, String> {
    write_setting_key("tavilyApiKey", None)?;
    Ok(TavilyStatus{connected:false,detail:"Disconnected".into(),masked_key:None})
}

fn read_firecrawl_key() -> Result<Option<String>, String> { read_setting_key("firecrawlApiKey") }

#[tauri::command]
fn firecrawl_status() -> FirecrawlStatus {
    match read_firecrawl_key() {
        Ok(Some(key)) => FirecrawlStatus{connected:true,detail:"API key saved".into(),masked_key:Some(mask_key(&key))},
        Ok(None) => FirecrawlStatus{connected:false,detail:"Needs Firecrawl API key".into(),masked_key:None},
        Err(e) => FirecrawlStatus{connected:false,detail:e,masked_key:None},
    }
}

#[tauri::command]
fn firecrawl_connect(input: FirecrawlConnectInput) -> Result<FirecrawlStatus, String> {
    let key = input.api_key.trim();
    if key.is_empty() { return Err("Firecrawl API key is required".into()); }
    if !key.starts_with("fc-") { return Err("Firecrawl API key should look like fc-...".into()); }
    let body = serde_json::json!({"query":"Drop the Grind Firecrawl connection test","limit":1}).to_string();
    let out = Command::new("curl")
        .args(["-sS", "-X", "POST", "https://api.firecrawl.dev/v2/search",
            "-H", &format!("Authorization: Bearer {key}"),
            "-H", "Content-Type: application/json",
            "-d", &body])
        .output().map_err(|e| format!("Could not run curl to test Firecrawl: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if !out.status.success() { return Err(format!("Firecrawl test failed: {text}")); }
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if v.get("success").and_then(|x| x.as_bool()).unwrap_or(false) {
        write_setting_key("firecrawlApiKey", Some(key))?;
        Ok(FirecrawlStatus{connected:true,detail:"Firecrawl search test passed".into(),masked_key:Some(mask_key(key))})
    } else if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
        Err(format!("Firecrawl rejected the key: {err}"))
    } else {
        Err(format!("Firecrawl test failed: {}", text.chars().take(300).collect::<String>()))
    }
}

#[tauri::command]
fn firecrawl_disconnect() -> Result<FirecrawlStatus, String> {
    write_setting_key("firecrawlApiKey", None)?;
    Ok(FirecrawlStatus{connected:false,detail:"Disconnected".into(),masked_key:None})
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://console.apify.com/") || url.starts_with("https://apify.com/") || url.starts_with("https://app.tavily.com/") || url.starts_with("https://docs.tavily.com/") || url.starts_with("https://www.firecrawl.dev/") || url.starts_with("https://docs.firecrawl.dev/")) { return Err("URL not allowed".into()); }
    Command::new("open").arg(url).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(){ tauri::Builder::default().manage(AgentRunState::default()).plugin(tauri_plugin_opener::init()).invoke_handler(tauri::generate_handler![create_project,delete_project,list_projects,open_project,list_workspace_tree,read_text_file,read_binary_file,write_text_file,create_text_file,create_project_folder,rename_project_path,copy_project_path,copy_project_path_to,upload_project_file,upload_resume,remove_resume,delete_project_file,delete_project_path,reveal_project_path,open_project_file,save_source_config,get_source_config,generate_apify_files,create_hunt_run,start_hunt_apify,tavily_extract,import_job_links,list_hunt_profiles,save_hunt_config,list_jobs,update_job_status,generate_application_packet,list_packets,list_chat_sessions,create_chat_session,delete_chat_session,list_chat_messages,save_chat_message,fork_chat_session,rename_chat_session,create_task_file_from_chat,agent_respond,start_agent_run,cancel_agent_run,codex_status,codex_connect,codex_disconnect,apify_mcp_status,apify_mcp_connect,apify_mcp_disconnect,tavily_status,tavily_connect,tavily_disconnect,firecrawl_status,firecrawl_connect,firecrawl_disconnect,open_external_url,resume::validate_resume,resume::render_resume_pdf,resume::render_resume,resume::list_skills]).run(tauri::generate_context!()).expect("error while running Drop the Grind"); }

#[cfg(test)]
mod tests { #[test] fn slugify_project_names(){assert_eq!(super::slugify("My 2026 Job Search!"),"my-2026-job-search");} #[test] fn rejects_traversal_paths(){assert!(super::safe_project_path("demo","../secrets.txt").is_err());} #[test] fn rejects_invalid_project_slug(){assert!(super::project_root("../demo").is_err());} #[test] fn normalizes_aliases(){ let v=serde_json::json!({"jobTitle":"Engineer","companyName":"Acme","jobUrl":"https://x"}); let j=super::normalize_job(&v).unwrap(); assert_eq!(j.title,"Engineer"); assert_eq!(j.company,"Acme"); } #[test] fn dedupe_is_stable(){assert_eq!(super::dedupe_key("A","B","C","D"),super::dedupe_key("a","b","c","d"));} }
