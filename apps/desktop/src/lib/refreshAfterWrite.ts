/** A committed write stays successful even if its subsequent display refresh fails. */
export function refreshAfterWrite<Args extends unknown[]>(
  load: (...args: Args) => Promise<void>,
  report: (message: string) => void,
): (...args: Args) => Promise<void> {
  return async (...args) => {
    try {
      await load(...args);
    } catch (error) {
      report(`Gespeichert. Die Anzeige konnte nicht aktualisiert werden. Bitte neu laden. ${String(error)}`);
    }
  };
}
