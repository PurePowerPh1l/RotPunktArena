/**
 * Parse RedDot hex capture files (see docs/captures/README.md).
 */

export type CaptureDirection = "rx" | "tx" | "unknown";

export interface CaptureChunk {
  direction: CaptureDirection;
  bytes: Uint8Array;
  /** Optional comment / label from the line. */
  label?: string;
  line: number;
}

const HEX_BYTE = /^[0-9a-fA-F]{2}$/;

function parseDirection(token: string): CaptureDirection | null {
  const t = token.toLowerCase().replace(/:$/, "");
  if (t === "rx" || t === "<" || t === "in") return "rx";
  if (t === "tx" || t === ">" || t === "out") return "tx";
  return null;
}

/**
 * Parse a capture document into byte chunks.
 * Empty lines and `#` comments are ignored.
 * Lines may start with RX:/TX:/</> then whitespace-separated hex bytes.
 */
export function parseCaptureText(text: string): CaptureChunk[] {
  const chunks: CaptureChunk[] = [];
  const lines = text.split(/\r?\n/);

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i]!;
    const lineNo = i + 1;
    const trimmed = raw.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    const parts = trimmed.split(/\s+/);
    let direction: CaptureDirection = "unknown";
    let start = 0;

    const dir = parseDirection(parts[0]!);
    if (dir) {
      direction = dir;
      start = 1;
    }

    const hex: number[] = [];
    for (let p = start; p < parts.length; p++) {
      const tok = parts[p]!;
      if (tok.startsWith("#")) break;
      const cleaned = tok.replace(/[,:;]/g, "");
      if (!cleaned) continue;
      if (!HEX_BYTE.test(cleaned)) {
        throw new Error(
          `Invalid hex token "${tok}" on line ${lineNo} (expected two hex digits)`,
        );
      }
      hex.push(Number.parseInt(cleaned, 16));
    }

    if (hex.length === 0) continue;
    chunks.push({
      direction,
      bytes: Uint8Array.from(hex),
      line: lineNo,
    });
  }

  return chunks;
}

/** Concatenate all RX (and unknown) bytes for stream parsing; TX is ignored by default. */
export function rxBytesFromCapture(
  chunks: CaptureChunk[],
  opts: { includeUnknown?: boolean } = {},
): Uint8Array {
  const includeUnknown = opts.includeUnknown ?? true;
  const parts: number[] = [];
  for (const c of chunks) {
    if (c.direction === "tx") continue;
    if (c.direction === "unknown" && !includeUnknown) continue;
    for (const b of c.bytes) parts.push(b);
  }
  return Uint8Array.from(parts);
}

export function bytesToHex(bytes: ArrayLike<number>, sep = " "): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0").toUpperCase()).join(
    sep,
  );
}
