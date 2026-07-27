import { useEffect, useMemo, useState } from "react";
import type {
  Competition,
  CompetitionEntry,
  CompetitionTeam,
} from "@rotpunktarena/domain";
import {
  ShooterAutocomplete,
  type ShooterValue,
} from "../../components/ShooterAutocomplete";
import { SearchSelect } from "../../components/SearchSelect";
import { formatPersonName } from "../../lib/format";
import {
  CompetitionCreateForm,
  type CompetitionCreateInput,
} from "../bureau/CompetitionCreateForm";

type Props = {
  competitions: Competition[];
  competitionId: string;
  onCompetitionIdChange: (id: string) => void;
  entries: CompetitionEntry[];
  entryId: string;
  onEntryIdChange: (id: string) => void;
  teams: CompetitionTeam[];
  teamId: string;
  onTeamIdChange: (id: string) => void;
  selectedComp: Competition | undefined;
  running: boolean;
  createBusy?: boolean;
  onCreateCompetition: (input: CompetitionCreateInput) => Promise<boolean>;
  /** Select or add shooter as active starter (auto-login). */
  onEnsureStarter: (shooter: ShooterValue) => Promise<boolean>;
  /** Create or select a global team. */
  onEnsureTeam: (name: string) => Promise<boolean>;
};

function competitionLabel(c: Competition): string {
  const bits = [
    c.kind === "training" ? "T" : null,
    c.name,
    c.nachkaufEnabled ? "NK" : null,
    c.teamScoringEnabled ? "Teams" : null,
  ].filter(Boolean);
  return bits.join(" · ");
}

function entryToShooter(e: CompetitionEntry | undefined): ShooterValue {
  if (!e) return { name: "", personId: null };
  return {
    name: formatPersonName(e.lastName, e.firstName, ""),
    personId: e.personId,
  };
}

export function LiveCompetitionPicker({
  competitions,
  competitionId,
  onCompetitionIdChange,
  entries,
  entryId,
  onEntryIdChange: _onEntryIdChange,
  teams,
  teamId,
  onTeamIdChange,
  selectedComp,
  running,
  createBusy = false,
  onCreateCompetition,
  onEnsureStarter,
  onEnsureTeam,
}: Props) {
  const [creating, setCreating] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createKey, setCreateKey] = useState(0);
  const [compDraft, setCompDraft] = useState("");
  const [teamDraft, setTeamDraft] = useState("");
  const [teamBusy, setTeamBusy] = useState(false);
  const [starterDraft, setStarterDraft] = useState<ShooterValue>(() =>
    entryToShooter(entries.find((e) => e.id === entryId)),
  );
  const [starterBusy, setStarterBusy] = useState(false);
  const [starterError, setStarterError] = useState<string | null>(null);

  const teamMode = Boolean(selectedComp?.teamScoringEnabled);
  const selectedEntry = entries.find((e) => e.id === entryId);

  const competitionOptions = useMemo(
    () =>
      competitions
        .filter((c) => c.status === "active")
        .map((c) => ({
          id: c.id,
          label: competitionLabel(c),
          matchText: c.name,
        })),
    [competitions],
  );

  const teamOptions = useMemo(
    () =>
      teams.map((t) => ({
        id: t.id,
        label: t.name,
        matchText: t.name,
      })),
    [teams],
  );

  useEffect(() => {
    if (starterBusy) return;
    setStarterDraft(entryToShooter(selectedEntry));
    setStarterError(null);
  }, [entryId, selectedEntry, starterBusy]);

  const starterDisabled = running || !competitionId || starterBusy;
  const starterPlaceholder = !competitionId
    ? ""
    : teamMode && !teamId
      ? "Zuerst Team wählen…"
      : "Starter wählen…";

  const handleCreate = async (input: CompetitionCreateInput) => {
    const ok = await onCreateCompetition(input);
    if (ok) {
      setCreating(false);
      setCompDraft("");
    }
    return ok;
  };

  const toggleCreate = () => {
    setCreating((open) => {
      if (open) return false;
      setCreateName(compDraft.trim());
      setCreateKey((k) => k + 1);
      return true;
    });
  };

  const createTeamFromDraft = async () => {
    const name = teamDraft.trim();
    if (!name || teamBusy) return;
    setTeamBusy(true);
    try {
      await onEnsureTeam(name);
    } finally {
      setTeamBusy(false);
    }
  };

  const commitStarter = async (next: ShooterValue) => {
    if (!next.personId || starterBusy || running || !competitionId) return;
    if (teamMode && !teamId) return;
    setStarterBusy(true);
    setStarterError(null);
    try {
      const ok = await onEnsureStarter(next);
      if (!ok) {
        setStarterDraft(entryToShooter(selectedEntry));
      }
    } catch (e) {
      setStarterError(String(e));
      setStarterDraft(entryToShooter(selectedEntry));
    } finally {
      setStarterBusy(false);
    }
  };

  const handleStarterChange = (next: ShooterValue) => {
    setStarterDraft(next);
    setStarterError(null);
    if (next.personId) void commitStarter(next);
  };

  return (
    <div className={`live-comp-cluster${creating ? " is-creating" : ""}`}>
      <label className="field field-compact">
        {selectedComp?.kind === "training" ? "Trainingswettkampf" : "Wettkampf"}
        <SearchSelect
          value={competitionId}
          options={competitionOptions}
          onChange={(id) => {
            onCompetitionIdChange(id);
            setCreating(false);
            setStarterError(null);
          }}
          disabled={running}
          placeholder={competitions.length === 0 ? "Wettkampf tippen…" : "Wettkampf wählen…"}
          allowCreate={!running}
          createBusy={createBusy}
          createExpanded={creating}
          onDraftChange={setCompDraft}
          onCreateClick={toggleCreate}
        />
      </label>

      {teamMode ? (
        <label className="field field-compact">
          Team
          <SearchSelect
            value={teamId}
            options={teamOptions}
            onChange={onTeamIdChange}
            disabled={running || teamBusy}
            placeholder={teams.length === 0 ? "Team tippen…" : "Team wählen…"}
            allowClear={false}
            allowCreate={!running}
            createBusy={teamBusy}
            onDraftChange={setTeamDraft}
            onCreateClick={() => void createTeamFromDraft()}
          />
        </label>
      ) : null}

      <label className="field field-compact">
        Starter
        <div className="field-action-row">
          <ShooterAutocomplete
            value={starterDraft}
            onChange={handleStarterChange}
            disabled={starterDisabled || (teamMode && !teamId)}
            allowPromote
            placeholder={starterPlaceholder}
          />
          {starterError ? (
            <span className="shooter-ac-error" title={starterError}>
              !
            </span>
          ) : null}
        </div>
      </label>

      {creating && !running ? (
        <div className="live-create-panel" role="dialog" aria-label="Neuen Wettkampf anlegen">
          <div className="live-create-panel-head">
            <strong>Neuer Wettkampf</strong>
            <button
              type="button"
              className="ghost"
              disabled={createBusy}
              onClick={() => setCreating(false)}
            >
              Schließen
            </button>
          </div>
          <CompetitionCreateForm
            key={createKey}
            busy={createBusy}
            allowTrainingKind={false}
            defaultActivateOnCreate
            initialName={createName}
            onCreate={handleCreate}
          />
        </div>
      ) : null}
    </div>
  );
}
