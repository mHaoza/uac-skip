interface IconProps {
  size?: number;
  className?: string;
}

const box = (size: number) => ({
  width: size,
  height: size,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
});

export function ShieldIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M12 3.2 5.2 6v5.4c0 4.3 2.9 7.6 6.8 9.4 3.9-1.8 6.8-5.1 6.8-9.4V6L12 3.2Z" />
      <path d="m9.2 12.1 2 2 3.6-3.9" />
    </svg>
  );
}

export function PlusIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

export function PlayIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M7 4.8v14.4l11.5-7.2L7 4.8Z" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function PencilIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M4 20h4l10-10a2.8 2.8 0 0 0-4-4L4 16v4Z" />
      <path d="m13.5 6.5 4 4" />
    </svg>
  );
}

export function TrashIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M4 7h16M9 7V4.8h6V7M6.5 7l.8 12.2h9.4L17.5 7" />
      <path d="M10.2 11v4.6M13.8 11v4.6" />
    </svg>
  );
}

export function FolderIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M3.5 7.5A1.5 1.5 0 0 1 5 6h4l2 2.4h8a1.5 1.5 0 0 1 1.5 1.5v7.6A1.5 1.5 0 0 1 19 19H5a1.5 1.5 0 0 1-1.5-1.5v-10Z" />
    </svg>
  );
}

export function SearchIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <circle cx="10.8" cy="10.8" r="6.3" />
      <path d="m15.5 15.5 4 4" />
    </svg>
  );
}

export function RefreshIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M20 12a8 8 0 1 1-2.6-5.9" />
      <path d="M20 4.5V10h-5.3" />
    </svg>
  );
}

export function MoreIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <circle cx="5.5" cy="12" r="1.4" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.4" fill="currentColor" stroke="none" />
      <circle cx="18.5" cy="12" r="1.4" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function WarningIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M12 4.5 21 19.5H3L12 4.5Z" />
      <path d="M12 10v4.2M12 17.2v.2" />
    </svg>
  );
}

export function CheckIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="m5 12.8 4.4 4.2L19 6.5" />
    </svg>
  );
}

export function CloseIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M6 6l12 12M18 6 6 18" />
    </svg>
  );
}

export function BoltIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M13.2 3 5.8 13.2h5L10 21l7.4-10.2h-5L13.2 3Z" />
    </svg>
  );
}

export function CopyIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <rect x="9" y="9" width="11" height="11" rx="2.2" />
      <path d="M15 6.5V5.4A1.4 1.4 0 0 0 13.6 4H5.4A1.4 1.4 0 0 0 4 5.4v8.2A1.4 1.4 0 0 0 5.4 15h1.1" />
    </svg>
  );
}

export function PowerIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M12 3.5v8" />
      <path d="M17.5 6.2a7.5 7.5 0 1 1-11 0" />
    </svg>
  );
}

export function SpinnerIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className} style={{ animation: "spin 0.8s linear infinite" }}>
      <path d="M12 3.5a8.5 8.5 0 1 0 8.5 8.5" />
    </svg>
  );
}

export function ShortcutIcon({ size = 24, className }: IconProps) {
  return (
    <svg {...box(size)} className={className}>
      <path d="M13.5 4.5h6v6" />
      <path d="M19.5 4.5 11 13" />
      <path d="M18 14.5v4a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6h4" />
    </svg>
  );
}
