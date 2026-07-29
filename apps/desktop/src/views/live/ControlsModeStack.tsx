import type { ReactNode } from "react";

type Props = {
  mode: "training" | "competition";
  training: ReactNode;
  competition: ReactNode;
};

/**
 * Both panes stay mounted in one grid cell so the footer keeps a stable
 * height (max of both). Only opacity crossfades — no height morph/pop.
 */
export function ControlsModeStack({ mode, training, competition }: Props) {
  return (
    <div className="controls-mode-stack" data-mode={mode}>
      <div
        className={`controls-mode-pane${mode === "training" ? " is-active" : ""}`}
        data-mode="training"
        aria-hidden={mode !== "training"}
      >
        {training}
      </div>
      <div
        className={`controls-mode-pane${mode === "competition" ? " is-active" : ""}`}
        data-mode="competition"
        aria-hidden={mode !== "competition"}
      >
        {competition}
      </div>
    </div>
  );
}
