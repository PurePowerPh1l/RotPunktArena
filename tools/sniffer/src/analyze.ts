/**
 * Offline analysis of STX frames for header/trailer mapping (Phase 0 hardware prep).
 */
import { parseShotFrame, SHOT_FRAME_LENGTH, CONTROL } from "@reddot/protocol";
import { bytesToHex, parseCaptureText, rxBytesFromCapture } from "./capture.ts";
import { RedDotStreamParser } from "@reddot/protocol";

export interface FrameRegionDump {
  index: number;
  valueRaw: number;
  distanceRaw: number;
  x: number;
  y: number;
  headerHex: string;
  trailerHex: string;
  headerAscii: string;
  trailerAscii: string;
  /** Per-byte dump of offsets 1..31 for notebook comparison across captures. */
  headerBytes: { offset: number; hex: string; ascii: string }[];
  trailerBytes: { offset: number; hex: string; ascii: string }[];
}

function asciiPreview(b: number): string {
  if (b >= 0x20 && b <= 0x7e) return String.fromCharCode(b);
  return ".";
}

function regionBytes(
  frame: Uint8Array,
  start: number,
  endExclusive: number,
): { offset: number; hex: string; ascii: string }[] {
  const out: { offset: number; hex: string; ascii: string }[] = [];
  for (let i = start; i < endExclusive; i++) {
    const b = frame[i]!;
    out.push({
      offset: i,
      hex: b.toString(16).padStart(2, "0").toUpperCase(),
      ascii: asciiPreview(b),
    });
  }
  return out;
}

/** Extract every complete STX shot frame from a capture document. */
export function analyzeCaptureText(text: string): FrameRegionDump[] {
  const chunks = parseCaptureText(text);
  const rx = rxBytesFromCapture(chunks);
  const parser = new RedDotStreamParser();
  const events = parser.push(rx);
  const dumps: FrameRegionDump[] = [];
  let index = 0;

  for (const ev of events) {
    if (ev.type !== "shot") continue;
    const frame = ev.shot.raw;
    if (frame.length < SHOT_FRAME_LENGTH || frame[0] !== CONTROL.STX) continue;
    const shot = parseShotFrame(frame);
    dumps.push({
      index: index++,
      valueRaw: shot.valueRaw,
      distanceRaw: shot.distanceRaw,
      x: shot.x,
      y: shot.y,
      headerHex: bytesToHex(shot.header),
      trailerHex: bytesToHex(shot.trailer),
      headerAscii: Array.from(shot.header, asciiPreview).join(""),
      trailerAscii: Array.from(shot.trailer, asciiPreview).join(""),
      headerBytes: regionBytes(frame, 1, 32),
      trailerBytes: regionBytes(frame, 55, 59),
    });
  }

  return dumps;
}

export function printFrameAnalysis(dumps: FrameRegionDump[]): void {
  if (dumps.length === 0) {
    console.log("Keine STX-Schussframes gefunden.");
    return;
  }

  for (const d of dumps) {
    console.log(`\n=== Frame #${d.index} value=${d.valueRaw} dist=${d.distanceRaw} x=${d.x} y=${d.y} ===`);
    console.log(`Header [1..31] hex: ${d.headerHex}`);
    console.log(`Header [1..31] ascii: "${d.headerAscii}"`);
    console.log("Header bytes:");
    for (const b of d.headerBytes) {
      console.log(`  [${String(b.offset).padStart(2, " ")}] ${b.hex} '${b.ascii}'`);
    }
    console.log(`Trailer [55..58] hex: ${d.trailerHex}`);
    console.log(`Trailer [55..58] ascii: "${d.trailerAscii}"`);
    console.log("Trailer bytes:");
    for (const b of d.trailerBytes) {
      console.log(`  [${String(b.offset).padStart(2, " ")}] ${b.hex} '${b.ascii}'`);
    }
  }

  // Cross-frame constancy hint for unknown regions
  if (dumps.length >= 2) {
    const sameHeader = dumps.every((d) => d.headerHex === dumps[0]!.headerHex);
    const sameTrailer = dumps.every((d) => d.trailerHex === dumps[0]!.trailerHex);
    console.log("\n— Vergleich über Frames —");
    console.log(
      `Header konstant: ${sameHeader ? "ja (vermutlich Geräte-/Session-Metadaten)" : "nein (pro Schuss unterschiedlich)"}`,
    );
    console.log(
      `Trailer konstant: ${sameTrailer ? "ja (Padding/Checksum-Kandidat)" : "nein"}`,
    );
  }
}
