/**
 * Optional live COM sniffing (9600 8N1). Graceful when serialport or port missing.
 * Writes a docs/captures-compatible .hex transcript when `outPath` is set.
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { RedDotStreamParser, SERIAL, encodeEnq, type Incoming } from "@reddot/protocol";
import { formatIncoming } from "./replay.ts";
import { bytesToHex } from "./capture.ts";

export interface PortSessionOptions {
  path: string;
  pollMs?: number;
  durationMs?: number;
  ackShots?: boolean;
  /** If set, write a capture `.hex` file (TX:/RX: lines) at end of session. */
  outPath?: string;
}

async function loadSerialPort(): Promise<typeof import("serialport") | null> {
  try {
    return await import("serialport");
  } catch (err) {
    console.error(
      "serialport-Modul nicht ladbar — Live-COM deaktiviert.",
      err instanceof Error ? err.message : err,
    );
    return null;
  }
}

export async function listSerialPorts(): Promise<
  { path: string; manufacturer?: string; friendlyName?: string }[]
> {
  const sp = await loadSerialPort();
  if (!sp) return [];
  try {
    const ports = await sp.SerialPort.list();
    return ports.map((p) => ({
      path: p.path,
      manufacturer: p.manufacturer,
      friendlyName: p.friendlyName,
    }));
  } catch (err) {
    console.error(
      "COM-Ports konnten nicht gelistet werden:",
      err instanceof Error ? err.message : err,
    );
    return [];
  }
}

function stampIso(): string {
  return new Date().toISOString();
}

export async function openAndSniff(opts: PortSessionOptions): Promise<number> {
  const sp = await loadSerialPort();
  if (!sp) {
    console.error("Abbruch: kein serialport verfügbar.");
    return 2;
  }

  const pollMs = opts.pollMs ?? 300;
  const durationMs = opts.durationMs ?? 10_000;
  const ackShots = opts.ackShots ?? true;
  const lines: string[] = [];

  const pushMeta = (s: string) => {
    lines.push(`# ${s}`);
  };
  const pushTx = (bytes: ArrayLike<number>, note?: string) => {
    const hex = bytesToHex(bytes);
    lines.push(note ? `TX: ${hex}  # ${note}` : `TX: ${hex}`);
  };
  const pushRx = (bytes: ArrayLike<number>, note?: string) => {
    const hex = bytesToHex(bytes);
    lines.push(note ? `RX: ${hex}  # ${note}` : `RX: ${hex}`);
  };

  pushMeta(`live capture started ${stampIso()}`);
  pushMeta(`port=${opts.path} baud=${SERIAL.baudRate} pollMs=${pollMs} durationMs=${durationMs}`);

  let port: InstanceType<typeof sp.SerialPort>;
  try {
    port = new sp.SerialPort({
      path: opts.path,
      baudRate: SERIAL.baudRate,
      dataBits: SERIAL.dataBits,
      parity: SERIAL.parity,
      stopBits: SERIAL.stopBits,
      autoOpen: false,
    });
  } catch (err) {
    console.error(
      `COM-Port ${opts.path} konnte nicht konfiguriert werden:`,
      err instanceof Error ? err.message : err,
    );
    return 2;
  }

  const openResult = await new Promise<Error | null>((resolve) => {
    port.open((err) => resolve(err ?? null));
  });
  if (openResult) {
    console.error(
      `COM-Port ${opts.path} nicht öffnenbar (Hardware fehlt oder Port belegt): ${openResult.message}`,
    );
    console.error(
      "Tipp: Capture-Dateien unter docs/captures/ mit `replay` / `analyze` auswerten — Live-Validierung wartet auf Hardware.",
    );
    return 2;
  }

  console.log(
    `Geöffnet: ${opts.path} @ ${SERIAL.baudRate} 8N1 — poll ${pollMs}ms, Dauer ${durationMs}ms`,
  );
  if (opts.outPath) {
    console.log(`Capture-Datei: ${opts.outPath}`);
  }

  let shotCount = 0;
  let nakCount = 0;
  const parser = new RedDotStreamParser();
  const onData = (buf: Buffer) => {
    console.log(`RX ${bytesToHex(buf)}`);
    pushRx(buf);
    const events: Incoming[] = parser.push(buf);
    for (const ev of events) {
      const line = formatIncoming(ev);
      if (line) console.log(`  → ${line}`);
      if (ev.type === "nak") nakCount += 1;
      if (ev.type === "shot") {
        shotCount += 1;
        if (ackShots) {
          port.write(Buffer.from(Uint8Array.of(0x06)));
          console.log("TX 06 (ACK)");
          pushTx([0x06], "ACK");
        }
      }
    }
  };
  port.on("data", onData);

  const enq = Buffer.from(encodeEnq());
  const poll = setInterval(() => {
    if (port.isOpen) {
      port.write(enq);
      console.log("TX 05 (ENQ)");
      pushTx([0x05], "ENQ");
    }
  }, pollMs);

  await new Promise<void>((resolve) => setTimeout(resolve, durationMs));
  clearInterval(poll);
  port.off("data", onData);

  await new Promise<void>((resolve) => {
    if (!port.isOpen) {
      resolve();
      return;
    }
    port.close(() => resolve());
  });
  console.log("Port geschlossen.");

  pushMeta(`ended ${stampIso()} shots=${shotCount} naks=${nakCount}`);
  if (opts.outPath) {
    try {
      mkdirSync(dirname(opts.outPath), { recursive: true });
      writeFileSync(opts.outPath, `${lines.join("\n")}\n`, "utf8");
      console.log(`Capture geschrieben: ${opts.outPath} (${shotCount} Schuss, ${nakCount} NAK)`);
    } catch (err) {
      console.error(
        "Capture-Datei konnte nicht geschrieben werden:",
        err instanceof Error ? err.message : err,
      );
      return 1;
    }
  }

  return 0;
}
