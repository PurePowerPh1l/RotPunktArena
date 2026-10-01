/** Shared print helper for Tauri WebView2. */

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/**
 * Print through the system dialog without opening an in-app preview.
 *
 * Keep a real layout size for WebView2 (it ignores 0×0 frames), but place
 * the document off screen. Production CSP disallows inline scripts, so
 * printing is initiated from the parent in the UI click handler.
 */
export function openPrintHtml(html: string): void {
  document.getElementById("rpa-print-frame")?.remove();

  const iframe = document.createElement("iframe");
  iframe.id = "rpa-print-frame";
  iframe.setAttribute("title", "Druckdokument");
  iframe.setAttribute("aria-hidden", "true");
  iframe.tabIndex = -1;
  iframe.style.cssText =
    "position:fixed;left:-10000px;top:0;width:100vw;height:100vh;border:0;pointer-events:none;";
  document.body.appendChild(iframe);

  const win = iframe.contentWindow;
  const doc = iframe.contentDocument;
  if (!win || !doc) {
    iframe.remove();
    return;
  }

  let cleaned = false;
  let cleanupTimer: number | undefined;
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    window.clearTimeout(cleanupTimer);
    win.removeEventListener("afterprint", cleanup);
    iframe.remove();
  };

  win.addEventListener("afterprint", cleanup);
  // Fallback for WebView versions that do not dispatch afterprint.
  cleanupTimer = window.setTimeout(cleanup, 180_000);

  try {
    doc.open();
    doc.write(html);
    doc.close();
    win.print();
  } catch (error) {
    cleanup();
    console.error("Druckdialog konnte nicht geöffnet werden", error);
  }
}

export const PRINT_BASE_CSS = `
  @page { margin: 12mm; }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    font-family: "Segoe UI", "IBM Plex Sans", sans-serif;
    color: #1a1a18;
    background: #fff;
  }
  h1 { font-size: 1.35rem; margin: 0 0 0.2rem; }
  h2 { font-size: 1.05rem; margin: 1.25rem 0 0.5rem; }
  .meta { color: #555; font-size: 0.9rem; margin-bottom: 1rem; }
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  th, td { border-bottom: 1px solid #ddd; padding: 0.35rem 0.4rem; text-align: right; }
  th:first-child, td:first-child { text-align: left; }
  th { color: #666; font-weight: 600; }
  .foot { margin-top: 1.5rem; font-size: 0.75rem; color: #777; }
  @media print {
    .noprint { display: none !important; }
  }
`;
