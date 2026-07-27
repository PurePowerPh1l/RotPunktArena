import { useState, type FormEvent, type PointerEvent } from "react";
import type { Person } from "@reddot/domain";
import { OverflowMenu } from "../../components/OverflowMenu";

type Props = {
  people: Person[];
  peopleQuery: string;
  selectedId: string | null;
  inList: Set<string>;
  busy?: boolean;
  includeArchived: boolean;
  onIncludeArchivedChange: (value: boolean) => void;
  onPeopleQueryChange: (q: string) => void;
  onCreate: (input: {
    firstName: string;
    lastName: string;
    club?: string;
  }) => Promise<boolean>;
  onUpdate: (
    personId: string,
    input: {
      firstName: string;
      lastName: string;
      club?: string;
    },
  ) => Promise<boolean>;
  onDelete: (personId: string) => Promise<boolean>;
  onSetArchived: (personId: string, archived: boolean) => Promise<boolean>;
  onAddToStartList: (personId: string) => Promise<boolean>;
  onRemoveFromStartList: (personId: string) => Promise<boolean>;
  /** Person IDs with start-list status `done` in the selected competition. */
  doneInList?: Set<string>;
  /** Competition allows Nachkauf — done shooters show gold label instead of red Fertig. */
  nachkaufEnabled?: boolean;
  /** When true, cannot add people to the current start list. */
  startListLocked?: boolean;
  /** Pointer-drag onto start list (preferred over HTML5 DnD). */
  onBeginPersonDrag?: (
    personId: string,
    label: string,
    e: PointerEvent,
  ) => void;
};

export function PeoplePanel({
  people,
  peopleQuery,
  selectedId,
  inList,
  busy = false,
  includeArchived,
  onIncludeArchivedChange,
  onPeopleQueryChange,
  onCreate,
  onUpdate,
  onDelete,
  onSetArchived,
  onAddToStartList,
  onRemoveFromStartList,
  doneInList,
  nachkaufEnabled = false,
  startListLocked = false,
  onBeginPersonDrag,
}: Props) {
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [club, setClub] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);

  const resetForm = () => {
    setFirstName("");
    setLastName("");
    setClub("");
    setEditingId(null);
  };

  const startEdit = (p: Person) => {
    setEditingId(p.id);
    setFirstName(p.firstName);
    setLastName(p.lastName);
    setClub(p.club ?? "");
  };

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (busy) return;
    const input = {
      firstName,
      lastName,
      club: club || undefined,
    };
    const ok = editingId
      ? await onUpdate(editingId, input)
      : await onCreate(input);
    if (ok) resetForm();
  };

  const confirmDelete = (p: Person) => {
    const name = `${p.lastName}, ${p.firstName}`;
    const ok = window.confirm(
      `„${name}“ wirklich endgültig löschen?\n\nDer Schütze wird aus allen Startlisten entfernt. Trainingsdaten werden ebenfalls gelöscht.`,
    );
    if (!ok) return;
    if (editingId === p.id) resetForm();
    void onDelete(p.id);
  };

  return (
    <section className="panel bureau-slot-panel">
      <p className="panel-lead">
        Stammdaten für alle Wettkämpfe — einmal anlegen, überall nutzen.
      </p>
      <form className="stack-form" onSubmit={(e) => void onSubmit(e)}>
        <div className="row-2">
          <label className="field">
            Vorname
            <input
              value={firstName}
              onChange={(e) => setFirstName(e.target.value)}
              required
            />
          </label>
          <label className="field">
            Nachname
            <input
              value={lastName}
              onChange={(e) => setLastName(e.target.value)}
              required
            />
          </label>
        </div>
        <label className="field">
          Verein
          <input value={club} onChange={(e) => setClub(e.target.value)} />
        </label>
        {editingId ? (
          <div className="row-2">
            <button type="submit" disabled={busy}>
              Änderungen speichern
            </button>
            <button
              type="button"
              className="secondary"
              disabled={busy}
              onClick={resetForm}
            >
              Abbrechen
            </button>
          </div>
        ) : (
          <button type="submit" disabled={busy}>
            Anlegen
          </button>
        )}
      </form>

      <div className="people-filter">
        <label className="field">
          Suche
          <input
            value={peopleQuery}
            onChange={(e) => onPeopleQueryChange(e.target.value)}
            placeholder="Name oder Verein…"
          />
        </label>

        <label
          className="check-field people-archiv-check"
          title="Archivierte Schützen wieder einblenden"
        >
          <input
            type="checkbox"
            checked={includeArchived}
            onChange={(e) => onIncludeArchivedChange(e.target.checked)}
          />
          Archiv anzeigen
        </label>
      </div>

      <div className="people-list-head">
        <h2 className="people-list-heading">Schützenliste</h2>
        <p className="hint people-list-hint">
          {startListLocked
            ? "Wettkampf geschlossen — Startliste gesperrt."
            : selectedId
              ? "Ziehen oder Zur Startliste · Dabei nimmt wieder raus."
              : "Zuerst einen Wettkampf wählen, dann aufnehmen."}
        </p>
      </div>
      <div className="people-list-box">
        <ul className="entity-list compact people-entity-list">
          {people.map((p) => {
            const already = inList.has(p.id);
            const isDone = Boolean(doneInList?.has(p.id));
            const archived = Boolean(p.archived);
            const isEditing = editingId === p.id;
            const canDrag =
              !already &&
              !archived &&
              !startListLocked &&
              Boolean(selectedId) &&
              Boolean(onBeginPersonDrag);
            const label = `${p.lastName}, ${p.firstName}`;
            return (
              <li
                key={p.id}
                className={`person-row${canDrag ? " person-row-draggable" : ""}${isEditing ? " person-row-editing" : ""}`}
                title={
                  canDrag
                    ? "Zur Startliste ziehen"
                    : startListLocked
                      ? "Wettkampf geschlossen — Ziehen nicht möglich"
                      : undefined
                }
                onPointerDown={(e) => {
                  if (!canDrag || !onBeginPersonDrag) return;
                  const t = e.target as HTMLElement;
                  if (t.closest(".person-row-actions, button, input, select, a")) {
                    return;
                  }
                  e.preventDefault();
                  onBeginPersonDrag(p.id, label, e);
                }}
              >
                <span
                  className={`person-drag-handle${canDrag ? "" : " is-disabled"}`}
                  aria-hidden
                >
                  ⋮⋮
                </span>
                <div className="person-row-main">
                  <span className="list-title">
                    {label}
                    {archived ? " · Archiv" : ""}
                  </span>
                  {p.club ? <span className="list-meta">{p.club}</span> : null}
                </div>
                <div className="person-row-actions">
                  {archived ? (
                    <button
                      type="button"
                      className="secondary"
                      disabled={busy}
                      onClick={() => void onSetArchived(p.id, false)}
                    >
                      Wiederherstellen
                    </button>
                  ) : isDone ? (
                    <button
                      type="button"
                      className={nachkaufEnabled ? "nachkauf-btn" : "fertig-btn"}
                      disabled
                      title={
                        nachkaufEnabled
                          ? "Serie beendet — Nachkauf in der Arena möglich"
                          : "Serie beendet — Entfernen nur im Admin-Modus über die Startliste"
                      }
                    >
                      {nachkaufEnabled ? "Nachkauf" : "Fertig"}
                    </button>
                  ) : (
                    <button
                      type="button"
                      className={already ? "dabei-btn" : undefined}
                      disabled={busy || !selectedId || startListLocked}
                      onClick={() =>
                        void (already
                          ? onRemoveFromStartList(p.id)
                          : onAddToStartList(p.id))
                      }
                      title={
                        !selectedId
                          ? "Zuerst einen Wettkampf wählen"
                          : startListLocked
                            ? "Wettkampf ist geschlossen — zuerst auf Aktiv setzen"
                            : already
                              ? "Aus der Startliste nehmen"
                              : "Zur Startliste hinzufügen"
                      }
                    >
                      {already ? "Dabei" : "Zur Startliste"}
                    </button>
                  )}
                  <OverflowMenu ariaLabel="Weitere Aktionen">
                    <button
                      type="button"
                      className="ghost"
                      disabled={busy}
                      onClick={() => startEdit(p)}
                    >
                      Bearbeiten
                    </button>
                    {!archived ? (
                      <button
                        type="button"
                        className="ghost"
                        disabled={busy}
                        title="Aus aktiven Listen ausblenden"
                        onClick={() => void onSetArchived(p.id, true)}
                      >
                        Archivieren
                      </button>
                    ) : null}
                    <button
                      type="button"
                      className="ghost danger-text"
                      disabled={busy}
                      title="Schütze endgültig löschen"
                      onClick={() => confirmDelete(p)}
                    >
                      Löschen
                    </button>
                  </OverflowMenu>
                </div>
              </li>
            );
          })}
          {people.length === 0 ? (
            <li className="empty-soft">Keine Schützen gefunden.</li>
          ) : null}
        </ul>
      </div>
    </section>
  );
}
