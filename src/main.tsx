import React from 'react';
import ReactDOM from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import { CheckCircle2, ChevronRight, FileText, Folder, FolderOpen, MessageSquare, Settings, Inbox, Package, Database, Search, Save, Plus, Circle, AlertCircle } from 'lucide-react';
import './styles.css';

type Project = { id: string; name: string; slug: string; rootPath: string; createdAt: string };
type FileTreeNode = { name: string; path: string; kind: 'directory' | 'file'; children?: FileTreeNode[] };
type TextFile = { content: string; version: string; readOnly: boolean };
type Mode = 'Setup' | 'Sources' | 'Job Inbox' | 'Packets' | 'Files' | 'Settings';

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
}

const modes: Array<{ label: Mode; icon: React.ReactNode }> = [
  { label: 'Setup', icon: <CheckCircle2 size={15} /> },
  { label: 'Sources', icon: <Database size={15} /> },
  { label: 'Job Inbox', icon: <Inbox size={15} /> },
  { label: 'Packets', icon: <Package size={15} /> },
  { label: 'Files', icon: <FileText size={15} /> },
  { label: 'Settings', icon: <Settings size={15} /> },
];

function TreeNode({ node, onOpen, active }: { node: FileTreeNode; onOpen: (node: FileTreeNode) => void; active?: string }) {
  const [open, setOpen] = React.useState(node.path === '');
  const isDir = node.kind === 'directory';
  return (
    <div>
      <button className={`tree-row ${active === node.path ? 'active' : ''}`} onClick={() => isDir ? setOpen(!open) : onOpen(node)}>
        {isDir ? (open ? <FolderOpen size={14}/> : <Folder size={14}/>) : <FileText size={14}/>}<span>{node.name === '.' ? 'Project' : node.name}</span>{isDir && <ChevronRight className={open ? 'rotate' : ''} size={13}/>} 
      </button>
      {isDir && open && node.children && <div className="tree-children">{node.children.map(child => <TreeNode key={child.path || child.name} node={child} onOpen={onOpen} active={active}/>)}</div>}
    </div>
  );
}

function App() {
  const [project, setProject] = React.useState<Project | null>(null);
  const [tree, setTree] = React.useState<FileTreeNode | null>(null);
  const [mode, setMode] = React.useState<Mode>('Setup');
  const [selectedPath, setSelectedPath] = React.useState<string>('');
  const [content, setContent] = React.useState('');
  const [readOnly, setReadOnly] = React.useState(false);
  const [dirty, setDirty] = React.useState(false);
  const [status, setStatus] = React.useState('No project open');
  const [projectName, setProjectName] = React.useState('My 2026 Job Search');

  const refreshTree = React.useCallback(async (slug = project?.slug) => {
    if (!slug) return;
    const next = await call<FileTreeNode>('list_workspace_tree', { projectSlug: slug });
    setTree(next);
  }, [project?.slug]);

  const createProject = async () => {
    try {
      setStatus('Creating project…');
      const p = await call<Project>('create_project', { name: projectName });
      setProject(p); setStatus(`Created ${p.name}`); await refreshTree(p.slug); setMode('Setup');
    } catch (e) { setStatus(String(e)); }
  };

  const openFile = async (node: FileTreeNode) => {
    if (!project || node.kind !== 'file') return;
    try {
      const file = await call<TextFile>('read_text_file', { projectSlug: project.slug, path: node.path });
      setSelectedPath(node.path); setContent(file.content); setReadOnly(file.readOnly); setDirty(false); setMode('Files');
      setStatus(file.readOnly ? `${node.name} is read-only` : `Opened ${node.name}`);
    } catch (e) { setStatus(String(e)); }
  };

  const saveFile = React.useCallback(async () => {
    if (!project || !selectedPath || readOnly) return;
    try {
      await call('write_text_file', { input: { projectSlug: project.slug, path: selectedPath, content } });
      setDirty(false); setStatus(`Saved ${selectedPath}`); await refreshTree();
    } catch (e) { setStatus(String(e)); }
  }, [project, selectedPath, readOnly, content, refreshTree]);

  React.useEffect(() => {
    const handler = (event: KeyboardEvent) => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') { event.preventDefault(); void saveFile(); } };
    window.addEventListener('keydown', handler); return () => window.removeEventListener('keydown', handler);
  }, [saveFile]);

  return <main className="app-frame">
    <div className="traffic"><span/><span/><span/></div>
    <aside className="sidebar">
      <div className="brand"><div className="brand-mark">D</div><div><strong>Drop the Grind</strong><small>Local job ops</small></div></div>
      <nav className="mode-nav">{modes.map(m => <button key={m.label} className={mode===m.label?'active':''} onClick={()=>setMode(m.label)}>{m.icon}<span>{m.label}</span></button>)}</nav>
      <div className="sidebar-section"><span>Project files</span>{tree ? <TreeNode node={tree} onOpen={openFile} active={selectedPath}/> : <p className="muted small">Create a project to reveal the workspace tree.</p>}</div>
    </aside>
    <section className="main-area">
      <header className="toolbar"><div><strong>{project?.name ?? 'No project'}</strong><span>{project ? project.rootPath : '~/.dropthegrind/workspace'}</span></div><div className="toolbar-actions"><span className="status-pill"><Circle size={8} fill="currentColor"/> Phase 1 shell</span><button className="ghost"><Settings size={15}/> Settings</button></div></header>
      <section className="content-grid"><div className="center-panel"><CenterView mode={mode} project={project} createProject={createProject} projectName={projectName} setProjectName={setProjectName} selectedPath={selectedPath} content={content} setContent={(v)=>{setContent(v); setDirty(true);}} readOnly={readOnly} dirty={dirty} saveFile={saveFile}/></div><ChatPanel selectedPath={selectedPath}/></section>
      <footer className="statusbar">{status}</footer>
    </section>
  </main>;
}

function CenterView(props: { mode: Mode; project: Project|null; createProject:()=>void; projectName:string; setProjectName:(v:string)=>void; selectedPath:string; content:string; setContent:(v:string)=>void; readOnly:boolean; dirty:boolean; saveFile:()=>void }) {
  if (!props.project) return <div className="empty-state"><div className="orb"/><h1>Create your job-search workspace</h1><p>Drop the Grind stores your resume, sources, jobs, packets, and chat locally under <code>~/.dropthegrind/workspace</code>.</p><div className="create-card"><input value={props.projectName} onChange={e=>props.setProjectName(e.target.value)} /><button className="primary" onClick={props.createProject}><Plus size={15}/> Create Project</button></div><p className="trust">Local-first · Human-reviewed · No auto-apply</p></div>;
  if (props.mode === 'Setup') return <SetupView/>;
  if (props.mode === 'Files') return <EditorView {...props}/>;
  return <PlaceholderView mode={props.mode}/>;
}
function SetupView(){ const items=['Add resume_original.pdf','Add or edit resume_extracted.md','Review preferences.json','Add Apify actor/source','Generate MCP task/config','Import Apify output JSON','Review jobs','Generate first application packet']; return <div className="view"><div className="view-title"><span className="dot green"/><div><h2>Project Setup</h2><p>Follow this local checklist to prepare the workspace for Phase 2.</p></div></div><div className="checklist">{items.map((it,i)=><div className="check-item" key={it}><span>{i<3?<CheckCircle2 size={16}/>:<Circle size={16}/>}</span><div><strong>{it}</strong><small>{i<3?'Created as editable workspace artifacts':'Coming in Phase 2'}</small></div></div>)}</div></div> }
function PlaceholderView({mode}:{mode:Mode}){ return <div className="view"><div className="view-title"><span className="dot blue"/><div><h2>{mode}</h2><p>{mode==='Sources'?'Apify MCP configuration and JSON import arrive in Phase 2.':mode==='Job Inbox'?'Imported jobs will be reviewed here after Phase 2.':mode==='Packets'?'Generated application packets will appear here after Phase 2.':'Workspace, Apify, and future provider settings.'}</p></div></div><div className="panel-card"><AlertCircle size={18}/><p>This view is intentionally scaffolded in Phase 1 so the app shell and navigation are ready without faking unfinished workflow behavior.</p></div></div> }
function EditorView({selectedPath,content,setContent,readOnly,dirty,saveFile}: any){ return <div className="editor-shell"><div className="editor-top"><div><span className="breadcrumb">{selectedPath || 'No file selected'}</span>{dirty && <span className="dirty">Unsaved</span>}</div><button disabled={!selectedPath||readOnly} onClick={saveFile}><Save size={14}/> Save</button></div>{!selectedPath?<div className="empty-editor">Select a file from the project tree.</div>:readOnly?<div className="readonly"><FileText size={24}/><h3>Read-only file</h3><p>Preview/editing is not supported for this file type yet. Reveal in Finder instead.</p></div>:<textarea className="editor" value={content} onChange={e=>setContent(e.target.value)} spellCheck={false}/>}</div> }
function ChatPanel({selectedPath}:{selectedPath:string}){ return <aside className="chat"><div className="chat-head"><MessageSquare size={15}/><strong>Project Chat</strong></div><div className="chat-body"><div className="message system"><p>Draft instructions, notes, and task prompts for this project.</p><small>Model execution is not connected yet.</small></div>{selectedPath && <div className="context-chip">Current file · {selectedPath}</div>}</div><div className="chat-compose"><textarea placeholder="Write local notes or task draft…"/><button>Save message</button></div></aside> }
ReactDOM.createRoot(document.getElementById('root')!).render(<App />);
