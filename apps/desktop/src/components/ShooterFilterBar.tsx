import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { TrainingShooterOption } from "@rotpunktarena/domain";
import type { LeagueRank } from "../training/league";
import { LeagueBadge } from "./LeagueBadge";

export type ShooterFilterKey = "all" | string;

type Props = {
  shooters: TrainingShooterOption[];
  filter: ShooterFilterKey;
  onChange: (next: ShooterFilterKey) => void;
  filterKeyOf: (o: { personId?: string | null; shooterName: string }) => ShooterFilterKey;
  /** Training league mark next to each name (training view only). */
  leagueOf?: (key: ShooterFilterKey) => LeagueRank | undefined;
};

/** Searchable shooter filter — scales cleanly past dozens of names. */
export function ShooterFilterBar({
  shooters,
  filter,
  onChange,
  filterKeyOf,
  leagueOf,
}: Props) {
  const listId = useId();
  const wrapRef = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);

  const sorted = useMemo(
    () => [...shooters].sort((a, b) => b.sessionCount - a.sessionCount),
    [shooters],
  );

  const selected = shooters.find((s) => filterKeyOf(s) === filter);
  const selectedLeague = filter !== "all" ? leagueOf?.(filter) : undefined;
  const triggerLabel =
    filter === "all" ? "Alle Schützen" : (selected?.shooterName ?? "Schütze");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return sorted;
    return sorted.filter((s) => s.shooterName.toLowerCase().includes(q));
  }, [sorted, query]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (!wrapRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const pick = (key: ShooterFilterKey) => {
    onChange(key);
    setOpen(false);
    setQuery("");
  };

  return (
    <div className="shooter-filter" role="group" aria-label="Schütze filtern">
      <button
        type="button"
        className={filter === "all" ? "chip chip-on" : "chip"}
        onClick={() => pick("all")}
      >
        Alle
      </button>

      <div className="shooter-picker" ref={wrapRef}>
        <button
          type="button"
          className={
            filter !== "all"
              ? "shooter-picker-trigger is-on"
              : open
                ? "shooter-picker-trigger is-open"
                : "shooter-picker-trigger"
          }
          aria-expanded={open}
          aria-controls={listId}
          aria-haspopup="listbox"
          onClick={() => setOpen((v) => !v)}
        >
          <span className="shooter-picker-trigger-label">
            {selectedLeague ? <LeagueBadge rank={selectedLeague} size="sm" /> : null}
            {triggerLabel}
          </span>
          <span className="shooter-picker-trigger-meta">
            {shooters.length} · wählen
          </span>
        </button>
        {open ? (
          <div className="shooter-picker-pop" id={listId} role="listbox">
            <input
              type="search"
              className="shooter-picker-input"
              placeholder="Name suchen…"
              value={query}
              autoFocus
              onChange={(e) => setQuery(e.target.value)}
              aria-label="Schütze suchen"
            />
            <ul className="shooter-picker-list">
              <li>
                <button
                  type="button"
                  role="option"
                  aria-selected={filter === "all"}
                  className={
                    filter === "all"
                      ? "shooter-picker-item is-on"
                      : "shooter-picker-item"
                  }
                  onClick={() => pick("all")}
                >
                  <span>Alle Schützen</span>
                </button>
              </li>
              {filtered.length === 0 ? (
                <li className="shooter-picker-empty">Kein Treffer</li>
              ) : (
                filtered.map((s) => {
                  const key = filterKeyOf(s);
                  const league = leagueOf?.(key);
                  return (
                    <li key={key}>
                      <button
                        type="button"
                        role="option"
                        aria-selected={filter === key}
                        className={
                          filter === key
                            ? "shooter-picker-item is-on"
                            : "shooter-picker-item"
                        }
                        onClick={() => pick(key)}
                      >
                        <span className="shooter-picker-name">
                          {league ? <LeagueBadge rank={league} size="sm" /> : null}
                          {s.shooterName}
                        </span>
                        <span className="shooter-picker-meta">{s.sessionCount}</span>
                      </button>
                    </li>
                  );
                })
              )}
            </ul>
          </div>
        ) : null}
      </div>
    </div>
  );
}
