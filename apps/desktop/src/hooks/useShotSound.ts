import { useCallback, useEffect, useRef, useState } from "react";
import type { PlayShotOpts, ShotSoundTier } from "../live/presenceContract";

const MUTE_KEY = "reddot.muteShotSound";

function readMuted(): boolean {
  try {
    return localStorage.getItem(MUTE_KEY) === "1";
  } catch {
    return false;
  }
}

function makeNoiseBuffer(ctx: AudioContext, seconds: number) {
  const buf = ctx.createBuffer(1, Math.floor(ctx.sampleRate * seconds), ctx.sampleRate);
  const data = buf.getChannelData(0);
  for (let i = 0; i < data.length; i++) {
    data[i] = Math.random() * 2 - 1;
  }
  return buf;
}

/** Short synthesized impact / plink via Web Audio (no asset dependency). */
function playImpact(ctx: AudioContext, gain = 0.28, freq = 420) {
  const t0 = ctx.currentTime;
  const master = ctx.createGain();
  master.gain.setValueAtTime(0.0001, t0);
  master.gain.exponentialRampToValueAtTime(gain, t0 + 0.004);
  master.gain.exponentialRampToValueAtTime(0.0001, t0 + 0.12);
  master.connect(ctx.destination);

  const crack = ctx.createOscillator();
  crack.type = "triangle";
  crack.frequency.setValueAtTime(freq, t0);
  crack.frequency.exponentialRampToValueAtTime(freq * 0.22, t0 + 0.09);
  crack.connect(master);
  crack.start(t0);
  crack.stop(t0 + 0.12);

  const noiseBuf = makeNoiseBuffer(ctx, 0.04);
  const data = noiseBuf.getChannelData(0);
  for (let i = 0; i < data.length; i++) {
    data[i] *= 1 - i / data.length;
  }
  const noise = ctx.createBufferSource();
  noise.buffer = noiseBuf;
  const noiseGain = ctx.createGain();
  noiseGain.gain.setValueAtTime(gain * 0.65, t0);
  noiseGain.gain.exponentialRampToValueAtTime(0.0001, t0 + 0.04);
  noise.connect(noiseGain);
  noiseGain.connect(ctx.destination);
  noise.start(t0);
}

/** Bright chime for tens / inner / best. */
function playChime(ctx: AudioContext, freqs: number[], gain = 0.16) {
  const t0 = ctx.currentTime;
  for (let i = 0; i < freqs.length; i++) {
    const osc = ctx.createOscillator();
    osc.type = "sine";
    const start = t0 + i * 0.04;
    const end = start + 0.22;
    osc.frequency.setValueAtTime(freqs[i]!, start);
    const g = ctx.createGain();
    g.gain.setValueAtTime(0.0001, start);
    g.gain.exponentialRampToValueAtTime(gain - i * 0.03, start + 0.01);
    g.gain.exponentialRampToValueAtTime(0.0001, end);
    osc.connect(g);
    g.connect(ctx.destination);
    osc.start(start);
    osc.stop(end + 0.02);
  }
}

function playSeriesDone(ctx: AudioContext) {
  playChime(ctx, [392, 523, 659], 0.14);
}

/** Meme glass shatter for a nuller (0 rings) — still pure Web Audio. */
function playGlassBreak(ctx: AudioContext) {
  const t0 = ctx.currentTime;
  const noiseBuf = makeNoiseBuffer(ctx, 0.55);

  const bursts: { delay: number; dur: number; freq: number; q: number; gain: number }[] = [
    { delay: 0, dur: 0.08, freq: 3200, q: 0.7, gain: 0.55 },
    { delay: 0.04, dur: 0.12, freq: 4800, q: 1.1, gain: 0.42 },
    { delay: 0.09, dur: 0.16, freq: 2100, q: 0.9, gain: 0.35 },
    { delay: 0.15, dur: 0.2, freq: 5600, q: 1.4, gain: 0.28 },
    { delay: 0.22, dur: 0.25, freq: 1400, q: 0.6, gain: 0.22 },
  ];

  for (const b of bursts) {
    const src = ctx.createBufferSource();
    src.buffer = noiseBuf;
    const bp = ctx.createBiquadFilter();
    bp.type = "bandpass";
    bp.frequency.setValueAtTime(b.freq, t0 + b.delay);
    bp.Q.setValueAtTime(b.q, t0 + b.delay);
    const g = ctx.createGain();
    g.gain.setValueAtTime(0.0001, t0 + b.delay);
    g.gain.exponentialRampToValueAtTime(b.gain, t0 + b.delay + 0.005);
    g.gain.exponentialRampToValueAtTime(0.0001, t0 + b.delay + b.dur);
    src.connect(bp);
    bp.connect(g);
    g.connect(ctx.destination);
    src.start(t0 + b.delay);
    src.stop(t0 + b.delay + b.dur + 0.02);
  }

  const rings = [2650, 3400, 4100, 5200, 6800];
  for (let i = 0; i < rings.length; i++) {
    const osc = ctx.createOscillator();
    osc.type = "sine";
    const start = t0 + 0.02 + i * 0.035;
    const end = start + 0.35 + Math.random() * 0.15;
    osc.frequency.setValueAtTime(rings[i]!, start);
    osc.frequency.exponentialRampToValueAtTime(rings[i]! * 0.92, end);
    const g = ctx.createGain();
    g.gain.setValueAtTime(0.0001, start);
    g.gain.exponentialRampToValueAtTime(0.12 - i * 0.015, start + 0.01);
    g.gain.exponentialRampToValueAtTime(0.0001, end);
    osc.connect(g);
    g.connect(ctx.destination);
    osc.start(start);
    osc.stop(end + 0.02);
  }
}

function playTier(ctx: AudioContext, tier: ShotSoundTier) {
  switch (tier) {
    case "miss":
      playGlassBreak(ctx);
      break;
    case "mid":
    case "hit":
      playImpact(ctx, 0.26, 400);
      break;
    case "ten":
      playImpact(ctx, 0.22, 520);
      playChime(ctx, [880, 1175], 0.12);
      break;
    case "inner":
      playImpact(ctx, 0.2, 560);
      playChime(ctx, [988, 1319, 1568], 0.13);
      break;
    case "best":
      playChime(ctx, [659, 880, 1175], 0.15);
      break;
    case "seriesDone":
      playSeriesDone(ctx);
      break;
  }
}

export function useShotSound() {
  const [muted, setMuted] = useState(readMuted);
  const ctxRef = useRef<AudioContext | null>(null);

  useEffect(() => {
    try {
      localStorage.setItem(MUTE_KEY, muted ? "1" : "0");
    } catch {
      /* ignore */
    }
  }, [muted]);

  const ensureCtx = useCallback(() => {
    if (!ctxRef.current) {
      const AC =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      ctxRef.current = new AC();
    }
    const ctx = ctxRef.current;
    if (ctx.state === "suspended") {
      void ctx.resume();
    }
    return ctx;
  }, []);

  const playShot = useCallback(
    (opts?: PlayShotOpts) => {
      if (muted) return;
      try {
        const ctx = ensureCtx();
        if (opts?.tier) {
          playTier(ctx, opts.tier);
          return;
        }
        if (opts?.miss) playGlassBreak(ctx);
        else playImpact(ctx);
      } catch {
        /* ignore audio failures */
      }
    },
    [muted, ensureCtx],
  );

  const toggleMute = useCallback(() => {
    setMuted((m) => !m);
  }, []);

  return { muted, toggleMute, playShot };
}
