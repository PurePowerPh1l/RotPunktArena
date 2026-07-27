import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import type { Person } from "@rotpunktarena/domain";
import * as api from "../api/commands";
import { createRequestSeq } from "../lib/requestSeq";

export type ShooterValue = {
  name: string;
  personId: string | null;
};

type Props = {
  value: ShooterValue;
  onChange: (next: ShooterValue) => void;
  disabled?: boolean;
  placeholder?: string;
  /** Show „Anlegen“ for unknown names. Default true. */
  allowPromote?: boolean;
  /**
   * When set, only these people appear in suggestions (e.g. team members).
   * Empty array → no suggestions.
   */
  allowedPersonIds?: string[] | null;
};

function personLabel(p: Person): string {
  const base = `${p.lastName}, ${p.firstName}`;
  return p.club ? `${base} · ${p.club}` : base;
}

function displayName(p: Person): string {
  if (p.lastName === "—") return p.firstName.trim();
  return `${p.firstName} ${p.lastName}`.trim();
}

type ListPos = {
  top: number;
  left: number;
  width: number;
  maxHeight: number;
  placement: "down" | "up";
};

/** Free-text shooter field with people-DB autocomplete (fixed portal, flips near bottom). */
export function ShooterAutocomplete({
  value,
  onChange,
  disabled,
  placeholder = "Name oder aus Liste wählen…",
  allowPromote = true,
  allowedPersonIds = null,
}: Props) {
  const listId = useId();
  const wrapRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const [open, setOpen] = useState(false);
  const [suggestions, setSuggestions] = useState<Person[]>([]);
  const [highlight, setHighlight] = useState(0);
  const [pos, setPos] = useState<ListPos | null>(null);
  const [promoting, setPromoting] = useState(false);
  const [promoteError, setPromoteError] = useState<string | null>(null);
  const suggestSeq = useRef(createRequestSeq()).current;

  useEffect(() => {
    if (disabled) {
      suggestSeq.begin();
      setSuggestions([]);
      setOpen(false);
      return;
    }
    const allowed =
      allowedPersonIds == null
        ? null
        : new Set(allowedPersonIds);
    if (allowed && allowed.size === 0) {
      suggestSeq.begin();
      setSuggestions([]);
      return;
    }
    // Linked selection keeps the display name in the input; that full-name
    // string rarely matches first/last alone, so browse unfiltered until edit.
    const q = value.personId ? undefined : value.name.trim() || undefined;
    const t = window.setTimeout(() => {
      const token = suggestSeq.begin();
      void api
        .listPeople(q)
        .then((list) => {
          if (!suggestSeq.isCurrent(token)) return;
          const filtered = allowed
            ? list.filter((p) => allowed.has(p.id))
            : list;
          setSuggestions(filtered.slice(0, 8));
          setHighlight(0);
        })
        .catch(() => {
          if (suggestSeq.isCurrent(token)) setSuggestions([]);
        });
    }, 160);
    return () => window.clearTimeout(t);
  }, [value.name, value.personId, disabled, suggestSeq, allowedPersonIds]);

  const showList = open && !disabled && suggestions.length > 0;
  const canPromote =
    allowPromote &&
    allowedPersonIds == null &&
    !disabled &&
    !value.personId &&
    value.name.trim().length > 0 &&
    !promoting;

  const updatePos = () => {
    const el = inputRef.current ?? wrapRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const gap = 4;
    const preferred = Math.min(224, suggestions.length * 40 + 12);
    const spaceBelow = window.innerHeight - rect.bottom - gap - 8;
    const spaceAbove = rect.top - gap - 8;
    const placeUp = spaceBelow < preferred && spaceAbove > spaceBelow;
    const maxHeight = Math.max(80, Math.min(preferred, placeUp ? spaceAbove : spaceBelow));
    setPos({
      left: rect.left,
      width: Math.max(rect.width, 160),
      maxHeight,
      placement: placeUp ? "up" : "down",
      top: placeUp ? rect.top - gap - maxHeight : rect.bottom + gap,
    });
  };

  useLayoutEffect(() => {
    if (!showList) {
      setPos(null);
      return;
    }
    updatePos();
    const onMove = () => updatePos();
    window.addEventListener("resize", onMove);
    window.addEventListener("scroll", onMove, true);
    return () => {
      window.removeEventListener("resize", onMove);
      window.removeEventListener("scroll", onMove, true);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- recompute when list opens / suggestions change
  }, [showList, suggestions.length]);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      const t = e.target as Node;
      if (wrapRef.current?.contains(t) || listRef.current?.contains(t)) return;
      setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  const pick = (p: Person) => {
    onChange({ name: displayName(p), personId: p.id });
    setOpen(false);
    setPromoteError(null);
  };

  const onInput = (text: string) => {
    onChange({ name: text, personId: null });
    // Clearing the field closes the list; typing opens/filters it.
    setOpen(text.trim().length > 0);
    setPromoteError(null);
  };

  const promote = async () => {
    const name = value.name.trim();
    if (!name || promoting) return;
    setPromoting(true);
    setPromoteError(null);
    try {
      const result = await api.promoteTrainingShooter(name);
      onChange({ name: displayName(result.person), personId: result.person.id });
      setOpen(false);
    } catch (e) {
      setPromoteError(String(e));
    } finally {
      setPromoting(false);
    }
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (!open || suggestions.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => (h + 1) % suggestions.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => (h - 1 + suggestions.length) % suggestions.length);
    } else if (e.key === "Enter") {
      const p = suggestions[highlight];
      if (p) {
        e.preventDefault();
        pick(p);
      }
    } else if (e.key === "Escape") {
      setOpen(false);
    }
  };

  const listStyle: CSSProperties | undefined = pos
    ? {
        position: "fixed",
        top: pos.top,
        left: pos.left,
        width: pos.width,
        maxHeight: pos.maxHeight,
        zIndex: 10000,
      }
    : undefined;

  return (
    <div className="shooter-ac" ref={wrapRef}>
      <input
        ref={inputRef}
        value={value.name}
        disabled={disabled}
        placeholder={placeholder}
        autoComplete="off"
        aria-autocomplete="list"
        aria-expanded={showList}
        aria-controls={listId}
        onChange={(e) => onInput(e.target.value)}
        onFocus={(e) => {
          setOpen(true);
          // Select so the next keystroke replaces and refilters immediately.
          e.currentTarget.select();
        }}
        onKeyDown={onKeyDown}
      />
      {value.personId ? (
        <span className="shooter-ac-link" title="Mit Personendatenbank verknüpft">
          DB
        </span>
      ) : canPromote ? (
        <button
          type="button"
          className="shooter-ac-promote"
          title="Als Person in Verwaltung anlegen und Trainingsserien verknüpfen"
          disabled={promoting}
          onClick={() => void promote()}
        >
          {promoting ? "…" : "Anlegen"}
        </button>
      ) : null}
      {promoteError ? (
        <span className="shooter-ac-error" title={promoteError}>
          !
        </span>
      ) : null}
      {showList && pos
        ? createPortal(
            <ul
              id={listId}
              ref={listRef}
              className={`shooter-ac-list shooter-ac-list-portal shooter-ac-list-${pos.placement}`}
              role="listbox"
              style={listStyle}
            >
              {suggestions.map((p, i) => (
                <li key={p.id} role="option" aria-selected={i === highlight}>
                  <button
                    type="button"
                    className={i === highlight ? "ac-on" : undefined}
                    onMouseEnter={() => setHighlight(i)}
                    onClick={() => pick(p)}
                  >
                    {personLabel(p)}
                  </button>
                </li>
              ))}
            </ul>,
            document.body,
          )
        : null}
    </div>
  );
}
