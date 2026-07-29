import { useEffect, useState } from "react";
import {
  subscribeAppDialog,
  type AppDialogRequest,
} from "../hooks/useAppDialog";
import { AppDialog } from "./AppDialog";

/** Single mount point for promise-based confirm/alert dialogs. */
export function AppDialogHost() {
  const [request, setRequest] = useState<AppDialogRequest | null>(null);

  useEffect(() => subscribeAppDialog(setRequest), []);

  if (!request) return null;

  if (request.kind === "confirm") {
    return (
      <AppDialog
        kind="confirm"
        options={request.options}
        onConfirm={() => request.resolve(true)}
        onCancel={() => request.resolve(false)}
      />
    );
  }

  return (
    <AppDialog
      kind="alert"
      options={request.options}
      onOk={() => request.resolve()}
    />
  );
}
