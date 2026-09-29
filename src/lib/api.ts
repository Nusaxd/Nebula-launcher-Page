import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type {
  Account,
  Bootstrap,
  GameState,
  Instance,
  LogLine,
  Progress,
  Settings,
  VersionList,
} from "./types";

/** True when running inside the Tauri webview (false in a plain browser, e.g. `npm run dev`). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const mock = await import("./mock");
  return mock.invoke<T>(cmd, args);
}

export async function on<T>(event: string, cb: (payload: T) => void): Promise<() => void> {
  if (inTauri) {
    return tauriListen<T>(event, (e) => cb(e.payload));
  }
  const mock = await import("./mock");
  return mock.listen<T>(event, cb);
}

export const api = {
  bootstrap: () => call<Bootstrap>("bootstrap"),
  updateSettings: (settings: Settings) => call<Settings>("update_settings", { settings }),
  testJava: (path: string) => call<string>("test_java", { path }),
  runningInstances: () => call<string[]>("running_instances"),

  listVersions: (refresh = false) => call<VersionList>("list_versions", { refresh }),
  installVersion: (versionId: string) => call<void>("install_version", { versionId }),
  uninstallVersion: (versionId: string) => call<void>("uninstall_version", { versionId }),

  saveInstance: (instance: Instance) => call<Instance>("save_instance", { instance }),
  duplicateInstance: (id: string) => call<Instance>("duplicate_instance", { id }),
  deleteInstance: (id: string, deleteFiles: boolean) =>
    call<void>("delete_instance", { id, deleteFiles }),
  selectInstance: (id: string | null) => call<void>("select_instance", { id }),
  openInstanceFolder: (id: string) => call<void>("open_instance_folder", { id }),
  openDataFolder: () => call<void>("open_data_folder"),
  openUrl: (url: string) => call<void>("open_url", { url }),

  addOfflineAccount: (username: string) => call<Account>("add_offline_account", { username }),
  loginElyby: (login: string, password: string, totp: string | null) =>
    call<Account>("login_elyby", { login, password, totp }),
  removeAccount: (id: string) => call<void>("remove_account", { id }),
  selectAccount: (id: string) => call<void>("select_account", { id }),

  launch: (id: string) => call<void>("launch_instance", { id }),
  kill: (id: string) => call<boolean>("kill_instance", { id }),
  getLogs: (id: string) => call<LogLine[]>("get_logs", { id }),
  clearLogs: (id: string) => call<void>("clear_logs", { id }),
};

export const events = {
  progress: (cb: (p: Progress) => void) => on<Progress>("install-progress", cb),
  gameState: (cb: (s: GameState) => void) => on<GameState>("game-state", cb),
  gameLog: (cb: (l: LogLine) => void) => on<LogLine>("game-log", cb),
};
