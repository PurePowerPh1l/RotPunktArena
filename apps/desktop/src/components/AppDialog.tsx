import { useId } from "react";
import { createPortal } from "react-dom";
import { useModal } from "../hooks/useModal";
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
  const modal = useModal(true, () => { if (props.kind === "confirm") props.onCancel(); else props.onOk(); });
  const options = props.options;
  const danger = props.kind === "confirm" && Boolean(props.options.danger);
  const eyebrow =
    options.eyebrow ?? (danger ? "Achtung" : props.kind === "alert" ? "Hinweis" : "Bestätigen");

  const confirmLabel =
    props.kind === "confirm"
      ? (props.options.confirmLabel ?? (danger ? "Löschen" : "OK"))
      : (props.options.okLabel ?? "OK");
  const cancelLabel =
    props.kind === "confirm" ? (props.options.cancelLabel ?? "Abbrechen") : null;

  return createPortal(
    <div
      ref={modal}
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
              data-modal-initial={danger || undefined}
              onClick={() => {
                if (props.kind === "confirm") props.onCancel();
              }}
            >
              {cancelLabel}
            </button>
          ) : null}
          <button
            data-modal-initial={!danger || undefined}
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
    </div>, document.body
  );
}
