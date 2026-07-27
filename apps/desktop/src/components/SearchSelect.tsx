import {
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";

export type SearchSelectOption = {
  id: string;
  label: string;
  /** Used for create-button „already exists“ check. Defaults to label. */
  matchText?: string;
};

type ListPos = {
  top: number;
  left: number;
  width: number;
  maxHeight: number;
  placement: "down" | "up";
};

type Props = {
  value: string;
  options: SearchSelectOption[];
  onChange: (id: string) => void;
  disabled?: boolean;
  placeholder?: string;
  /** Allow clearing the selection (empty id). Default true. */
  allowClear?: boolean;
  /**
   * Show „Anlegen“ when the typed draft is non-empty and does not match the
   * current selection (same pattern as ShooterAutocomplete promote).
   */
  allowCreate?: boolean;
  createLabel?: string;
  createBusy?: boolean;
  createExpanded?: boolean;
  onCreateClick?: () => void;
  /** Called when the draft query changes (for prefilling create forms). */
  onDraftChange?: (draft: string) => void;
};

/**
 * Combobox styled like ShooterAutocomplete: filterable input + portal list.
 * Value is an option id (not free text).
 */
export function SearchSelect({
  value,
  options,
  onChange,
  disabled,
  placeholder = "Aus Liste wählen…",
  allowClear = true,
  allowCreate = false,
  createLabel = "Anlegen",
  createBusy = false,
  createExpanded = false,
  onCreateClick,
  onDraftChange,
}: Props) {
  const listId = useId();
  const wrapRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const selectedLabel = options.find((o) => o.id === value)?.label ?? "";

  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState(selectedLabel);
  const [highlight, setHighlight] = useState(0);
  const [pos, setPos] = useState<ListPos | null>(null);
  const editing = useRef(false);

  useEffect(() => {
    if (!value || !options.some((o) => o.id === value)) {
      editing.current = false;
      setQuery("");
      return;
    }
    if (!editing.current) setQuery(selectedLabel);
  }, [selectedLabel, value, options]);

  useEffect(() => {
    onDraftChange?.(query);
  }, [query, onDraftChange]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q || q === selectedLabel.trim().toLowerCase()) return options;
    return options.filter((o) => o.label.toLowerCase().includes(q));
  }, [options, query, selectedLabel]);

  const draft = query.trim();
  const draftLower = draft.toLowerCase();
  const matchesSelection =
    draftLower.length > 0 && draftLower === selectedLabel.trim().toLowerCase();
  const matchesSuggestion = options.some(
    (o) => (o.matchText ?? o.label).trim().toLowerCase() === draftLower,
  );
  const canCreate =
    allowCreate &&
    !disabled &&
    Boolean(onCreateClick) &&
    draft.length > 0 &&
    !matchesSelection &&
    !matchesSuggestion;

  const showList = open && !disabled && filtered.length > 0;

  const updatePos = () => {
    const el = inputRef.current ?? wrapRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const gap = 4;
    const preferred = Math.min(224, filtered.length * 40 + 12);
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
    // eslint-disable-next-line react-hooks/exhaustive-deps -- recompute when list opens / options change
  }, [showList, filtered.length]);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      const t = e.target as Node;
      if (wrapRef.current?.contains(t) || listRef.current?.contains(t)) return;
      editing.current = false;
      setOpen(false);
      setQuery(selectedLabel);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [selectedLabel]);

  useEffect(() => {
    if (disabled) {
      editing.current = false;
      setOpen(false);
      setQuery(selectedLabel);
    }
  }, [disabled, selectedLabel]);

  const pick = (id: string) => {
    editing.current = false;
    onChange(id);
    setQuery(options.find((o) => o.id === id)?.label ?? "");
    setOpen(false);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (!open) {
      if (e.key === "ArrowDown" || e.key === "Enter") {
        e.preventDefault();
        editing.current = true;
        setOpen(true);
        setHighlight(0);
      }
      return;
    }
    if (filtered.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => (h + 1) % filtered.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => (h - 1 + filtered.length) % filtered.length);
    } else if (e.key === "Enter") {
      const opt = filtered[highlight];
      if (opt) {
        e.preventDefault();
        pick(opt.id);
      }
    } else if (e.key === "Escape") {
      editing.current = false;
      setOpen(false);
      setQuery(selectedLabel);
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
        value={query}
        disabled={disabled}
        placeholder={placeholder}
        autoComplete="off"
        aria-autocomplete="list"
        aria-expanded={showList}
        aria-controls={listId}
        onChange={(e) => {
          editing.current = true;
          const text = e.target.value;
          setQuery(text);
          // Clearing the field closes the list; typing opens/filters it.
          setOpen(text.trim().length > 0);
          setHighlight(0);
          if (allowClear && text.trim() === "") onChange("");
        }}
        onFocus={(e) => {
          editing.current = true;
          setOpen(true);
          setHighlight(0);
          // Select so the next keystroke replaces and refilters immediately.
          e.currentTarget.select();
        }}
        onKeyDown={onKeyDown}
      />
      {canCreate || createExpanded ? (
        <button
          type="button"
          className="shooter-ac-promote"
          title="Neuen Eintrag anlegen"
          aria-expanded={createExpanded}
          disabled={createBusy || disabled}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => onCreateClick?.()}
        >
          {createBusy ? "…" : createLabel}
        </button>
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
              {filtered.map((opt, i) => (
                <li key={opt.id} role="option" aria-selected={i === highlight}>
                  <button
                    type="button"
                    className={i === highlight ? "ac-on" : undefined}
                    onMouseEnter={() => setHighlight(i)}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => pick(opt.id)}
                  >
                    {opt.label}
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
