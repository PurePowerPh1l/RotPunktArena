import {
  CONTROL,
  RedDotStreamParser,
  SERIAL,
  SHOT_FRAME_LENGTH,
  buildSyntheticShotFrame,
  distanceDisplay,
  encodeAck,
  encodeEnq,
  type Incoming,
  type Shot,
  valueDisplay,
} from "@reddot/protocol";
import { bytesToHex, parseCaptureText, rxBytesFromCapture } from "./capture.ts";

function formatShot(shot: Shot): string {
  const v = valueDisplay(shot.valueRaw, true);
  const d = distanceDisplay(shot.distanceRaw);
  return `SHOT value=${shot.valueRaw} (${v}) distance=${shot.distanceRaw} (${d}) x=${shot.x} y=${shot.y}`;
}

export function formatIncoming(ev: Incoming): string | null {
  switch (ev.type) {
    case "nak":
      return "NAK (kein Schuss / keepalive)";
    case "ack":
      return "ACK";
    case "shot":
      return formatShot(ev.shot);
    case "need_more":
      return "… warte auf Rest des STX-Frames";
    case "skip":
      return "skip (unbekanntes Byte)";
    default:
      return null;
  }
}

export function replayBytes(bytes: Uint8Array): Incoming[] {
  const parser = new RedDotStreamParser();
  return parser.push(bytes).filter((e) => e.type !== "need_more");
}

export function replayCaptureFile(text: string): Incoming[] {
  const chunks = parseCaptureText(text);
  const rx = rxBytesFromCapture(chunks);
  return replayBytes(rx);
}

export function buildDemoFrame(): Uint8Array {
  return buildSyntheticShotFrame({
    valueAscii: "10.5",
    distanceAscii: "012.30",
    xAscii: "00123",
    yAscii: "-0045",
  });
}

export function printEvents(events: Incoming[]): void {
  for (const ev of events) {
    const line = formatIncoming(ev);
    if (line) console.log(line);
  }
}

export { CONTROL, SERIAL, SHOT_FRAME_LENGTH, encodeAck, encodeEnq, bytesToHex };
