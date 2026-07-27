import { useMemo, useState, type FormEvent } from "react";
import type { CompetitionTeam, Person } from "@reddot/domain";
import { SearchSelect } from "../../components/SearchSelect";
import { formatPersonName } from "../../lib/format";

type Props = {
  teams: CompetitionTeam[];
  people: Person[];
  busy?: boolean;
  includeArchived: boolean;
  onIncludeArchivedChange: (value: boolean) => void;
  onCreate: (name: string) => Promise<boolean>;
  onRename: (teamId: string, name: string) => Promise<boolean>;
  onSetArchived: (teamId: string, archived: boolean) => Promise<boolean>;
  onDelete: (teamId: string) => Promise<boolean>;
  onAddPerson: (teamId: string, personId: string) => Promise<boolean>;
  onRemovePerson: (teamId: string, personId: string) => Promise<boolean>;
};

export function TeamsMasterPanel({
  teams,
  people,
  busy = false,
  includeArchived,
  onIncludeArchivedChange,
  onCreate,
  onRename,
  onSetArchived,
  onDelete,
  onAddPerson,
  onRemovePerson,
}: Props) {
  const [teamName, setTeamName] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [assignPersonId, setAssignPersonId] = useState("");

  const peopleById = useMemo(() => {
    const m = new Map<string, Person>();
    for (const p of people) m.set(p.id, p);
    return m;
  }, [people]);

  const personTeamId = useMemo(() => {
    const m = new Map<string, string>();
    for (const t of teams) {
      if (t.archived) continue;
      for (const pid of t.memberPersonIds ?? []) m.set(pid, t.id);
    }
    return m;
  }, [teams]);

  const selected = teams.find((t) => t.id === selectedId) ?? null;

  const assignOptions = useMemo(() => {
    const memberIds = new Set(selected?.memberPersonIds ?? []);
    return people
      .filter((p) => !p.archived && !memberIds.has(p.id))
      .map((p) => {
        const name = formatPersonName(p.lastName, p.firstName);
        const club = p.club?.trim();
        const otherTeamId = personTeamId.get(p.id);
        const otherTeam =
          otherTeamId && otherTeamId !== selectedId
            ? teams.find((t) => t.id === otherTeamId)?.name
            : null;
        const bits = [name, club || null, otherTeam ? `jetzt: ${otherTeam}` : null].filter(
          Boolean,
        );
        return { id: p.id, label: bits.join(" · ") };
      });
  }, [people, personTeamId, selected?.memberPersonIds, selectedId, teams]);

  const onSubmitCreate = async (e: FormEvent) => {
    e.preventDefault();
    if (busy || !teamName.trim()) return;
    const ok = await onCreate(teamName.trim());
    if (ok) setTeamName("");
  };

  const startRename = (t: CompetitionTeam) => {
    setRenamingId(t.id);
    setRenameValue(t.name);
    setSelectedId(t.id);
  };

  const submitRename = async (e: FormEvent) => {
    e.preventDefault();
    if (!renamingId || busy || !renameValue.trim()) return;
    const ok = await onRename(renamingId, renameValue.trim());
    if (ok) setRenamingId(null);
  };

  const confirmDelete = (t: CompetitionTeam) => {
    const ok = window.confirm(
      `„${t.name}“ wirklich endgültig löschen?\n\nDie Team-Mitgliedschaften entfallen. Wettkampfergebnisse bleiben erhalten.`,
    );
    if (!ok) return;
    if (selectedId === t.id) setSelectedId(null);
    if (renamingId === t.id) setRenamingId(null);
    void onDelete(t.id);
  };

  const onAssign = async (e: FormEvent) => {
    e.preventDefault();
    if (!selected || busy || !assignPersonId) return;
    const ok = await onAddPerson(selected.id, assignPersonId);
    if (ok) setAssignPersonId("");
  };

  return (
    <section className="panel bureau-slot-panel">
      <p className="panel-lead">
        Vereinsmannschaften — gelten für alle Wettkämpfe. Schützen können hier dem Team
        zugeordnet oder umgezogen werden.
      </p>

      <form className="clone-row" onSubmit={(e) => void onSubmitCreate(e)}>
        <label className="field grow">
          Teamname
          <input
            value={teamName}
            onChange={(e) => setTeamName(e.target.value)}
            placeholder="SV Beispiel I"
            disabled={busy}
          />
        </label>
        <button type="submit" disabled={busy || !teamName.trim()}>
          Anlegen
        </button>
      </form>

      <label className="check-field">
        <input
          type="checkbox"
          checked={includeArchived}
          onChange={(e) => onIncludeArchivedChange(e.target.checked)}
        />
        Archiv anzeigen
      </label>

      <ul className="entity-list">
        {teams.map((t) => {
          const archived = Boolean(t.archived);
          const memberCount = (t.memberPersonIds ?? []).length;
          const isActive = t.id === selectedId;
          return (
            <li key={t.id}>
              <button
                type="button"
                className={isActive ? "list-item active" : "list-item"}
                onClick={() => setSelectedId(t.id)}
              >
                <span className="list-title">
                  {t.name}
                  {archived ? " · Archiv" : ""}
                </span>
                <span className="list-meta">
                  {memberCount === 0
                    ? "Keine Mitglieder"
                    : `${memberCount} ${memberCount === 1 ? "Mitglied" : "Mitglieder"}`}
                </span>
              </button>
            </li>
          );
        })}
        {teams.length === 0 ? (
          <li className="empty-soft">Noch keine Teams — Name eingeben und anlegen.</li>
        ) : null}
      </ul>

      {selected ? (
        <div className="team-master-detail">
          {renamingId === selected.id ? (
            <form className="clone-row" onSubmit={(e) => void submitRename(e)}>
              <label className="field grow">
                Umbenennen
                <input
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  disabled={busy}
                  autoFocus
                />
              </label>
              <button type="submit" disabled={busy || !renameValue.trim()}>
                Speichern
              </button>
              <button
                type="button"
                className="secondary"
                disabled={busy}
                onClick={() => setRenamingId(null)}
              >
                Abbrechen
              </button>
            </form>
          ) : (
            <div className="status-row team-master-actions">
              <strong>{selected.name}</strong>
              {selected.archived ? (
                <button
                  type="button"
                  className="secondary"
                  disabled={busy}
                  onClick={() => void onSetArchived(selected.id, false)}
                >
                  Wiederherstellen
                </button>
              ) : (
                <>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => startRename(selected)}
                  >
                    Umbenennen
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onSetArchived(selected.id, true)}
                  >
                    Archivieren
                  </button>
                </>
              )}
              <button
                type="button"
                className="ghost danger-text"
                disabled={busy}
                onClick={() => confirmDelete(selected)}
              >
                Endgültig löschen
              </button>
            </div>
          )}

          {!selected.archived ? (
            <>
              <form className="clone-row" onSubmit={(e) => void onAssign(e)}>
                <label className="field grow">
                  Schütze zuweisen
                  <SearchSelect
                    value={assignPersonId}
                    options={assignOptions}
                    onChange={setAssignPersonId}
                    disabled={busy}
                    placeholder="Schütze wählen…"
                    allowClear
                  />
                </label>
                <button type="submit" disabled={busy || !assignPersonId}>
                  Zuweisen
                </button>
              </form>

              <ul className="team-members">
                {(selected.memberPersonIds ?? []).map((pid) => {
                  const person = peopleById.get(pid);
                  return (
                    <li key={pid} className="team-member">
                      <span>
                        {person
                          ? `${formatPersonName(person.lastName, person.firstName)}${
                              person.club?.trim() ? ` · ${person.club.trim()}` : ""
                            }`
                          : pid}
                      </span>
                      <button
                        type="button"
                        className="ghost"
                        disabled={busy}
                        onClick={() => void onRemovePerson(selected.id, pid)}
                      >
                        ×
                      </button>
                    </li>
                  );
                })}
                {(selected.memberPersonIds ?? []).length === 0 ? (
                  <li className="empty-soft">Noch keine Mitglieder — Schütze oben zuweisen.</li>
                ) : null}
              </ul>
            </>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
