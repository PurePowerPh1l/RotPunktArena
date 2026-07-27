/** Shared print iframe helper for Tauri WebView2. */

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export function openPrintHtml(html: string): void {
  const iframe = document.createElement("iframe");
  iframe.setAttribute("aria-hidden", "true");
  iframe.style.cssText =
    "position:fixed;right:0;bottom:0;width:0;height:0;border:0;opacity:0;pointer-events:none";
  document.body.appendChild(iframe);
  const doc = iframe.contentDocument;
  if (!doc) {
    iframe.remove();
    const w = window.open("", "_blank");
    if (!w) return;
    w.document.write(html);
    w.document.close();
    return;
  }
  doc.open();
  doc.write(html);
  doc.close();
  const cleanup = () => {
    setTimeout(() => iframe.remove(), 800);
  };
  iframe.contentWindow?.addEventListener("afterprint", cleanup);
  setTimeout(cleanup, 60_000);
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
