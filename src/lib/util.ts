import type { Instance, VersionType } from "./types";

export const TYPE_LABEL: Record<VersionType, string> = {
  release: "Release",
  snapshot: "Snapshot",
  old_beta: "Beta",
  old_alpha: "Alpha",
};

export const TYPE_BADGE: Record<VersionType, string> = {
  release: "bg-emerald-500/15 text-emerald-300 ring-emerald-400/30",
  snapshot: "bg-amber-500/15 text-amber-300 ring-amber-400/30",
  old_beta: "bg-sky-500/15 text-sky-300 ring-sky-400/30",
  old_alpha: "bg-rose-500/15 text-rose-300 ring-rose-400/30",
};

/** Full class names so Tailwind can see them. */
export const COLORS: Record<string, { gradient: string; dot: string }> = {
  violet: { gradient: "from-violet-500 to-fuchsia-600", dot: "bg-violet-500" },
  blue: { gradient: "from-sky-500 to-indigo-600", dot: "bg-sky-500" },
  emerald: { gradient: "from-emerald-400 to-teal-600", dot: "bg-emerald-500" },
  rose: { gradient: "from-rose-500 to-orange-500", dot: "bg-rose-500" },
  amber: { gradient: "from-amber-400 to-orange-600", dot: "bg-amber-500" },
  cyan: { gradient: "from-cyan-400 to-blue-600", dot: "bg-cyan-500" },
  pink: { gradient: "from-pink-500 to-purple-600", dot: "bg-pink-500" },
  slate: { gradient: "from-slate-500 to-zinc-700", dot: "bg-slate-500" },
};

export function gradientFor(color: string): string {
  return (COLORS[color] ?? COLORS.violet).gradient;
}

export function formatBytes(n: number): string {
  if (!n) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatDuration(secs: number): string {
  if (!secs) return "Never played";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h > 0) return `${h}h ${m}m played`;
  if (m > 0) return `${m}m played`;
  return "Less than a minute";
}

export function timeAgo(unixSecs: number | null): string {
  if (!unixSecs) return "Never";
  const diff = Math.max(0, Date.now() / 1000 - unixSecs);
  if (diff < 60) return "Just now";
  if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h ago`;
  if (diff < 86400 * 30) return `${Math.floor(diff / 86400)} d ago`;
  return new Date(unixSecs * 1000).toLocaleDateString();
}

export function formatDate(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "";
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function blankInstance(versionId = "", name = ""): Instance {
  return {
    id: "",
    name,
    versionId,
    memoryMb: null,
    javaPath: "",
    jvmArgs: "",
    width: null,
    height: null,
    fullscreen: false,
    server: "",
    color: Object.keys(COLORS)[Math.floor(Math.random() * 7)],
    createdAt: 0,
    lastPlayed: null,
    playTimeSecs: 0,
  };
}

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}
