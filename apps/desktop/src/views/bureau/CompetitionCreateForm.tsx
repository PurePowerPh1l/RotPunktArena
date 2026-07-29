import { useEffect, useState, type FormEvent } from "react";
import type { Competition, CompetitionKind, ScoringMode } from "@rotpunktarena/domain";
import { SearchSelect } from "../../components/SearchSelect";

export type CompetitionCreateInput = {
  name: string;
  date: string;
  discipline: string;
  maxShots: number;
  scoringMode: ScoringMode;
  nachkaufEnabled: boolean;
  nachkaufShots: number;
  teamScoringEnabled: boolean;
  teamCount: number;
  kind: CompetitionKind;
  /** Zehntelwertung; default false = ganze Ringe. */
  tenthsEnabled: boolean;
  /** After create, set status to active (UI-only; not sent to create_competition). */
  activateOnCreate: boolean;
};

type Props = {
  busy?: boolean;
  /** When false, hide Trainingswettkampf option (quick-create). Default true. */
  allowTrainingKind?: boolean;
  /** Prefill name when opening a blank create form (e.g. Arena draft). */
  initialName?: string;
  /** Default for „Sofort aktiv“ on create. Arena: true; Verwaltung: false. */
  defaultActivateOnCreate?: boolean;
  /** When set, form edits this competition (Speichern) instead of creating. */
  editing?: Competition | null;
  onCreate: (input: CompetitionCreateInput) => Promise<boolean>;
  onUpdate?: (id: string, input: CompetitionCreateInput) => Promise<boolean>;
};

function defaultsFrom(editing?: Competition | null, initialName = "") {
  if (!editing) {
    return {
      compName: initialName,
      compDate: new Date().toISOString().slice(0, 10),
      discipline: "Luftgewehr",
      maxShots: 40,
      scoringMode: "ringe" as ScoringMode,
      nachkaufEnabled: false,
      teamScoringEnabled: false,
      teamCount: 3,
      kind: "competition" as CompetitionKind,
      tenthsEnabled: false,
    };
  }
  return {
    compName: editing.name,
    compDate: editing.date,
    discipline: editing.discipline,
    maxShots: editing.maxShots,
    scoringMode: (editing.scoringMode === "teiler" ? "teiler" : "ringe") as ScoringMode,
    nachkaufEnabled: Boolean(editing.nachkaufEnabled),
    teamScoringEnabled: Boolean(editing.teamScoringEnabled),
    teamCount: editing.teamCount ?? 3,
    kind: (editing.kind === "training" ? "training" : "competition") as CompetitionKind,
    tenthsEnabled: Boolean(editing.tenthsEnabled),
  };
}

export function CompetitionCreateForm({
  busy = false,
  allowTrainingKind = true,
  initialName = "",
  defaultActivateOnCreate = false,
  editing = null,
  onCreate,
  onUpdate,
}: Props) {
  const initial = defaultsFrom(editing, initialName);
  const [compName, setCompName] = useState(initial.compName);
  const [compDate, setCompDate] = useState(initial.compDate);
  const [discipline, setDiscipline] = useState(initial.discipline);
  const [maxShots, setMaxShots] = useState(initial.maxShots);
  const [scoringMode, setScoringMode] = useState<ScoringMode>(initial.scoringMode);
  const [nachkaufEnabled, setNachkaufEnabled] = useState(initial.nachkaufEnabled);
  const [tenthsEnabled, setTenthsEnabled] = useState(initial.tenthsEnabled);
  const [teamScoringEnabled, setTeamScoringEnabled] = useState(
    initial.teamScoringEnabled,
  );
  const [teamCount, setTeamCount] = useState(initial.teamCount);
  const [kind, setKind] = useState<CompetitionKind>(initial.kind);
  const [activateOnCreate, setActivateOnCreate] = useState(defaultActivateOnCreate);

  const editingId = editing?.id ?? null;

  useEffect(() => {
    if (!editing) return;
    const next = defaultsFrom(editing);
    setCompName(next.compName);
    setCompDate(next.compDate);
    setDiscipline(next.discipline);
    setMaxShots(next.maxShots);
    setScoringMode(next.scoringMode);
    setNachkaufEnabled(next.nachkaufEnabled);
    setTenthsEnabled(next.tenthsEnabled);
    setTeamScoringEnabled(next.teamScoringEnabled);
    setTeamCount(next.teamCount);
    setKind(next.kind);
  }, [
    editingId,
    editing?.name,
    editing?.date,
    editing?.discipline,
    editing?.maxShots,
    editing?.scoringMode,
    editing?.nachkaufEnabled,
    editing?.tenthsEnabled,
    editing?.teamScoringEnabled,
    editing?.teamCount,
    editing?.kind,
  ]);

  const setAsTraining = (checked: boolean) => {
    const next: CompetitionKind = checked ? "training" : "competition";
    setKind(next);
    if (checked) {
      setMaxShots((n) => (n === 40 ? 10 : n));
      setCompName((n) => (n.trim() ? n : "Trainingswettkampf"));
    }
  };

  const buildInput = (): CompetitionCreateInput => ({
    name: compName,
    date: compDate,
    discipline,
    maxShots,
    scoringMode,
    nachkaufEnabled,
    nachkaufShots: 0,
    teamScoringEnabled,
    teamCount: teamScoringEnabled ? teamCount : 3,
    kind: allowTrainingKind ? kind : "competition",
    tenthsEnabled,
    activateOnCreate: editingId ? false : activateOnCreate,
  });

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (busy) return;
    const input = buildInput();
    if (editingId && onUpdate) {
      await onUpdate(editingId, input);
      return;
    }
    const ok = await onCreate(input);
    if (!ok) return;
    setCompName("");
    setKind("competition");
    setMaxShots(40);
    setNachkaufEnabled(false);
    setTenthsEnabled(false);
    setTeamScoringEnabled(false);
    setTeamCount(3);
    setActivateOnCreate(defaultActivateOnCreate);
  };

  return (
    <form className="stack-form" onSubmit={(e) => void onSubmit(e)}>
      <label className="field">
        Name
        <input
          value={compName}
          onChange={(e) => setCompName(e.target.value)}
          placeholder="Vereinsmeisterschaft"
          required
        />
      </label>
      <div className="row-2">
        <label className="field">
          Datum
          <input
            type="date"
            value={compDate}
            onChange={(e) => setCompDate(e.target.value)}
          />
        </label>
        <label className="field">
          Disziplin
          <input
            value={discipline}
            onChange={(e) => setDiscipline(e.target.value)}
          />
        </label>
      </div>
      <div className="row-2">
        <label className="field">
          Max. Schüsse
          <input
            type="number"
            min={1}
            value={maxShots}
            onChange={(e) => setMaxShots(Number(e.target.value) || 40)}
          />
        </label>
        <label className="field">
          Wertung
          <SearchSelect
            value={scoringMode}
            options={[
              { id: "ringe", label: "Ringe / Punkte" },
              { id: "teiler", label: "Teiler" },
            ]}
            onChange={(id) => setScoringMode(id as ScoringMode)}
            placeholder="Wertung…"
            allowClear={false}
          />
        </label>
      </div>
      <label
        className="check-field"
        title="Punkte als Zehntel (10,5) statt ganze Ringe (10) werten"
      >
        <input
          type="checkbox"
          checked={tenthsEnabled}
          onChange={(e) => setTenthsEnabled(e.target.checked)}
        />
        <span className="check-field-copy">
          Zehntelwertung
          <span className="field-hint">sonst ganze Ringe</span>
        </span>
      </label>
      {allowTrainingKind ? (
        <label
          className="check-field"
          title="Eigener Wettkampf-Typ mit Startliste für Trainingsbetrieb"
        >
          <input
            type="checkbox"
            checked={kind === "training"}
            onChange={(e) => setAsTraining(e.target.checked)}
          />
          <span className="check-field-copy">
            Trainingswettkampf
            <span className="field-hint">mit Startliste, kürzere Serie</span>
          </span>
        </label>
      ) : null}
      <label
        className="check-field"
        title="Nach Fertig darf eine weitere Serie gestartet werden"
      >
        <input
          type="checkbox"
          checked={nachkaufEnabled}
          onChange={(e) => setNachkaufEnabled(e.target.checked)}
        />
        <span className="check-field-copy">
          Nachkauf erlaubt
          <span className="field-hint">weitere Serie nach Fertig</span>
        </span>
      </label>
      <label
        className="check-field"
        title="Teamwertung aktivieren — Zuweisung in der Startliste"
      >
        <input
          type="checkbox"
          checked={teamScoringEnabled}
          onChange={(e) => setTeamScoringEnabled(e.target.checked)}
        />
        <span className="check-field-copy">
          Teamwertung
          <span className="field-hint">Zuweisung in der Startliste</span>
        </span>
      </label>
      {!editingId ? (
        <label
          className="check-field"
          title="Status sofort auf Aktiv setzen — in der Arena auswählbar"
        >
          <input
            type="checkbox"
            checked={activateOnCreate}
            onChange={(e) => setActivateOnCreate(e.target.checked)}
          />
          <span className="check-field-copy">
            Sofort aktiv
            <span className="field-hint">in der Arena auswählbar</span>
          </span>
        </label>
      ) : null}
      {teamScoringEnabled ? (
        <label className="field">
          Wertende Schützen pro Team
          <input
            type="number"
            min={1}
            max={20}
            value={teamCount}
            onChange={(e) => setTeamCount(Math.max(1, Number(e.target.value) || 3))}
          />
          <span className="field-hint">Beste N Ergebnisse zählen</span>
        </label>
      ) : null}
      <button type="submit" disabled={busy}>
        {editingId && onUpdate
          ? "Änderungen speichern"
          : allowTrainingKind && kind === "training"
            ? "Trainingswettkampf anlegen"
            : "Wettkampf anlegen"}
      </button>
    </form>
  );
}
