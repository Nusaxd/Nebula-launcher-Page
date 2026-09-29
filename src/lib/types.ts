export type VersionType = "release" | "snapshot" | "old_beta" | "old_alpha";
export type Page = "library" | "versions" | "accounts" | "console" | "settings";

export interface VersionEntry {
  id: string;
  type: VersionType;
  url: string;
  releaseTime: string;
  sha1: string;
}

export interface VersionList {
  latestRelease: string;
  latestSnapshot: string;
  versions: VersionEntry[];
  installed: string[];
  offline: boolean;
}

export interface Instance {
  id: string;
  name: string;
  versionId: string;
  memoryMb: number | null;
  javaPath: string;
  jvmArgs: string;
  width: number | null;
  height: number | null;
  fullscreen: boolean;
  server: string;
  color: string;
  createdAt: number;
  lastPlayed: number | null;
  playTimeSecs: number;
}

export type AccountKind = "offline" | "elyby";

export interface Account {
  id: string;
  kind: AccountKind;
  username: string;
  uuid: string;
}

export interface Settings {
  defaultMemoryMb: number;
  javaPath: string;
  jvmArgs: string;
  managedJava: boolean;
  hideOnLaunch: boolean;
  concurrency: number;
}

export interface Bootstrap {
  settings: Settings;
  accounts: Account[];
  instances: Instance[];
  selectedAccount: string | null;
  selectedInstance: string | null;
  dataDir: string;
  os: string;
  appVersion: string;
}

export interface Progress {
  id: string;
  stage: string;
  doneBytes: number;
  totalBytes: number;
  doneFiles: number;
  totalFiles: number;
}

export type GameStatus = "preparing" | "running" | "stopped" | "crashed" | "error";

export interface GameState {
  instanceId: string;
  state: GameStatus;
  message: string | null;
  exitCode: number | null;
}

export interface LogLine {
  instanceId: string;
  stream: "stdout" | "stderr" | "launcher";
  line: string;
}
