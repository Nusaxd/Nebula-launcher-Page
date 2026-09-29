// In-memory fake backend so the UI can be developed and previewed in a normal browser.
// It is only ever loaded when the app is NOT running inside Tauri.
import type {
  Account,
  Bootstrap,
  GameState,
  Instance,
  LogLine,
  Progress,
  Settings,
  VersionEntry,
  VersionList,
} from "./types";

const bus = new EventTarget();

function emit<T>(name: string, payload: T) {
  bus.dispatchEvent(new CustomEvent(name, { detail: payload }));
}

export async function listen<T>(name: string, cb: (p: T) => void): Promise<() => void> {
  const h = (e: Event) => cb((e as CustomEvent<T>).detail);
  bus.addEventListener(name, h);
  return () => bus.removeEventListener(name, h);
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const rid = () => Math.random().toString(16).slice(2, 14);

// ---- fake data -------------------------------------------------------------
function v(id: string, type: VersionEntry["type"], date: string): VersionEntry {
  return { id, type, url: "", releaseTime: date, sha1: "" };
}
const VERSIONS: VersionEntry[] = [
  v("26.4-snapshot-1", "snapshot", "2026-09-22T13:38:53+00:00"),
  v("26.3", "release", "2026-09-15T11:23:02+00:00"),
  v("26.3-rc-1", "snapshot", "2026-09-10T11:28:25+00:00"),
  v("26.2", "release", "2026-06-16T12:03:33+00:00"),
  v("1.21.8", "release", "2025-07-17T12:00:00+00:00"),
  v("1.21.4", "release", "2024-12-03T10:12:57+00:00"),
  v("25w14craftmine", "snapshot", "2025-04-01T13:00:00+00:00"),
  v("1.20.1", "release", "2023-06-12T13:25:51+00:00"),
  v("1.19.4", "release", "2023-03-14T12:56:18+00:00"),
  v("1.16.5", "release", "2021-01-15T16:30:00+00:00"),
  v("1.12.2", "release", "2017-09-18T08:39:46+00:00"),
  v("1.8.9", "release", "2015-12-09T10:23:08+00:00"),
  v("1.7.10", "release", "2014-05-14T17:29:23+00:00"),
  v("1.6.4", "release", "2013-09-19T15:52:37+00:00"),
  v("1.0", "release", "2011-11-17T22:00:00+00:00"),
  v("b1.8.1", "old_beta", "2011-09-14T22:00:00+00:00"),
  v("b1.7.3", "old_beta", "2011-07-07T22:00:00+00:00"),
  v("b1.3_01", "old_beta", "2011-04-25T22:00:00+00:00"),
  v("a1.2.6", "old_alpha", "2010-12-03T22:00:00+00:00"),
  v("a1.1.2_01", "old_alpha", "2010-09-23T22:00:00+00:00"),
  v("a1.0.4", "old_alpha", "2010-07-30T22:00:00+00:00"),
];

let settings: Settings = {
  defaultMemoryMb: 2048,
  javaPath: "",
  jvmArgs: "",
  managedJava: true,
  hideOnLaunch: false,
  concurrency: 12,
};
let accounts: Account[] = [
  { id: "acc1", kind: "offline", username: "Steve", uuid: "5627dd98e6be3c21b8a8e92344183641" },
];
let instances: Instance[] = [
  {
    id: "inst1",
    name: "Survival 1.21",
    versionId: "1.21.4",
    memoryMb: 4096,
    javaPath: "",
    jvmArgs: "",
    width: null,
    height: null,
    fullscreen: false,
    server: "",
    color: "emerald",
    createdAt: Date.now() / 1000 - 86400 * 9,
    lastPlayed: Date.now() / 1000 - 3600 * 5,
    playTimeSecs: 3600 * 12 + 600,
  },
  {
    id: "inst2",
    name: "Beta nostalgia",
    versionId: "b1.7.3",
    memoryMb: null,
    javaPath: "",
    jvmArgs: "",
    width: 854,
    height: 480,
    fullscreen: false,
    server: "",
    color: "amber",
    createdAt: Date.now() / 1000 - 86400 * 3,
    lastPlayed: null,
    playTimeSecs: 0,
  },
];
let selectedAccount: string | null = "acc1";
let selectedInstance: string | null = "inst1";
const installed = new Set<string>(["1.21.4"]);
const logs = new Map<string, LogLine[]>();
const running = new Map<string, { stop: () => void }>();

function log(instanceId: string, stream: LogLine["stream"], line: string) {
  const entry: LogLine = { instanceId, stream, line };
  const buf = logs.get(instanceId) ?? [];
  buf.push(entry);
  logs.set(instanceId, buf);
  emit("game-log", entry);
}

function state(instanceId: string, s: GameState["state"], message: string | null = null, exitCode: number | null = null) {
  emit<GameState>("game-state", { instanceId, state: s, message, exitCode });
}

async function fakeInstall(id: string, versionId: string) {
  const stages = ["Fetching version info", "Downloading Minecraft client", "Downloading libraries", "Downloading assets"];
  for (const stage of stages) {
    const total = stage.includes("assets") ? 4_200 : 40;
    const size = stage.includes("client") ? 24_000_000 : stage.includes("assets") ? 180_000_000 : 30_000_000;
    if (stage === stages[0]) {
      emit<Progress>("install-progress", { id, stage, doneBytes: 0, totalBytes: 0, doneFiles: 0, totalFiles: 0 });
      await sleep(400);
      continue;
    }
    for (let i = 0; i <= 10; i++) {
      emit<Progress>("install-progress", {
        id,
        stage,
        doneBytes: (size * i) / 10,
        totalBytes: size,
        doneFiles: Math.round((total * i) / 10),
        totalFiles: total,
      });
      await sleep(90);
    }
  }
  installed.add(versionId);
}

async function fakeLaunch(inst: Instance) {
  state(inst.id, "preparing");
  logs.delete(inst.id);
  await fakeInstall(inst.id, inst.versionId);
  const acc = accounts.find((a) => a.id === selectedAccount) ?? accounts[0];
  log(inst.id, "launcher", `Minecraft ${inst.versionId} as ${acc?.username} (${acc?.kind})`);
  log(inst.id, "launcher", "Java: /demo/runtimes/java-runtime-delta/bin/java");
  state(inst.id, "running");
  const lines = [
    "[main/INFO]: Environment: Environment[sessionHost=https://sessionserver.mojang.com]",
    "[main/INFO]: Setting user: " + (acc?.username ?? "Player"),
    "[Render thread/INFO]: Backend library: LWJGL version 3.3.3",
    "[Render thread/INFO]: Reloading ResourceManager: vanilla",
    "[Render thread/WARN]: Ignoring unknown sound event minecraft:demo",
    "[Server thread/INFO]: Starting integrated minecraft server version " + inst.versionId,
    "[Server thread/INFO]: Preparing spawn area: 100%",
    "[Render thread/INFO]: Stopping!",
  ];
  let i = 0;
  const timer = setInterval(() => {
    if (i < lines.length) log(inst.id, i === 4 ? "stderr" : "stdout", lines[i++]);
  }, 500);
  running.set(inst.id, {
    stop: () => {
      clearInterval(timer);
      running.delete(inst.id);
      log(inst.id, "launcher", "Game exited");
      state(inst.id, "stopped", null, 0);
    },
  });
}

// ---- command router ----------------------------------------------------------
type Args = Record<string, unknown> | undefined;

export async function invoke<T>(cmd: string, args: Args = {}): Promise<T> {
  await sleep(40);
  const r = await route(cmd, args as Record<string, unknown>);
  return r as T;
}

async function route(cmd: string, a: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "bootstrap":
      return {
        settings,
        accounts,
        instances,
        selectedAccount,
        selectedInstance,
        dataDir: "~/.local/share/dev.nusaxd.nebula-launcher (browser preview)",
        os: "linux",
        appVersion: "0.1.0",
      } satisfies Bootstrap;
    case "update_settings":
      settings = a.settings as Settings;
      return settings;
    case "test_java":
      return 'openjdk version "21.0.4" 2024-07-16 (mock)';
    case "running_instances":
      return [...running.keys()];
    case "list_versions":
      await sleep(300);
      return {
        latestRelease: "26.3",
        latestSnapshot: "26.4-snapshot-1",
        versions: VERSIONS,
        installed: [...installed],
        offline: false,
      } satisfies VersionList;
    case "install_version":
      await fakeInstall(a.versionId as string, a.versionId as string);
      return null;
    case "uninstall_version":
      installed.delete(a.versionId as string);
      return null;
    case "save_instance": {
      const inst = { ...(a.instance as Instance) };
      if (!inst.name.trim()) throw "Please give the instance a name";
      if (!inst.id) {
        inst.id = rid();
        inst.createdAt = Date.now() / 1000;
        instances = [...instances, inst];
        selectedInstance = inst.id;
      } else {
        instances = instances.map((i) => (i.id === inst.id ? { ...i, ...inst } : i));
      }
      return inst;
    }
    case "duplicate_instance": {
      const src = instances.find((i) => i.id === a.id);
      if (!src) throw "Instance not found";
      const copy = { ...src, id: rid(), name: src.name + " (copy)", lastPlayed: null, playTimeSecs: 0 };
      instances = [...instances, copy];
      return copy;
    }
    case "delete_instance":
      instances = instances.filter((i) => i.id !== a.id);
      if (selectedInstance === a.id) selectedInstance = instances[0]?.id ?? null;
      return null;
    case "select_instance":
      selectedInstance = a.id as string | null;
      return null;
    case "open_instance_folder":
    case "open_data_folder":
      return null;
    case "open_url":
      window.open(a.url as string, "_blank");
      return null;
    case "add_offline_account": {
      const name = String(a.username).trim();
      if (name.length < 3 || name.length > 16) throw "Usernames must be between 3 and 16 characters long";
      if (!/^\w+$/.test(name)) throw "Usernames may only contain letters, numbers and underscores";
      const acc: Account = { id: rid(), kind: "offline", username: name, uuid: rid() + rid() + rid().slice(0, 8) };
      accounts = [...accounts, acc];
      selectedAccount = acc.id;
      return acc;
    }
    case "login_elyby": {
      await sleep(600);
      if (a.password === "2fa" && !a.totp) throw "Account protected with two factor auth.";
      if (a.password === "wrong") throw "Invalid credentials. Invalid email or password.";
      const acc: Account = { id: rid(), kind: "elyby", username: String(a.login).split("@")[0], uuid: rid() + rid() + rid().slice(0, 8) };
      accounts = [...accounts, acc];
      selectedAccount = acc.id;
      return acc;
    }
    case "remove_account":
      accounts = accounts.filter((x) => x.id !== a.id);
      if (selectedAccount === a.id) selectedAccount = accounts[0]?.id ?? null;
      return null;
    case "select_account":
      selectedAccount = a.id as string;
      return null;
    case "launch_instance": {
      const inst = instances.find((i) => i.id === a.id);
      if (!inst) throw "Instance not found";
      if (!accounts.length) throw "Add an account first (Accounts tab)";
      void fakeLaunch(inst);
      return null;
    }
    case "kill_instance": {
      const r = running.get(a.id as string);
      r?.stop();
      return !!r;
    }
    case "get_logs":
      return logs.get(a.id as string) ?? [];
    case "clear_logs":
      logs.delete(a.id as string);
      return null;
    default:
      throw `Unknown command: ${cmd}`;
  }
}
