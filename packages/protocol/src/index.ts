/**
 * DISAG RedDot serial protocol parser.
 * See docs/protocol.md
 */

export const CONTROL = {
  STX: 0x02,
  ENQ: 0x05,
  ACK: 0x06,
  DC1: 0x11,
  NAK: 0x15,
  ETB: 0x17,
} as const;

export const SERIAL = {
  baudRate: 9600,
  dataBits: 8 as const,
  parity: "none" as const,
  stopBits: 1 as const,
};

/** DC1 command IDs (big-endian uint16 after DC1). */
export const DC1_CMD = {
  getVars: 4020,
  reset: 4022,
  init: 4023,
  discTypeOld: 1,
  discTypeNew: 4,
} as const;

export const SHOT_FRAME_LENGTH = 59;

export interface Shot {
  /** Raw integer from ASCII field (tenths encoded, e.g. 105 = 10.5). */
  valueRaw: number;
  /** Raw distance integer (tenths, e.g. 123 = 12.3). */
  distanceRaw: number;
  x: number;
  y: number;
  /** Header bytes [1..31] — unknown until hardware capture. */
  header: Uint8Array;
  /** Trailer bytes [55..58] — unknown until hardware capture. */
  trailer: Uint8Array;
  raw: Uint8Array;
}

export type Incoming =
  | { type: "nak" }
  | { type: "ack" }
  | { type: "shot"; shot: Shot }
  | { type: "need_more" }
  | { type: "skip" };

function asciiField(buf: Uint8Array, offset: number, length: number): string {
  return String.fromCharCode(...buf.subarray(offset, offset + length));
}

function parseDottedInt(field: string): number {
  return Number.parseInt(field.replace(/\./g, ""), 10);
}

/** Parse one 59-byte STX frame (including STX at index 0). */
export function parseShotFrame(frame: Uint8Array): Shot {
  if (frame.length < SHOT_FRAME_LENGTH) {
    throw new Error(`Shot frame too short: ${frame.length}`);
  }
  if (frame[0] !== CONTROL.STX) {
    throw new Error(`Expected STX, got 0x${frame[0]!.toString(16)}`);
  }

  const valueRaw = parseDottedInt(asciiField(frame, 32, 4));
  const distanceRaw = parseDottedInt(asciiField(frame, 37, 6));
  const x = Number.parseInt(asciiField(frame, 44, 5), 10);
  const y = Number.parseInt(asciiField(frame, 50, 5), 10);

  return {
    valueRaw,
    distanceRaw,
    x,
    y,
    header: frame.slice(1, 32),
    trailer: frame.slice(55, 59),
    raw: frame.slice(0, SHOT_FRAME_LENGTH),
  };
}

export function valueDisplay(valueRaw: number, tenths: boolean): number {
  const v = valueRaw / 10;
  return tenths ? v : Math.floor(v);
}

export function distanceDisplay(distanceRaw: number): number {
  return distanceRaw / 10;
}

/** Build [DC1, hi, lo] for a command id. */
export function encodeDc1(cmd: number): Uint8Array {
  return Uint8Array.of(CONTROL.DC1, (cmd >> 8) & 0xff, cmd & 0xff);
}

export function encodeEnq(): Uint8Array {
  return Uint8Array.of(CONTROL.ENQ);
}

export function encodeAck(): Uint8Array {
  return Uint8Array.of(CONTROL.ACK);
}

/**
 * Incremental byte-stream consumer for STX / NAK / ACK framing.
 */
export class RedDotStreamParser {
  private buffer: number[] = [];

  push(bytes: ArrayLike<number>): Incoming[] {
    for (let i = 0; i < bytes.length; i++) {
      this.buffer.push(bytes[i]!);
    }
    return this.drain();
  }

  private drain(): Incoming[] {
    const out: Incoming[] = [];
    while (this.buffer.length > 0) {
      const b = this.buffer[0]!;
      if (b === CONTROL.NAK) {
        this.buffer.shift();
        out.push({ type: "nak" });
        continue;
      }
      if (b === CONTROL.ACK) {
        this.buffer.shift();
        out.push({ type: "ack" });
        continue;
      }
      if (b === CONTROL.STX) {
        if (this.buffer.length < SHOT_FRAME_LENGTH) {
          out.push({ type: "need_more" });
          return out;
        }
        const frame = Uint8Array.from(this.buffer.splice(0, SHOT_FRAME_LENGTH));
        out.push({ type: "shot", shot: parseShotFrame(frame) });
        continue;
      }
      // Unknown leading byte — skip and continue
      this.buffer.shift();
      out.push({ type: "skip" });
    }
    return out;
  }
}

/** Synthetic frame helper for tests (unknown regions filled with 0x20 space / zeros).
 * Bytes 1–16 carry a unique nonce so identical aims don't collide on SHA-256 dedupe.
 */
export function buildSyntheticShotFrame(opts: {
  valueAscii: string; // length 4, e.g. "10.5"
  distanceAscii: string; // length 6, e.g. "012.30"
  xAscii: string; // length 5, e.g. "00123"
  yAscii: string; // length 5, e.g. "-0045"
}): Uint8Array {
  const frame = new Uint8Array(SHOT_FRAME_LENGTH);
  frame[0] = CONTROL.STX;
  frame.fill(0x20, 1, 32);
  const nonce = `${Date.now()}${Math.floor(Math.random() * 1e6)}`.slice(-16).padStart(16, "0");
  for (let i = 0; i < 16; i++) frame[1 + i] = nonce.charCodeAt(i);
  const write = (offset: number, s: string, len: number) => {
    if (s.length !== len) throw new Error(`Field length ${s.length} != ${len}`);
    for (let i = 0; i < len; i++) frame[offset + i] = s.charCodeAt(i);
  };
  write(32, opts.valueAscii, 4);
  write(37, opts.distanceAscii, 6);
  write(44, opts.xAscii, 5);
  write(50, opts.yAscii, 5);
  return frame;
}
