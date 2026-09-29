import { api, events } from "./api";
import type {
  Account,
  GameState,
  Instance,
  LogLine,
  Page,
  Progress,
  Settings,
  VersionEntry,
} from "./types";
import { errorText } from "./util";

export interface Toast {
  id: number;
  kind: "info" | "success" | "error";
  text: string;
}

const MAX_LOG_LINES = 3000;

class Launcher {
  ready = $state(false);
  page = $state<Page>("library");

  settings = $state<Settings>({
    defaultMemoryMb: 2048,
    javaPath: "",
    jvmArgs: "",
    managedJava: true,
    hideOnLaunch: false,
    concurrency: 12,
  });
  accounts = $state<Account[]>([]);
  instances = $state<Instance[]>([]);
  selectedAccountId = $state<string | null>(null);
  selectedInstanceId = $state<string | null>(null);
  dataDir = $state("");
  os = $state("");
  appVersion = $state("");

  versions = $state.raw<VersionEntry[]>([]);
  installed = $state<string[]>([]);
  latestRelease = $state("");
  latestSnapshot = $state("");
  versionsLoading = $state(false);
  versionsOffline = $state(false);
  versionsError = $state("");
  /** version id -> true while its download is running from the Versions page */
  installing = $state<Record<string, boolean>>({});

  progress = $state<Record<string, Progress>>({});
  games = $state<Record<string, GameState>>({});
  logs = $state.raw<Record<string, LogLine[]>>({});
  toasts = $state<Toast[]>([]);

  // modals
  editing = $state<Instance | null>(null);
  showElyLogin = $state(false);
  deleting = $state<Instance | null>(null);

  consoleInstanceId = $state<string | null>(null);

  private pendingLogs: Record<string, LogLine[]> = {};
  private flushTimer: ReturnType<typeof setTimeout> | null = null;
  private toastSeq = 0;

  // ---- derived helpers ------------------------------------------------------
  get currentAccount(): Account | null {
    return this.accounts.find((a) => a.id === this.selectedAccountId) ?? this.accounts[0] ?? null;
  }
  get currentInstance(): Instance | null {
    return this.instances.find((i) => i.id === this.selectedInstanceId) ?? null;
  }
  get versionById(): Map<string, VersionEntry> {
    return new Map(this.versions.map((v) => [v.id, v]));
  }
  isInstalled(id: string): boolean {
    return this.installed.includes(id);
  }
  statusOf(instanceId: string): GameState["state"] | "idle" {
    return this.games[instanceId]?.state ?? "idle";
  }
  isBusy(instanceId: string): boolean {
    const s = this.statusOf(instanceId);
    return s === "preparing" || s === "running";
  }

  // ---- lifecycle ------------------------------------------------------------
  async init() {
    try {
      const b = await api.bootstrap();
      this.settings = b.settings;
      this.accounts = b.accounts;
      this.instances = b.instances;
      this.selectedAccountId = b.selectedAccount;
      this.selectedInstanceId = b.selectedInstance ?? b.instances[0]?.id ?? null;
      this.consoleInstanceId = this.selectedInstanceId;
      this.dataDir = b.dataDir;
      this.os = b.os;
      this.appVersion = b.appVersion;
    } catch (e) {
      this.toast("error", errorText(e));
    }

    await events.progress((p) => {
      this.progress[p.id] = p;
    });
    await events.gameState((s) => this.onGameState(s));
    await events.gameLog((l) => this.onLog(l));

    try {
      const running = await api.runningInstances();
      for (const id of running) {
        this.games[id] = { instanceId: id, state: "running", message: null, exitCode: null };
      }
    } catch {
      /* ignore */
    }

    this.ready = true;
    void this.refreshVersions();
  }

  async refreshVersions(force = false) {
    this.versionsLoading = true;
    this.versionsError = "";
    try {
      const list = await api.listVersions(force);
      this.versions = list.versions;
      this.installed = list.installed;
      this.latestRelease = list.latestRelease;
      this.latestSnapshot = list.latestSnapshot;
      this.versionsOffline = list.offline;
    } catch (e) {
      this.versionsError = errorText(e);
    } finally {
      this.versionsLoading = false;
    }
  }

  // ---- toasts ---------------------------------------------------------------
  toast(kind: Toast["kind"], text: string, ms = 5000) {
    const id = ++this.toastSeq;
    this.toasts.push({ id, kind, text });
    setTimeout(() => this.dismissToast(id), ms);
  }
  dismissToast(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  // ---- events ---------------------------------------------------------------
  private onGameState(s: GameState) {
    this.games[s.instanceId] = s;
    const name = this.instances.find((i) => i.id === s.instanceId)?.name ?? "Instance";
    if (s.state === "running" || s.state === "error" || s.state === "stopped" || s.state === "crashed") {
      delete this.progress[s.instanceId];
    }
    if (s.state === "running") this.toast("success", `${name} is running`, 2500);
    if (s.state === "error") this.toast("error", s.message ?? "Launch failed", 9000);
    if (s.state === "crashed") this.toast("error", `${name} crashed. ${s.message ?? ""}`, 9000);
    if (s.state === "stopped" || s.state === "crashed" || s.state === "error") {
      // played time / last played changed
      void this.reloadInstances();
      if (!this.installed.length || s.state === "stopped") void this.refreshInstalledOnly();
    }
  }

  private onLog(l: LogLine) {
    (this.pendingLogs[l.instanceId] ??= []).push(l);
    if (!this.flushTimer) {
      this.flushTimer = setTimeout(() => this.flushLogs(), 100);
    }
  }

  private flushLogs() {
    this.flushTimer = null;
    const next = { ...this.logs };
    for (const [id, lines] of Object.entries(this.pendingLogs)) {
      const merged = (next[id] ?? []).concat(lines);
      next[id] = merged.length > MAX_LOG_LINES ? merged.slice(merged.length - MAX_LOG_LINES) : merged;
    }
    this.pendingLogs = {};
    this.logs = next;
  }

  async loadLogs(id: string) {
    try {
      const lines = await api.getLogs(id);
      this.pendingLogs[id] = [];
      this.logs = { ...this.logs, [id]: lines };
    } catch {
      /* ignore */
    }
  }

  async clearLogs(id: string) {
    await api.clearLogs(id);
    this.pendingLogs[id] = [];
    this.logs = { ...this.logs, [id]: [] };
  }

  private async reloadInstances() {
    try {
      const b = await api.bootstrap();
      this.instances = b.instances;
    } catch {
      /* ignore */
    }
  }

  private async refreshInstalledOnly() {
    try {
      const list = await api.listVersions(false);
      this.installed = list.installed;
    } catch {
      /* ignore */
    }
  }

  // ---- navigation -----------------------------------------------------------
  go(page: Page) {
    this.page = page;
    if (page === "console") {
      const id = this.consoleInstanceId ?? this.selectedInstanceId ?? this.instances[0]?.id ?? null;
      this.consoleInstanceId = id;
      if (id) void this.loadLogs(id);
    }
  }

  openConsole(instanceId: string) {
    this.consoleInstanceId = instanceId;
    void this.loadLogs(instanceId);
    this.page = "console";
  }

  // ---- instances ------------------------------------------------------------
  async selectInstance(id: string) {
    this.selectedInstanceId = id;
    await api.selectInstance(id).catch(() => {});
  }

  async saveInstance(inst: Instance): Promise<boolean> {
    try {
      const saved = await api.saveInstance(inst);
      const idx = this.instances.findIndex((i) => i.id === saved.id);
      if (idx >= 0) this.instances[idx] = saved;
      else {
        this.instances.push(saved);
        this.selectedInstanceId = saved.id;
      }
      this.toast("success", `Saved “${saved.name}”`, 2500);
      return true;
    } catch (e) {
      this.toast("error", errorText(e));
      return false;
    }
  }

  async duplicateInstance(id: string) {
    try {
      const copy = await api.duplicateInstance(id);
      this.instances.push(copy);
      this.selectedInstanceId = copy.id;
      this.toast("success", `Created “${copy.name}”`, 2500);
    } catch (e) {
      this.toast("error", errorText(e));
    }
  }

  async deleteInstance(inst: Instance, deleteFiles: boolean) {
    try {
      await api.deleteInstance(inst.id, deleteFiles);
      this.instances = this.instances.filter((i) => i.id !== inst.id);
      if (this.selectedInstanceId === inst.id) {
        this.selectedInstanceId = this.instances[0]?.id ?? null;
      }
      this.toast("info", `Deleted “${inst.name}”`, 2500);
    } catch (e) {
      this.toast("error", errorText(e));
    }
  }

  async launch(id: string) {
    if (!this.accounts.length) {
      this.toast("error", "Add an account first");
      this.page = "accounts";
      return;
    }
    this.games[id] = { instanceId: id, state: "preparing", message: null, exitCode: null };
    this.progress[id] = { id, stage: "Preparing", doneBytes: 0, totalBytes: 0, doneFiles: 0, totalFiles: 0 };
    try {
      await api.launch(id);
    } catch (e) {
      delete this.games[id];
      delete this.progress[id];
      this.toast("error", errorText(e));
    }
  }

  async kill(id: string) {
    try {
      await api.kill(id);
    } catch (e) {
      this.toast("error", errorText(e));
    }
  }

  // ---- versions -------------------------------------------------------------
  async installVersion(versionId: string) {
    this.installing[versionId] = true;
    try {
      await api.installVersion(versionId);
      this.toast("success", `Minecraft ${versionId} installed`, 3000);
      await this.refreshInstalledOnly();
    } catch (e) {
      this.toast("error", errorText(e), 8000);
    } finally {
      delete this.installing[versionId];
      delete this.progress[versionId];
    }
  }

  async uninstallVersion(versionId: string) {
    try {
      await api.uninstallVersion(versionId);
      this.installed = this.installed.filter((v) => v !== versionId);
      this.toast("info", `Removed ${versionId}`, 2500);
    } catch (e) {
      this.toast("error", errorText(e));
    }
  }

  // ---- accounts -------------------------------------------------------------
  async addOffline(username: string): Promise<boolean> {
    try {
      const acc = await api.addOfflineAccount(username);
      this.accounts.push(acc);
      this.selectedAccountId = acc.id;
      this.toast("success", `Added offline account ${acc.username}`, 2500);
      return true;
    } catch (e) {
      this.toast("error", errorText(e));
      return false;
    }
  }

  /** Throws with the backend message so the dialog can react to two-factor prompts. */
  async loginEly(login: string, password: string, totp: string | null) {
    const acc = await api.loginElyby(login, password, totp);
    const idx = this.accounts.findIndex((a) => a.id === acc.id);
    if (idx >= 0) this.accounts[idx] = acc;
    else this.accounts.push(acc);
    this.selectedAccountId = acc.id;
    this.toast("success", `Signed in as ${acc.username}`, 2500);
  }

  async selectAccount(id: string) {
    this.selectedAccountId = id;
    await api.selectAccount(id).catch(() => {});
  }

  async removeAccount(id: string) {
    try {
      await api.removeAccount(id);
      this.accounts = this.accounts.filter((a) => a.id !== id);
      if (this.selectedAccountId === id) this.selectedAccountId = this.accounts[0]?.id ?? null;
    } catch (e) {
      this.toast("error", errorText(e));
    }
  }

  // ---- settings -------------------------------------------------------------
  async saveSettings(next: Settings) {
    try {
      this.settings = await api.updateSettings(next);
      this.toast("success", "Settings saved", 2000);
    } catch (e) {
      this.toast("error", errorText(e));
    }
  }
}

export const app = new Launcher();
