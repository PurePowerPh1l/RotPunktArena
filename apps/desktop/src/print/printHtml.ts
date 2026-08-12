/** Shared print helper for Tauri WebView2. */

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/**
 * Open a print preview and trigger the system print dialog.
 *
 * Must not rely on inline `<script>` / `onclick`: production CSP is
 * `script-src 'self'`, so those never run in the iframe. `window.print()`
 * is called from this parent function (same tick as the UI click) on an
 * iframe with a real layout size — WebView2 ignores 0×0 frames.
 */
export function openPrintHtml(html: string): void {
  document.getElementById("rpa-print-frame")?.remove();

  const iframe = document.createElement("iframe");
  iframe.id = "rpa-print-frame";
  iframe.setAttribute("title", "Druckvorschau");
  iframe.style.cssText =
    "position:fixed;inset:0;width:100%;height:100%;border:0;z-index:2147483647;background:#fff;";
  document.body.appendChild(iframe);

  const win = iframe.contentWindow;
  const doc = iframe.contentDocument;
  if (!win || !doc) {
    iframe.remove();
    return;
  }

  let cleaned = false;
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    window.removeEventListener("keydown", onEsc, true);
    iframe.remove();
  };

  const onEsc = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return;
    e.preventDefault();
    cleanup();
  };

  win.addEventListener("afterprint", cleanup);
  window.addEventListener("keydown", onEsc, true);

  doc.open();
  doc.write(html);
  doc.close();
  attachPrintChrome(doc, win, cleanup);

  try {
    win.focus();
    win.print();
  } catch {
    // Preview stays; user can print via the chrome bar or Esc.
  }

  window.setTimeout(cleanup, 180_000);
}

function attachPrintChrome(
  doc: Document,
  win: Window,
  onClose: () => void,
): void {
  const bar = doc.createElement("div");
  bar.className = "noprint";
  bar.style.cssText = "display:flex;gap:0.5rem;margin-bottom:1rem;";

  const printBtn = doc.createElement("button");
  printBtn.type = "button";
  printBtn.textContent = "Drucken";
  printBtn.style.cssText = "padding:0.5rem 0.9rem;";
  printBtn.addEventListener("click", () => {
    win.focus();
    win.print();
  });

  const closeBtn = doc.createElement("button");
  closeBtn.type = "button";
  closeBtn.textContent = "Schließen";
  closeBtn.style.cssText = "padding:0.5rem 0.9rem;";
  closeBtn.addEventListener("click", onClose);

  bar.append(printBtn, closeBtn);
  doc.body.insertBefore(bar, doc.body.firstChild);
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
