#!/usr/bin/env node
/**
 * RedDot protocol sniffer / replay CLI.
 *
 * Usage:
 *   npm run sniffer -- replay <file.hex>
 *   npm run sniffer -- analyze <file.hex>
 *   npm run sniffer -- synth
 *   npm run sniffer -- ports
 *   npm run sniffer -- port COM3 [--duration 10000] [--out path.hex]
 */
import { mkdirSync } from "node:fs";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildSyntheticShotFrame,
  CONTROL,
  SHOT_FRAME_LENGTH,
} from "@rotpunktarena/protocol";
import { analyzeCaptureText, printFrameAnalysis } from "./analyze.ts";
import { bytesToHex } from "./capture.ts";
import { listSerialPorts, openAndSniff } from "./port.ts";
import {
  buildDemoFrame,
  printEvents,
  replayBytes,
  replayCaptureFile,
} from "./replay.ts";

const here = dirname(fileURLToPath(import.meta.url));
/** Default live-capture directory (repo docs/captures/live). */
const DEFAULT_LIVE_DIR = resolve(here, "../../../docs/captures/live");

function usage(): never {
  console.log(`RedDot sniffer — Protokoll-Replay und optionaler COM-Sniff

Befehle:
  replay <datei.hex>     Capture/Hex-Datei parsen (RX-Bytes)
  analyze <datei.hex>    Header [1..31] + Trailer [55..58] pro STX-Frame dumpem
  synth [opts]           Synthetischen 59-Byte-Schussframe erzeugen & parsen
  ports                  Verfügbare COM-Ports listen
  port <COMx> [opts]     Live-Sniff (9600 8N1); graceful ohne Hardware

synth-Optionen:
  --value 10.5           Ringwert-ASCII (4 Zeichen, Default 10.5)
  --distance 012.30      Distanz-ASCII (6 Zeichen)
  --x 00123              X-ASCII (5 Zeichen)
  --y -0045              Y-ASCII (5 Zeichen)
  --hex-only             Nur Hex ausgeben, nicht parsen

port-Optionen:
  --duration <ms>        Session-Länge (Default 30000 für Live-Captures)
  --poll <ms>            ENQ-Intervall (Default 300)
  --no-ack               Kein ACK nach Schussframe
  --out <datei.hex>      Capture unter docs/captures-Format speichern
  --out-dir <ordner>     Auto-Dateiname live-YYYYMMDD-HHMMSS.hex in Ordner
                         (Default-Ordner wenn weder --out noch --out-dir: docs/captures/live)

Beispiele:
  npm run sniffer -- replay ../../docs/captures/synthetic-shot.hex
  npm run sniffer -- analyze ../../docs/captures/synthetic-shot.hex
  npm run sniffer -- synth
  npm run sniffer -- ports
  npm run sniffer -- port COM3 --duration 60000
  npm run sniffer -- port COM3 --duration 60000 --out ../../docs/captures/live/shot-probe.hex
`);
  process.exit(1);
}

function argValue(args: string[], name: string, fallback?: string): string | undefined {
  const i = args.indexOf(name);
  if (i >= 0 && args[i + 1]) return args[i + 1]!;
  return fallback;
}

function hasFlag(args: string[], name: string): boolean {
  return args.includes(name);
}

function defaultLiveOutPath(outDir?: string): string {
  const dir = outDir ? resolve(outDir) : DEFAULT_LIVE_DIR;
  mkdirSync(dir, { recursive: true });
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  const stamp = `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}-${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`;
  return resolve(dir, `live-${stamp}.hex`);
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const cmd = argv[0];
  if (!cmd || cmd === "-h" || cmd === "--help") usage();

  if (cmd === "replay") {
    const file = argv[1];
    if (!file) usage();
    const text = readFileSync(resolve(file), "utf8");
    const events = replayCaptureFile(text);
    if (events.length === 0) {
      console.log("Keine Events (leerer Capture oder nur TX).");
      process.exit(0);
    }
    printEvents(events);
    const shots = events.filter((e) => e.type === "shot");
    console.log(`— ${events.length} Event(s), davon ${shots.length} Schuss`);
    process.exit(0);
  }

  if (cmd === "analyze") {
    const file = argv[1];
    if (!file) usage();
    const text = readFileSync(resolve(file), "utf8");
    const dumps = analyzeCaptureText(text);
    printFrameAnalysis(dumps);
    console.log(`\n— ${dumps.length} STX-Frame(s) analysiert —`);
    console.log(
      "Nächster Schritt: gleiche Offsets über mehrere Live-Captures vergleichen → docs/protocol.md aktualisieren.",
    );
    process.exit(0);
  }

  if (cmd === "synth") {
    const value = argValue(argv, "--value", "10.5")!;
    const distance = argValue(argv, "--distance", "012.30")!;
    const x = argValue(argv, "--x", "00123")!;
    const y = argValue(argv, "--y", "-0045")!;
    const frame = buildSyntheticShotFrame({
      valueAscii: value.padEnd(4).slice(0, 4),
      distanceAscii: distance.padEnd(6).slice(0, 6),
      xAscii: x.padStart(5, "0").slice(-5),
      yAscii: y.length === 5 ? y : y.padStart(5, " ").slice(-5),
    });

    const useDemo =
      value === "10.5" &&
      distance === "012.30" &&
      x === "00123" &&
      y === "-0045";
    const out = useDemo ? buildDemoFrame() : frame;

    console.log(`STX-Frame (${SHOT_FRAME_LENGTH} bytes):`);
    console.log(bytesToHex(out));
    if (!hasFlag(argv, "--hex-only")) {
      console.log("---");
      printEvents(replayBytes(Uint8Array.of(CONTROL.NAK, ...out)));
    }
    process.exit(0);
  }

  if (cmd === "ports") {
    const ports = await listSerialPorts();
    if (ports.length === 0) {
      console.log(
        "Keine COM-Ports gefunden (oder serialport nicht verfügbar). Live-Validierung wartet auf Hardware.",
      );
      process.exit(0);
    }
    for (const p of ports) {
      const extra = [p.manufacturer, p.friendlyName].filter(Boolean).join(" — ");
      console.log(extra ? `${p.path}\t${extra}` : p.path);
    }
    process.exit(0);
  }

  if (cmd === "port") {
    const path = argv[1];
    if (!path) usage();
    const durationMs = Number.parseInt(
      argValue(argv, "--duration", "30000")!,
      10,
    );
    const pollMs = Number.parseInt(argValue(argv, "--poll", "300")!, 10);
    const explicitOut = argValue(argv, "--out");
    const outDir = argValue(argv, "--out-dir");
    // Always persist live sessions somewhere under docs/captures/live unless --no-out
    const outPath = hasFlag(argv, "--no-out")
      ? undefined
      : explicitOut
        ? resolve(explicitOut)
        : defaultLiveOutPath(outDir);

    const code = await openAndSniff({
      path,
      durationMs,
      pollMs,
      ackShots: !hasFlag(argv, "--no-ack"),
      outPath,
    });
    process.exit(code);
  }

  usage();
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : err);
  process.exit(1);
});
