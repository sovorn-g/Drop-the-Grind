# Plan – PDF Reopen Selection Bug

## What

- Fix the PDF auto-open guard so selecting a PDF opens it in Preview again after the user navigates away and returns to the same PDF.

## How

- Update the `EditorView` PDF `useEffect` in `src/main.tsx` to clear `pdfOpenedRef.current` whenever the active selection is not a valid PDF selection, then keep the existing path guard for the active PDF.
- **Scope**: In scope: frontend PDF selection/open behavior in `EditorView`. Out of scope: Rust `open_project_file`, macOS `open` behavior, the manual `Open again` button, file tree behavior, non-PDF previews.
- **Assumptions**: React Strict Mode duplicate effects should still be deduped while the same PDF remains the active selection.
- **Reuses**: `pdfOpenedRef` in `src/main.tsx`; existing `call('open_project_file', ...)` command path; existing extension parsing in `EditorView`.

## TODO

- Update `src/main.tsx` `EditorView` PDF `useEffect`: after computing `ext`, add an early branch for `!project`, empty `selectedPath`, or `ext !== 'pdf'` that sets `pdfOpenedRef.current = ''` and returns; then replace the current combined `if` with an explicit same-path guard and the existing `open_project_file` call. (uses: `pdfOpenedRef` and `call` from `src/main.tsx`)

## Outcome

- Selecting a PDF still opens it once in Preview.
- Navigating from a PDF to an `.md` or other non-PDF file and back to the same PDF opens it again.
- Re-rendering while the same PDF remains selected does not repeatedly call `open_project_file`.
- No backend or manual `Open again` behavior changes.
