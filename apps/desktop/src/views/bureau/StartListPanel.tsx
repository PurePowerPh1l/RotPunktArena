import { useEffect, useMemo, useState, type FormEvent, type PointerEvent } from "react";
import type { Competition, CompetitionEntry, CompetitionTeam, EntryStatus } from "@rotpunktarena/domain";
import { OverflowMenu } from "../../components/OverflowMenu";
import { SearchSelect } from "../../components/SearchSelect";
import { formatPersonName } from "../../lib/format";
import { peekReddotDrag } from "../../lib/reddotDnd";
import { ENTRY_LABEL } from "./labels";

export type ArenaHandoff = {
  competitionId: string;
  entryId: string;
};

type Props = {
  selected: Competition | null;
  selectedId: string | null;
  competitions: Competition[];
  entries: CompetitionEntry[];
  teams: CompetitionTeam[];
  teamsActive: boolean;
  busy?: boolean;
  /** Admin-Modus: Fertig-Status darf geändert / Eintrag entfernt werden. */
  adminMode?: boolean;
  teamByEntry: Map<string, string>;
  /** Highlight while reordering via pointer DnD. */
  dragOverEntry: string | null;
  /** Entry currently being dragged (dim row). */
  draggingEntryId: string | null;
  listDropActive: boolean;
  onListDropActive: (active: boolean) => void;
  onAddByName: (name: string) => Promise<boolean>;
  onSetEntryStatus: (entryId: string, status: EntryStatus) => Promise<boolean>;
  onRemoveEntry: (entryId: string) => Promise<boolean>;
  /** Archive person globally (Stammdata), then drop from this start list. */
  onArchivePerson: (personId: string) => Promise<boolean>;
  onSetEntryTeam: (entryId: string, teamId: string) => Promise<boolean | void>;
  onCloneFrom: (fromId: string) => Promise<boolean>;
  onOpenLive: (handoff?: ArenaHandoff) => void;
  /** Open results sheet for this starter (e.g. from Nachkauf badge). */
  onOpenEntryResults?: (entryId: string) => void;
  /** Pointer-drag: reorder onto another row, or drop outside to remove. */
  onBeginEntryDrag?: (
    entryId: string,
    label: string,
    e: PointerEvent,
  ) => void;
};

export function StartListPanel({
  selected,
  selectedId,
  competitions,
  entries,
  teams,
  teamsActive,
  busy = false,
  adminMode = false,
  teamByEntry,
  dragOverEntry,
  draggingEntryId,
  listDropActive,
  onListDropActive,
  onAddByName,
  onSetEntryStatus,
  onRemoveEntry,
  onArchivePerson,
  onSetEntryTeam,
  onCloneFrom,
  onOpenLive,
  onOpenEntryResults,
  onBeginEntryDrag,
}: Props) {
  const [cloneFromId, setCloneFromId] = useState("");
  const [quickName, setQuickName] = useState("");
  const [selectedEntryId, setSelectedEntryId] = useState<string | null>(null);

  const isTraining = selected?.kind === "training";
  const activeTeams = teams.filter((t) => !t.archived);
  const locked = selected?.status === "closed";
  const canEdit = Boolean(selected) && !locked && !busy;

  const cloneOptions = useMemo(
    () =>
      competitions
        .filter((c) => c.id !== selectedId)
        .map((c) => ({
          id: c.id,
          label: `${c.name}${c.kind === "training" ? " · Training" : ""}`,
        })),
    [competitions, selectedId],
  );

  const teamOptions = useMemo(
    () => activeTeams.map((t) => ({ id: t.id, label: t.name })),
    [activeTeams],
  );

  const statusOptions = useMemo(
    () =>
      (Object.keys(ENTRY_LABEL) as EntryStatus[]).map((s) => ({
        id: s,
        label: ENTRY_LABEL[s],
      })),
    [],
  );

  useEffect(() => {
    setSelectedEntryId((prev) =>
      prev && entries.some((e) => e.id === prev) ? prev : null,
    );
  }, [entries, selectedId]);

  useEffect(() => {
    if (!selectedEntryId) return;
    const onDoc = (ev: MouseEvent) => {
      const t = ev.target as HTMLElement | null;
      if (!t) return;
      // Other start-list rows update selection themselves — don't clear first.
      if (t.closest("[data-start-entry]")) return;
      if (t.closest("[data-arena-open]")) return;
      setSelectedEntryId(null);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [selectedEntryId]);

  const arenaTarget = selectedEntryId
    ? entries.find((e) => e.id === selectedEntryId) ?? null
    : null;

  const onQuickAdd = async (e: FormEvent) => {
    e.preventDefault();
    const name = quickName.trim();
    if (!name || !canEdit) return;
    const ok = await onAddByName(name);
    if (ok) setQuickName("");
  };

  const archiveEntry = async (e: CompetitionEntry) => {
    const name = formatPersonName(e.lastName, e.firstName, "");
    const ok = window.confirm(
      `„${name}“ archivieren?\n\nDer Schütze wird global ausgeblendet und aus dieser Startliste entfernt.`,
    );
    if (!ok) return;
    const archived = await onArchivePerson(e.personId);
    if (archived) await onRemoveEntry(e.id);
  };

  const openArena = async () => {
    if (!selected || !arenaTarget || locked || selected.status === "template") {
      return;
    }
    const ok =
      arenaTarget.status === "active"
        ? true
        : await onSetEntryStatus(arenaTarget.id, "active");
    if (!ok) return;
    onOpenLive({ competitionId: selected.id, entryId: arenaTarget.id });
  };

  return (
    <section
      data-start-list-drop={canEdit ? "true" : "false"}
      className={`panel start-list-panel${listDropActive ? " drop-target-active" : ""}${locked ? " start-list-locked" : ""}`}
      onDragLeave={(e) => {
        if (e.currentTarget.contains(e.relatedTarget as Node | null)) return;
        onListDropActive(false);
      }}
      onDrop={(e) => {
        // Ignore person drops here — handled by pointer DnD in BureauView
        if (peekReddotDrag()?.kind === "person") {
          e.preventDefault();
        }
      }}
    >
      <div className="bureau-fill-head">
        <div>
          <h2>{isTraining ? "Trainingsstartliste" : "Startliste"}</h2>
          <p className="panel-lead">
            {!selected
              ? "Oben einen Wettkampf wählen."
              : locked
                ? `${selected.name} · geschlossen`
                : selected.status === "template"
                  ? "Vorlage — zum Schießen zuerst als Wettkampf anlegen."
                  : "Starter wählen → Zur Arena · ⋮⋮ aus Startliste ziehen."}
          </p>
          {locked && selected ? (
            <p className="hint start-list-lock-hint">
              Zum Ändern den Status wieder auf Aktiv setzen.
            </p>
          ) : null}
        </div>

        {selected ? (
          <div className="start-list-toolbar">
            <form className="clone-row start-quick-add" onSubmit={(e) => void onQuickAdd(e)}>
              <label className="field grow">
                Schnellstarter
                <input
                  value={quickName}
                  onChange={(e) => setQuickName(e.target.value)}
                  placeholder="Name tippen — legt Schütze + Starter an…"
                  disabled={!canEdit}
                />
              </label>
              <button type="submit" disabled={!quickName.trim() || !canEdit}>
                {busy ? "…" : "Hinzufügen"}
              </button>
            </form>

            <div className="start-list-toolbar-side">
              {!locked && selected.status !== "template" ? (
                <button
                  type="button"
                  data-arena-open
                  disabled={busy || !arenaTarget}
                  title={
                    arenaTarget
                      ? `${formatPersonName(arenaTarget.lastName, arenaTarget.firstName, "")} in der Arena laden`
                      : "Zuerst einen Starter in der Liste wählen"
                  }
                  onClick={() => void openArena()}
                >
                  Zur Arena
                </button>
              ) : null}
              <OverflowMenu
                label="Mehr"
                ariaLabel="Weitere Startlisten-Aktionen"
                className="start-more"
                menuClassName="start-more-menu"
                disabled={!canEdit}
              >
                <p className="row-overflow-menu-title">Liste übernehmen</p>
                <label className="field">
                  Aus Wettkampf
                  <SearchSelect
                    value={cloneFromId}
                    options={cloneOptions}
                    onChange={setCloneFromId}
                    disabled={!canEdit}
                    placeholder="Anderer Wettkampf…"
                    allowClear
                  />
                </label>
                <button
                  type="button"
                  className="secondary"
                  disabled={!canEdit || !cloneFromId}
                  onClick={() => void onCloneFrom(cloneFromId)}
                >
                  Übernehmen
                </button>
              </OverflowMenu>
            </div>
          </div>
        ) : null}
      </div>

      <div className="start-list-box">
        <div className="start-table-wrap">
          <table className="start-table">
            <thead>
              <tr>
                <th aria-label="Reihenfolge" />
                <th>#</th>
                <th>Name</th>
                <th>Verein</th>
                {teamsActive ? <th>Team</th> : null}
                <th>Status</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {entries.map((e) => {
                const isSelected = selectedEntryId === e.id;
                const label = formatPersonName(e.lastName, e.firstName, "");
                const isDone = e.status === "done";
                const canRemove = canEdit;
                const canDragOut = canEdit && Boolean(onBeginEntryDrag);
                const nk = Math.max(0, e.nachkaufPurchased ?? 0);
                return (
                  <tr
                    key={e.id}
                    data-start-entry={e.id}
                    className={[
                      e.status === "active" ? "row-active" : "",
                      isDone ? "row-done" : "",
                      isSelected ? "row-selected" : "",
                      dragOverEntry === e.id ? "row-drop" : "",
                      draggingEntryId === e.id ? "row-dragging" : "",
                    ]
                      .filter(Boolean)
                      .join(" ") || undefined}
                    aria-selected={isSelected}
                    onClick={() => setSelectedEntryId(e.id)}
                  >
                    <td
                      className={`drag-handle${canDragOut ? " is-active" : ""}`}
                      title={
                        canDragOut
                          ? "Ziehen: Reihenfolge ändern oder aus der Liste nehmen"
                          : undefined
                      }
                      onPointerDown={(ev) => {
                        if (!canDragOut || !onBeginEntryDrag) return;
                        const t = ev.target as HTMLElement;
                        if (t.closest("button, input, select, a, .shooter-ac")) return;
                        ev.preventDefault();
                        ev.stopPropagation();
                        onBeginEntryDrag(e.id, label, ev);
                      }}
                    >
                      {canEdit ? "⋮⋮" : ""}
                    </td>
                    <td>{e.startOrder}</td>
                    <td>
                      <span className="start-name-cell">
                        <span className="start-name">{label}</span>
                        {nk > 0 ? (
                          <button
                            type="button"
                            className="start-nk-badge"
                            title={
                              nk === 1
                                ? "1× Nachkauf — Ergebnisse ansehen (Vereinsdank gebucht)"
                                : `${nk}× Nachkauf — Ergebnisse ansehen (Vereinsdank gebucht)`
                            }
                            onClick={(ev) => {
                              ev.stopPropagation();
                              onOpenEntryResults?.(e.id);
                            }}
                          >
                            NK{nk > 1 ? `×${nk}` : ""}
                          </button>
                        ) : null}
                      </span>
                    </td>
                    <td>{e.club ?? "—"}</td>
                    {teamsActive ? (
                      <td onClick={(ev) => ev.stopPropagation()}>
                        {canEdit ? (
                          <SearchSelect
                            value={teamByEntry.get(e.id) ?? ""}
                            options={teamOptions}
                            onChange={(id) => void onSetEntryTeam(e.id, id)}
                            disabled={busy}
                            placeholder="Team…"
                            allowClear
                          />
                        ) : (
                          activeTeams.find((t) => t.id === teamByEntry.get(e.id))?.name ?? "—"
                        )}
                      </td>
                    ) : null}
                    <td onClick={(ev) => ev.stopPropagation()}>
                      {canEdit ? (
                        <SearchSelect
                          value={e.status}
                          options={statusOptions}
                          onChange={(id) =>
                            void onSetEntryStatus(e.id, id as EntryStatus)
                          }
                          disabled={busy}
                          placeholder="Status…"
                          allowClear={false}
                        />
                      ) : (
                        ENTRY_LABEL[e.status] ?? e.status
                      )}
                    </td>
                    <td onClick={(ev) => ev.stopPropagation()}>
                      {canEdit ? (
                        <OverflowMenu ariaLabel="Starter-Aktionen">
                          <button
                            type="button"
                            className="ghost"
                            disabled={busy}
                            onClick={() => void archiveEntry(e)}
                          >
                            Archivieren
                          </button>
                          <button
                            type="button"
                            className="ghost danger-text"
                            disabled={busy || !canRemove}
                            title={
                              isDone && !adminMode
                                ? "Fertig — Admin-Passwort beim Entfernen"
                                : undefined
                            }
                            onClick={() => void onRemoveEntry(e.id)}
                          >
                            Entfernen
                          </button>
                        </OverflowMenu>
                      ) : null}
                    </td>
                  </tr>
                );
              })}
              {entries.length === 0 ? (
                <tr>
                  <td colSpan={teamsActive ? 7 : 6} className="empty">
                    Noch keine Starter — Schnellstarter oben oder aus der Schützenliste ziehen.
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}
