import { useCallback, useEffect, useRef, useState } from "react";

type WinApi = {
  isFullscreen: () => Promise<boolean>;
  isMaximized: () => Promise<boolean>;
  setFullscreen: (v: boolean) => Promise<void>;
  unmaximize: () => Promise<void>;
  onResized: (handler: () => void) => Promise<() => void>;
};

async function getWin(): Promise<WinApi | null> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    return getCurrentWindow() as unknown as WinApi;
  } catch {
    return null;
  }
}

/**
 * Exclusive fullscreen only via UI button (and Esc to leave).
 * Windows maximize / snap / title-bar stay normal maximize — no hijack.
 */
export function useTrueFullscreen() {
  const [fullscreen, setFullscreenState] = useState(false);
  const busy = useRef(false);
  const winRef = useRef<WinApi | null>(null);

  const sync = useCallback(async () => {
    const win = winRef.current ?? (await getWin());
    winRef.current = win;
    if (!win) {
      setFullscreenState(Boolean(document.fullscreenElement));
      return;
    }
    setFullscreenState(await win.isFullscreen());
  }, []);

  const enterFullscreen = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    try {
      const win = winRef.current ?? (await getWin());
      winRef.current = win;
      if (win) {
        if (await win.isMaximized()) await win.unmaximize();
        await win.setFullscreen(true);
      } else if (!document.fullscreenElement) {
        await document.documentElement.requestFullscreen?.();
      }
      await sync();
    } catch {
      /* ignore */
    } finally {
      busy.current = false;
    }
  }, [sync]);

  const exitFullscreen = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    try {
      const win = winRef.current ?? (await getWin());
      winRef.current = win;
      if (win) {
        if (await win.isFullscreen()) await win.setFullscreen(false);
      } else if (document.fullscreenElement) {
        await document.exitFullscreen?.();
      }
      await sync();
    } catch {
      /* ignore */
    } finally {
      busy.current = false;
    }
  }, [sync]);

  const toggleFullscreen = useCallback(async () => {
    const win = winRef.current ?? (await getWin());
    winRef.current = win;
    const on = win
      ? await win.isFullscreen()
      : Boolean(document.fullscreenElement);
    if (on) await exitFullscreen();
    else await enterFullscreen();
  }, [enterFullscreen, exitFullscreen]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void (async () => {
      const win = await getWin();
      if (cancelled) return;
      winRef.current = win;
      await sync();

      if (!win) return;

      // Sync state only — do not promote maximize/snap to exclusive fullscreen.
      unlisten = await win.onResized(() => {
        if (!busy.current) void sync();
      });
    })();

    const onFsChange = () => void sync();
    document.addEventListener("fullscreenchange", onFsChange);

    return () => {
      cancelled = true;
      unlisten?.();
      document.removeEventListener("fullscreenchange", onFsChange);
    };
  }, [sync]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Esc leaves exclusive fullscreen (no title bar otherwise).
      if (e.key !== "Escape") return;
      void (async () => {
        const win = winRef.current ?? (await getWin());
        winRef.current = win;
        const on = win
          ? await win.isFullscreen()
          : Boolean(document.fullscreenElement);
        if (on) {
          e.preventDefault();
          void exitFullscreen();
        }
      })();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [exitFullscreen]);

  useEffect(() => {
    document.body.classList.toggle("is-app-fullscreen", fullscreen);
    return () => document.body.classList.remove("is-app-fullscreen");
  }, [fullscreen]);

  return { fullscreen, enterFullscreen, exitFullscreen, toggleFullscreen };
}
