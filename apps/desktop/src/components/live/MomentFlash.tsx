import type { MomentFlashKind } from "../../hooks/useTrainingMoments";

type Props = {
  flashKind: MomentFlashKind;
  toast: string | null;
  onDismissToast?: () => void;
  className?: string;
};

/** Non-blocking training beat overlay (flash ring + short toast). */
export function MomentFlash({
  flashKind,
  toast,
  onDismissToast,
  className,
}: Props) {
  if (!flashKind && !toast) return null;

  return (
    <div
      className={["moment-layer", className].filter(Boolean).join(" ")}
      aria-live="polite"
    >
      {flashKind ? (
        <div
          className={`moment-flash moment-flash-${flashKind}`}
          aria-hidden
        />
      ) : null}
      {toast ? (
        <button
          type="button"
          className="moment-toast"
          onClick={onDismissToast}
        >
          {toast}
        </button>
      ) : null}
    </div>
  );
}
