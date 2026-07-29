import { useEffect, useId, useRef } from "react";
import type { AlertDialogOptions, ConfirmDialogOptions } from "../hooks/useAppDialog";

type ConfirmProps = {
  kind: "confirm";
  options: ConfirmDialogOptions;
  onConfirm: () => void;
  onCancel: () => void;
};

type AlertProps = {
  kind: "alert";
  options: AlertDialogOptions;
  onOk: () => void;
};

type Props = ConfirmProps | AlertProps;

/** In-app modal card — same visual language as the startup update notice. */
export function AppDialog(props: Props) {
  const titleId = useId();
  const bodyId = useId();
  const primaryRef = useRef<HTMLButtonElement>(null);
  const options = props.options;
  const danger = props.kind === "confirm" && Boolean(props.options.danger);
  const eyebrow =
    options.eyebrow ?? (danger ? "Achtung" : props.kind === "alert" ? "Hinweis" : "Bestätigen");

  useEffect(() => {
    primaryRef.current?.focus();
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (props.kind === "confirm") props.onCancel();
        else props.onOk();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [props]);

  const confirmLabel =
    props.kind === "confirm"
      ? (props.options.confirmLabel ?? (danger ? "Löschen" : "OK"))
      : (props.options.okLabel ?? "OK");
  const cancelLabel =
    props.kind === "confirm" ? (props.options.cancelLabel ?? "Abbrechen") : null;

  return (
    <div
      className="app-dialog-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      aria-describedby={bodyId}
      onClick={(e) => {
        if (e.target !== e.currentTarget) return;
        if (props.kind === "confirm") props.onCancel();
        else props.onOk();
      }}
    >
      <div className={`app-dialog-card${danger ? " is-danger" : ""}`}>
        <p className="app-dialog-eyebrow">{eyebrow}</p>
        <h2 id={titleId}>{options.title}</h2>
        <p id={bodyId} className="app-dialog-body">
          {options.body}
        </p>
        <div className="app-dialog-actions">
          {cancelLabel ? (
            <button
              type="button"
              className="secondary"
              onClick={() => {
                if (props.kind === "confirm") props.onCancel();
              }}
            >
              {cancelLabel}
            </button>
          ) : null}
          <button
            ref={primaryRef}
            type="button"
            className={danger ? "app-dialog-danger" : undefined}
            onClick={() => {
              if (props.kind === "confirm") props.onConfirm();
              else props.onOk();
            }}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
