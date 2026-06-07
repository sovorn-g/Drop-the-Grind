# Implementation Report – PDF Reopen Selection Bug

## Completed

Updated the `EditorView` PDF `useEffect` in `src/main.tsx` to clear `pdfOpenedRef.current` whenever the active selection is not a valid PDF selection. This ensures navigating away from a PDF to a non-PDF file and back re-opens the PDF in Preview.

The fix:
- **Before**: The `useEffect` guarded on `ext==='pdf' && project && selectedPath && pdfOpenedRef.current !== selectedPath`. When the user left a PDF, `pdfOpenedRef.current` stayed set to the old path, so coming back to the same PDF failed the `!==` check and nothing happened.
- **After**: The first early branch clears `pdfOpenedRef.current = ''` and returns when `!project || !selectedPath || ext !== 'pdf'`. This means any non-PDF selection resets the tracker. A second early guard (`pdfOpenedRef.current === selectedPath`) still prevents duplicate opens from React re-renders while the same PDF remains selected. The actual `open_project_file` call is unchanged.

## Files Changed

- `src/main.tsx` — Rewrote the PDF `useEffect` guard logic inside `EditorView` (lines 270–278).

## Verification

- `npm run build`: passed with no errors (1582 modules transformed, build completed in 934ms).
- Manual code review confirms all three success criteria from the plan:
  1. Selecting a PDF still opens it once in Preview (ref is cleared on non-PDF selection, so first PDF selection hits the open path).
  2. Navigating from PDF → .md → back to the same PDF opens it again (ref was cleared when the .md was selected).
  3. Re-rendering while the same PDF remains selected does not repeatedly call `open_project_file` (`=== selectedPath` guard returns early).

## Blockers

None.

## Observations

None.
