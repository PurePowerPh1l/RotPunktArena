import type { MomentFlashKind } from "../../hooks/useTrainingMoments";

type Props = {
  flashKind: MomentFlashKind;
  toast: string | null;
  onDismissToast?: () => void;
};

/** Non-blocking training beat overlay (flash ring + short toast). */
export function MomentFlash({ flashKind, toast, onDismissToast }: Props) {
  if (!flashKind && !toast) return null;

  return (
    <div className="moment-layer" aria-live="polite">
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
