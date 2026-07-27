import type { ReactNode } from "react";

type SectionProps = {
  title: string;
  description?: string;
  danger?: boolean;
  children: ReactNode;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
};

/** Settings section — optionally collapsible via open/onOpenChange. */
export function SettingsSection({
  title,
  description,
  danger = false,
  children,
  open = true,
  onOpenChange,
}: SectionProps) {
  const collapsible = typeof onOpenChange === "function";
  const expanded = collapsible ? open : true;

  return (
    <section
      className={`settings-section${danger ? " is-danger" : ""}${expanded ? " is-open" : " is-collapsed"}`}
    >
      {collapsible ? (
        <button
          type="button"
          className="settings-section-toggle"
          aria-expanded={expanded}
          onClick={() => onOpenChange(!open)}
        >
          <span className="settings-section-toggle-copy">
            <span className="settings-section-title">{title}</span>
            {!expanded && description ? (
              <span className="settings-section-peek">{description}</span>
            ) : null}
          </span>
          <span className="settings-section-chevron" aria-hidden />
        </button>
      ) : (
        <header className="settings-section-head">
          <h3 className="settings-section-title">{title}</h3>
          {description ? (
            <p className="settings-section-desc">{description}</p>
          ) : null}
        </header>
      )}
      {expanded ? (
        <div className="settings-section-body">
          {collapsible && description ? (
            <p className="settings-section-desc">{description}</p>
          ) : null}
          {children}
        </div>
      ) : null}
    </section>
  );
}

type RowProps = {
  label: string;
  hint?: string;
  children: ReactNode;
};

/** Label left / control right — scan-friendly preference row. */
export function SettingsRow({ label, hint, children }: RowProps) {
  return (
    <div className="settings-row">
      <div className="settings-row-copy">
        <span className="settings-row-label">{label}</span>
        {hint ? <span className="settings-row-hint">{hint}</span> : null}
      </div>
      <div className="settings-row-control">{children}</div>
    </div>
  );
}

type HintProps = {
  children: ReactNode;
};

export function SettingsHint({ children }: HintProps) {
  return <p className="settings-hint">{children}</p>;
}

type ToggleProps = {
  label: string;
  hint?: string;
  checked?: boolean;
  disabled?: boolean;
  onChange?: (next: boolean) => void;
};

/** Preference toggle — disabled = visible placeholder until wired. */
export function SettingsToggle({
  label,
  hint,
  checked = false,
  disabled = true,
  onChange,
}: ToggleProps) {
  return (
    <label
      className={`side-sheet-toggle settings-toggle${disabled ? " is-placeholder" : ""}`}
    >
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange?.(e.target.checked)}
      />
      <span>
        {label}
        {hint ? <span className="field-hint">{hint}</span> : null}
      </span>
    </label>
  );
}

type ChoiceOption = {
  value: string;
  label: string;
};

type ChoiceProps = {
  label: string;
  hint?: string;
  value: string;
  options: ChoiceOption[];
  disabled?: boolean;
  onChange?: (value: string) => void;
};

/** Compact choice group (Farbschema, Trefferanzeige, …). */
export function SettingsChoice({
  label,
  hint,
  value,
  options,
  disabled = true,
  onChange,
}: ChoiceProps) {
  return (
    <div className={`settings-choice${disabled ? " is-placeholder" : ""}`}>
      <div className="settings-row-copy">
        <span className="settings-row-label">{label}</span>
        {hint ? <span className="settings-row-hint">{hint}</span> : null}
      </div>
      <div className="settings-choice-options" role="group" aria-label={label}>
        {options.map((opt) => (
          <button
            key={opt.value}
            type="button"
            className={`settings-choice-btn${value === opt.value ? " is-on" : ""}`}
            disabled={disabled}
            aria-pressed={value === opt.value}
            onClick={() => onChange?.(opt.value)}
          >
            {opt.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export type SettingsStatusTone =
  | "ok"
  | "progress"
  | "neutral"
  | "idle"
  | "locked";

type StatusPillProps = {
  tone?: SettingsStatusTone;
  children: ReactNode;
};

/** Subtle status chip — product tone, not alert chrome. */
export function SettingsStatusPill({
  tone = "neutral",
  children,
}: StatusPillProps) {
  return (
    <span className={`settings-status-pill is-${tone}`}>{children}</span>
  );
}

type InfoRowProps = {
  label: string;
  value: string;
  /** path = muted monospace technical path */
  variant?: "default" | "path";
  /** Render value as status pill instead of plain text. */
  statusTone?: SettingsStatusTone;
};

/** Read-only info row: label left, value/pill right. */
export function SettingsInfoRow({
  label,
  value,
  variant = "default",
  statusTone,
}: InfoRowProps) {
  return (
    <div
      className={`settings-info-row${variant === "path" ? " is-path" : ""}`}
    >
      <span className="settings-info-label">{label}</span>
      <div className="settings-info-value">
        {statusTone ? (
          <SettingsStatusPill tone={statusTone}>{value}</SettingsStatusPill>
        ) : (
          <span
            className={
              variant === "path" ? "settings-info-path" : "settings-info-text"
            }
            title={variant === "path" ? value : undefined}
          >
            {value}
          </span>
        )}
      </div>
    </div>
  );
}

/** @deprecated Prefer SettingsInfoRow — kept for gradual migration. */
export function SettingsValue({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <SettingsRow label={label} hint={hint}>
      <span className="settings-value">{value}</span>
    </SettingsRow>
  );
}

type LockedCardProps = {
  statusLabel: string;
  children: ReactNode;
};

/** Calm locked / protected state surface for Admin. */
export function SettingsLockedCard({
  statusLabel,
  children,
}: LockedCardProps) {
  return (
    <div className="settings-locked-card">
      <div className="settings-locked-card-status">
        <SettingsStatusPill tone="locked">{statusLabel}</SettingsStatusPill>
      </div>
      <div className="settings-locked-card-body">{children}</div>
    </div>
  );
}
