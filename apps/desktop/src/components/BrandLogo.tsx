/** RedDot brand mark + wordmark for Arena / Verwaltung headers. */
export function BrandLogo({ subtitle }: { subtitle: string }) {
  return (
    <div className="brand" aria-label={`RedDot ${subtitle}`}>
      <div className="brand-row">
        <svg
          className="brand-logo"
          viewBox="0 0 40 40"
          width="42"
          height="42"
          aria-hidden="true"
        >
          <circle cx="20" cy="20" r="18.5" fill="none" stroke="currentColor" strokeWidth="1.3" opacity="0.28" />
          <circle cx="20" cy="20" r="12.5" fill="none" stroke="currentColor" strokeWidth="1.15" opacity="0.48" />
          <circle cx="20" cy="20" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.05" opacity="0.7" />
          <circle cx="20" cy="20" r="2.85" className="brand-logo-dot" />
          <line x1="20" y1="2.5" x2="20" y2="37.5" stroke="currentColor" strokeWidth="0.65" opacity="0.22" />
          <line x1="2.5" y1="20" x2="37.5" y2="20" stroke="currentColor" strokeWidth="0.65" opacity="0.22" />
        </svg>
        <div className="brand-text">
          <span className="brand-mark">RedDot</span>
          <span className="brand-sub">{subtitle}</span>
        </div>
      </div>
    </div>
  );
}
