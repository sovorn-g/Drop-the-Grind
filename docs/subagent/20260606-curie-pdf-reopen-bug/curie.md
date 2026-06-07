## Scope Investigated

- PDF click-to-open flow end-to-end: frontend file tree → `openFile` → `EditorView` effect → Rust `open_project_file` command → macOS `open`
- The `pdfOpenedRef` guard in `EditorView` that prevents re-opening
- What I did not investigate: macOS `open` command behavior, Preview.app lifecycle, or Tauri event loop specifics

## Findings

### Root cause: `pdfOpenedRef` blocks re-open on same PDF

**`EditorView` in `src/main.tsx#L265-L286`**

```javascript
const pdfOpenedRef=React.useRef('');
React.useEffect(()=>{
    const ext=(selectedPath.split('.').pop()||'').toLowerCase();
    if(ext==='pdf'&&project&&selectedPath&&pdfOpenedRef.current!==selectedPath){
        pdfOpenedRef.current=selectedPath;
        call('open_project_file',{input:{projectSlug:project.slug,path:selectedPath}}).catch(()=>{});
    }
},[project,selectedPath]);
```

- **Finding**: The `pdfOpenedRef` is set to the PDF's path on first open, then never cleared. When the user navigates away to an `.md` file and back to the *same* PDF, `pdfOpenedRef.current` still equals `selectedPath`, so the guard `pdfOpenedRef.current !== selectedPath` evaluates to `false` and the `open_project_file` call is skipped.
- **Relevance**: This is the exact bug described. The ref's intended purpose was to prevent duplicate `open` calls within the same render/mount cycle (e.g., React Strict Mode double-fire), but it also blocks legitimate re-opens.

### Full click-to-open flow

1. **`TreeNode` in `src/main.tsx#L120`** — click handler calls `onOpen(node)`, which is `openFile`.
2. **`openFile` in `src/main.tsx#L158`** — calls `read_text_file` Rust command, sets `selectedPath` to `node.path`.
3. **`EditorView` effect** (above) — fires on `selectedPath` change. If extension is `pdf`, calls `open_project_file`.
4. **`open_project_file` in `src-tauri/src/lib.rs#L550`** — runs `Command::new("open").arg(path).spawn()`.
5. macOS `open` launches Preview.app for the PDF.

### `"Open again"` button works correctly

**`EditorView` render in `src/main.tsx#L278`**

```jsx
<button onClick={()=>call('open_project_file',{input:{projectSlug:project.slug,path:selectedPath}})}>
  <ExternalLink size={13}/> Open again
</button>
```

The manual "Open again" button bypasses the ref guard and always works. This confirms the Rust command is fine and the issue is purely the frontend guard.

### No alternative PDF-opening paths

The only code path that auto-opens PDFs on selection is the `EditorView` effect above. There are no event listeners, IPC channels, or alternate components that trigger PDF opens.

## Relationships

- `EditorView` (line 265) depends on `selectedPath` prop set by `openFile` (line 158) in `App`.
- `openFile` is passed as `onOpen` to every `TreeNode` (line 120).
- `open_project_file` Rust command (line 550) is the sole backend pathway.
- The `pdfOpenedRef` guard is the only thing between "PDF selected" and "Preview opens."

## Open Questions / Gaps

- None. The bug is fully identified.

## Start Here

- `src/main.tsx` — `EditorView` function at line 265. The fix is in the `useEffect` starting at line 275: either clear `pdfOpenedRef.current` when `ext !== 'pdf'`, or replace the ref-based guard with a different dedup strategy (e.g., track a navigation sequence ID instead of the path).
