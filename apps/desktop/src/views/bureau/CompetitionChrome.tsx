import { useEffect, useMemo, useRef, useState } from "react";
import type {
  Competition,
  CompetitionKind,
  CompetitionStatus,
} from "@reddot/domain";
import { ExpandSlot } from "../../components/ExpandSlot";
import { OverflowMenu } from "../../components/OverflowMenu";
import {
  CompetitionCreateForm,
  type CompetitionCreateInput,
} from "./CompetitionCreateForm";
import { KIND_LABEL, STATUS_LABEL, competitionListMeta } from "./labels";

type Props = {
  competitions: Competition[];
  selected: Competition | null;
  selectedId: string | null;
  busy?: boolean;
  includeArchived: boolean;
  onIncludeArchivedChange: (value: boolean) => void;
  onSelect: (id: string | null) => void;
  onCreate: (input: CompetitionCreateInput) => Promise<boolean>;
  onUpdate: (id: string, input: CompetitionCreateInput) => Promise<boolean>;
  onSetStatus: (status: CompetitionStatus) => Promise<boolean>;
  onSaveAsTemplate: () => Promise<boolean>;
  onCreateFromTemplate: (sourceId: string) => Promise<boolean>;
};

type FormMode = "hidden" | "create" | "edit";

export function CompetitionChrome({
  competitions,
  selected,
  selectedId,
  busy = false,
  includeArchived,
  onIncludeArchivedChange,
  onSelect,
  onCreate,
  onUpdate,
  onSetStatus,
  onSaveAsTemplate,
  onCreateFromTemplate,
}: Props) {
  const [formMode, setFormMode] = useState<FormMode>("hidden");
  /** Last open mode — kept during close animation so content does not flicker. */
  const [formKind, setFormKind] = useState<"create" | "edit">("create");
  const [pickerOpen, setPickerOpen] = useState(false);
  const pickerRef = useRef<HTMLDivElement>(null);

  const { regular, templates } = useMemo(() => {
    const regular: Competition[] = [];
    const templates: Competition[] = [];
    for (const c of competitions) {
      if (c.status === "template") templates.push(c);
      else regular.push(c);
    }
    return { regular, templates };
  }, [competitions]);

  const isArchived = selected?.status === "archived";
  const isTemplate = selected?.status === "template";
  const selectedKind: CompetitionKind =
    selected?.kind === "training" ? "training" : "competition";
  const canEdit = Boolean(selected && !isTemplate && !isArchived);

  useEffect(() => {
    if (formMode === "edit" && !canEdit) setFormMode("hidden");
  }, [formMode, canEdit]);

  useEffect(() => {
    if (!pickerOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (!pickerRef.current?.contains(e.target as Node)) {
        setPickerOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setPickerOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [pickerOpen]);

  const openCreate = () => {
    setPickerOpen(false);
    setFormKind("create");
    setFormMode("create");
  };

  const openEdit = () => {
    if (!canEdit) return;
    setPickerOpen(false);
    setFormKind("edit");
    setFormMode("edit");
  };

  const closeForm = () => setFormMode("hidden");

  const togglePicker = () => {
    setPickerOpen((o) => {
      if (!o) setFormMode("hidden");
      return !o;
    });
  };

  const handleCreate = async (input: CompetitionCreateInput) => {
    const ok = await onCreate(input);
    if (ok) setFormMode("hidden");
    return ok;
  };

  const handleUpdate = async (id: string, input: CompetitionCreateInput) => {
    const ok = await onUpdate(id, input);
    if (ok) setFormMode("hidden");
    return ok;
  };

  const handleSelect = (id: string) => {
    onSelect(id);
    setPickerOpen(false);
    if (formMode === "create") setFormMode("hidden");
  };

  const statusLabel = selected
    ? (STATUS_LABEL[selected.status as keyof typeof STATUS_LABEL] ??
      selected.status)
    : null;

  return (
    <div className="competition-chrome">
      <div className="competition-chrome-bar">
        <div
          className={`competition-chrome-title${pickerOpen ? " is-picking" : ""}`}
          ref={pickerRef}
        >
          <button
            type="button"
            className="competition-chrome-pick"
            disabled={busy}
            aria-expanded={pickerOpen}
            aria-haspopup="listbox"
            onClick={togglePicker}
          >
            <span className="competition-chrome-name">
              {selected?.name ?? "Wettkampf wählen"}
            </span>
            {selected ? (
              <span className="competition-chrome-meta">
                {competitionListMeta(selected, { omitStatus: true })}
              </span>
            ) : (
              <span className="competition-chrome-meta">
                Wählen oder neu anlegen
              </span>
            )}
            <span
              className={`competition-chrome-caret${pickerOpen ? " is-open" : ""}`}
              aria-hidden
            >
              ▾
            </span>
          </button>

          {pickerOpen ? (
            <div className="competition-picker" role="listbox">
              <div className="competition-picker-head">
                <span className="list-meta">Wettkämpfe</span>
                <label className="check-field competition-picker-archiv">
                  <input
                    type="checkbox"
                    checked={includeArchived}
                    onChange={(e) => onIncludeArchivedChange(e.target.checked)}
                  />
                  Archiv
                </label>
              </div>
              <ul className="entity-list competition-picker-list">
                {regular.map((c) => (
                  <li key={c.id}>
                    <button
                      type="button"
                      role="option"
                      aria-selected={c.id === selectedId}
                      className={
                        c.id === selectedId ? "list-item active" : "list-item"
                      }
                      onClick={() => handleSelect(c.id)}
                    >
                      <span className="competition-picker-row-top">
                        <span className="list-title">{c.name}</span>
                        <span
                          className={`competition-status-chip status-${c.status}`}
                        >
                          {STATUS_LABEL[c.status as keyof typeof STATUS_LABEL] ??
                            c.status}
                        </span>
                      </span>
                      <span className="list-meta">
                        {competitionListMeta(c, { omitStatus: true })}
                      </span>
                    </button>
                  </li>
                ))}
                {regular.length === 0 ? (
                  <li className="empty-soft">Noch keine Wettkämpfe.</li>
                ) : null}
              </ul>

              {templates.length > 0 ? (
                <>
                  <h3 className="subhead">Vorlagen</h3>
                  <ul className="entity-list compact competition-picker-list">
                    {templates.map((c) => (
                      <li key={c.id}>
                        <div
                          className={`template-row${c.id === selectedId ? " is-active" : ""}`}
                        >
                          <button
                            type="button"
                            className="template-pick"
                            onClick={() => handleSelect(c.id)}
                          >
                            <span className="list-title">{c.name}</span>
                            <span className="list-meta">
                              {c.discipline} · {c.maxShots} Schüsse
                              {c.kind === "training"
                                ? ` · ${KIND_LABEL.training}`
                                : ""}
                            </span>
                          </button>
                          <button
                            type="button"
                            disabled={busy}
                            title="Wettkampf aus Vorlage anlegen"
                            onClick={() => {
                              setPickerOpen(false);
                              void onCreateFromTemplate(c.id);
                            }}
                          >
                            Anlegen
                          </button>
                        </div>
                      </li>
                    ))}
                  </ul>
                </>
              ) : null}
            </div>
          ) : null}
        </div>

        <div className="competition-chrome-actions">
          {selected && !isTemplate && !isArchived ? (
            <div
              className="competition-status-seg"
              role="group"
              aria-label="Wettkampf-Status"
            >
              <button
                type="button"
                className={`competition-status-chip status-draft${selected.status === "draft" ? " is-current" : ""}`}
                disabled={busy || selected.status === "draft"}
                title="Entwurf: Startliste vorbereiten"
                onClick={() => void onSetStatus("draft")}
              >
                Entwurf
              </button>
              <button
                type="button"
                className={`competition-status-chip status-active${selected.status === "active" ? " is-current" : ""}`}
                disabled={busy || selected.status === "active"}
                title="Aktiv: Schießen in der Arena möglich"
                onClick={() => void onSetStatus("active")}
              >
                Aktiv
              </button>
              {selected.nachkaufEnabled ? (
                <span
                  className="competition-status-chip status-nachkauf is-current"
                  title="Nachkauf erlaubt — fertige Starter können erneut starten"
                >
                  Nachkauf
                </span>
              ) : null}
              <button
                type="button"
                className={`competition-status-chip status-closed${selected.status === "closed" ? " is-current" : ""}`}
                disabled={busy || selected.status === "closed"}
                title="Geschlossen: Startliste gesperrt, Ergebnisse bleiben"
                onClick={() => void onSetStatus("closed")}
              >
                Geschlossen
              </button>
            </div>
          ) : selected && statusLabel ? (
            <span
              className={`competition-status-chip status-${selected.status} is-current`}
              title={KIND_LABEL[selectedKind]}
            >
              {statusLabel}
            </span>
          ) : null}
          {formMode === "hidden" ? (
            <>
              <button type="button" disabled={busy} onClick={openCreate}>
                Neu
              </button>
              {canEdit ? (
                <button
                  type="button"
                  className="secondary"
                  disabled={busy}
                  onClick={openEdit}
                >
                  Bearbeiten
                </button>
              ) : null}
              {selected && !isTemplate && !isArchived ? (
                <OverflowMenu
                  label="Mehr"
                  ariaLabel="Weitere Wettkampf-Aktionen"
                  className="competition-more"
                  menuClassName="competition-more-menu"
                  disabled={busy}
                >
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onSetStatus("archived")}
                  >
                    Archivieren
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    onClick={() => void onSaveAsTemplate()}
                  >
                    Als Vorlage
                  </button>
                </OverflowMenu>
              ) : null}
              {isArchived && selected ? (
                <button
                  type="button"
                  className="secondary"
                  disabled={busy}
                  onClick={() => void onSetStatus("closed")}
                >
                  Wiederherstellen
                </button>
              ) : null}
              {isTemplate && selected ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void onCreateFromTemplate(selected.id)}
                >
                  Wettkampf anlegen
                </button>
              ) : null}
            </>
          ) : null}
        </div>
      </div>

      <ExpandSlot open={formMode !== "hidden"} className="competition-form-slot">
        <div className="bureau-inline-form competition-chrome-form">
          <div className="form-edit-head">
            <span className="list-meta">
              {formKind === "edit"
                ? `Bearbeiten: ${selected?.name ?? ""}`
                : "Neuer Wettkampf"}
            </span>
            <button
              type="button"
              className="ghost"
              disabled={busy}
              onClick={closeForm}
            >
              Schließen
            </button>
          </div>
          <CompetitionCreateForm
            key={formKind === "edit" ? selected?.id ?? "edit" : "create"}
            busy={busy}
            editing={formKind === "edit" ? selected : null}
            onCreate={handleCreate}
            onUpdate={handleUpdate}
          />
        </div>
      </ExpandSlot>
    </div>
  );
}
