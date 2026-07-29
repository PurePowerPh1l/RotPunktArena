export type ConfirmDialogOptions = {
  title: string;
  body: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  eyebrow?: string;
};

export type AlertDialogOptions = {
  title: string;
  body: string;
  okLabel?: string;
  eyebrow?: string;
};

export type AppDialogRequest =
  | {
      kind: "confirm";
      options: ConfirmDialogOptions;
      resolve: (ok: boolean) => void;
    }
  | {
      kind: "alert";
      options: AlertDialogOptions;
      resolve: () => void;
    };

type Listener = (request: AppDialogRequest | null) => void;

let current: AppDialogRequest | null = null;
const listeners = new Set<Listener>();
const queue: Array<() => void> = [];

function emit(): void {
  for (const listener of listeners) {
    listener(current);
  }
}

export function subscribeAppDialog(listener: Listener): () => void {
  listeners.add(listener);
  listener(current);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Promise-based confirm — resolves true/false. Replaces window.confirm.
 * Only one dialog visible; further calls queue until the current closes.
 */
export function confirmDialog(options: ConfirmDialogOptions): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    const show = () => {
      current = {
        kind: "confirm",
        options,
        resolve: (ok) => {
          current = null;
          emit();
          resolve(ok);
          const next = queue.shift();
          next?.();
        },
      };
      emit();
    };
    if (current == null) show();
    else queue.push(show);
  });
}

/** Promise-based alert — resolves when dismissed. Replaces window.alert. */
export function alertDialog(options: AlertDialogOptions): Promise<void> {
  return new Promise<void>((resolve) => {
    const show = () => {
      current = {
        kind: "alert",
        options,
        resolve: () => {
          current = null;
          emit();
          resolve();
          const next = queue.shift();
          next?.();
        },
      };
      emit();
    };
    if (current == null) show();
    else queue.push(show);
  });
}
