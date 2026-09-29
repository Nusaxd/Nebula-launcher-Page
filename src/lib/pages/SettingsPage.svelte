<script lang="ts">
  import { api } from "../api";
  import { app } from "../store.svelte";
  import type { Settings } from "../types";
  import { errorText } from "../util";
  import Icon from "../components/Icon.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Toggle from "../components/Toggle.svelte";

  let draft = $state<Settings>({ ...app.settings });
  let javaResult = $state("");
  let javaOk = $state(true);
  let testing = $state(false);

  let dirty = $derived(JSON.stringify($state.snapshot(draft)) !== JSON.stringify($state.snapshot(app.settings)));

  async function testJava() {
    testing = true;
    javaResult = "";
    try {
      javaResult = await api.testJava(draft.javaPath);
      javaOk = true;
    } catch (e) {
      javaResult = errorText(e);
      javaOk = false;
    } finally {
      testing = false;
    }
  }

  async function save() {
    await app.saveSettings($state.snapshot(draft) as Settings);
    draft = { ...app.settings };
  }
</script>

<PageHeader title="Settings" subtitle="Defaults used by every instance unless it overrides them.">
  {#snippet actions()}
    <button class="btn-ghost" disabled={!dirty} onclick={() => (draft = { ...app.settings })}>Reset</button>
    <button class="btn-primary" disabled={!dirty} onclick={save}><Icon name="check" size={16} /> Save</button>
  {/snippet}
</PageHeader>

<div class="min-h-0 flex-1 space-y-4 overflow-y-auto px-8 pb-8">
  <section class="card p-5">
    <h2 class="mb-4 font-semibold">Memory</h2>
    <div class="mb-1 flex items-center justify-between">
      <span class="label mb-0">Default maximum memory</span>
      <span class="rounded-lg bg-white/10 px-2 py-0.5 text-xs tabular-nums">{draft.defaultMemoryMb} MB · {(draft.defaultMemoryMb / 1024).toFixed(1)} GB</span>
    </div>
    <input type="range" class="w-full" min="512" max="16384" step="256" bind:value={draft.defaultMemoryMb} />
    <p class="hint">Modern versions are happy with 2–4 GB. Old alpha/beta versions need far less.</p>
  </section>

  <section class="card p-5">
    <h2 class="mb-4 font-semibold">Java</h2>
    <Toggle
      bind:checked={draft.managedJava}
      label="Download Java automatically"
      description="Uses Mojang's official runtime for each version (Java 8 for old versions, 17/21/25 for new ones)."
    />
    <div class="mt-3">
      <label class="label" for="s-java">Custom Java executable</label>
      <div class="flex gap-2">
        <input id="s-java" class="input font-mono text-xs" bind:value={draft.javaPath} placeholder="Automatic — or /usr/lib/jvm/java-21/bin/java" />
        <button class="btn-ghost shrink-0" onclick={testJava} disabled={testing}>{testing ? "Testing…" : "Test"}</button>
      </div>
      {#if javaResult}
        <p class="selectable mt-2 rounded-lg px-3 py-2 font-mono text-xs {javaOk ? 'bg-emerald-500/10 text-emerald-200' : 'bg-rose-500/10 text-rose-200'}">{javaResult}</p>
      {/if}
      <p class="hint">When set, this Java is used for all instances (unless an instance has its own).</p>
    </div>
    <div class="mt-4">
      <label class="label" for="s-jvm">Global JVM arguments</label>
      <input id="s-jvm" class="input font-mono text-xs" bind:value={draft.jvmArgs} placeholder="-XX:+UseG1GC -XX:MaxGCPauseMillis=50" />
    </div>
  </section>

  <section class="card p-5">
    <h2 class="mb-2 font-semibold">Launcher</h2>
    <Toggle bind:checked={draft.hideOnLaunch} label="Hide launcher while playing" description="The window comes back when the game closes." />
    <div class="mt-3">
      <div class="mb-1 flex items-center justify-between">
        <span class="label mb-0">Parallel downloads</span>
        <span class="text-xs text-zinc-400 tabular-nums">{draft.concurrency}</span>
      </div>
      <input type="range" class="w-full" min="1" max="32" step="1" bind:value={draft.concurrency} />
    </div>
  </section>

  <section class="card p-5">
    <h2 class="mb-3 font-semibold">Storage</h2>
    <div class="flex items-center gap-3">
      <div class="selectable min-w-0 flex-1 truncate rounded-xl border border-white/10 bg-ink-900/70 px-3 py-2 font-mono text-xs text-zinc-400">{app.dataDir}</div>
      <button class="btn-ghost shrink-0" onclick={() => api.openDataFolder().catch((e) => app.toast("error", errorText(e)))}>
        <Icon name="folder" size={16} /> Open
      </button>
    </div>
    <p class="hint">Versions, libraries, assets and Java runtimes are shared between instances. Instances live in <span class="font-mono">instances/</span>.</p>
  </section>

  <section class="card flex items-center justify-between p-5">
    <div>
      <h2 class="font-semibold">Nebula Launcher</h2>
      <p class="text-xs text-zinc-500">Version {app.appVersion} · Rust + Tauri 2 + Svelte 5</p>
    </div>
    <button class="btn-ghost" onclick={() => api.openUrl("https://github.com/Nusaxd/Nebula-launcher-Page")}>
      <Icon name="external" size={15} /> GitHub
    </button>
  </section>
</div>
