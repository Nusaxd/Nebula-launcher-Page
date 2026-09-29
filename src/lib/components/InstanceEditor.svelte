<script lang="ts">
  import { app } from "../store.svelte";
  import type { Instance, VersionType } from "../types";
  import { COLORS, TYPE_LABEL, formatDate } from "../util";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import Toggle from "./Toggle.svelte";
  import TypeBadge from "./TypeBadge.svelte";

  // Local editable copy of the instance being created / edited.
  let draft = $state<Instance>({ ...(app.editing as Instance) });
  const isNew = !draft.id;

  let types = $state<Record<VersionType, boolean>>({
    release: true,
    snapshot: false,
    old_beta: false,
    old_alpha: false,
  });
  let query = $state("");
  let useDefaultMem = $state(draft.memoryMb === null);
  let memory = $state(draft.memoryMb ?? app.settings.defaultMemoryMb);
  let customRes = $state(draft.width !== null && draft.height !== null);
  let busy = $state(false);

  // When editing an instance on a non-release version, make sure it is visible in the list.
  {
    const cur = app.versions.find((v) => v.id === draft.versionId);
    if (cur) types[cur.type] = true;
  }

  let filtered = $derived(
    app.versions.filter((v) => types[v.type] && (!query || v.id.toLowerCase().includes(query.trim().toLowerCase()))),
  );
  let selectedEntry = $derived(app.versions.find((v) => v.id === draft.versionId));

  function pickVersion(id: string) {
    const prevDefault = draft.versionId ? `Minecraft ${draft.versionId}` : "";
    draft.versionId = id;
    if (!draft.name || draft.name === prevDefault) draft.name = `Minecraft ${id}`;
  }

  function close() {
    app.editing = null;
  }

  async function save() {
    busy = true;
    const payload: Instance = {
      ...$state.snapshot(draft),
      memoryMb: useDefaultMem ? null : memory,
      width: customRes ? Number(draft.width) || 854 : null,
      height: customRes ? Number(draft.height) || 480 : null,
    };
    const ok = await app.saveInstance(payload);
    busy = false;
    if (ok) close();
  }
</script>

<Modal title={isNew ? "New instance" : "Edit instance"} onclose={close} width="max-w-2xl">
  <div class="space-y-5">
    <div class="flex gap-4">
      <div class="min-w-0 flex-1">
        <label class="label" for="inst-name">Name</label>
        <input id="inst-name" class="input" bind:value={draft.name} placeholder="My survival world" />
      </div>
      <div>
        <span class="label">Color</span>
        <div class="flex h-[38px] items-center gap-1.5">
          {#each Object.entries(COLORS) as [key, c] (key)}
            <button
              type="button"
              class="size-5 rounded-full transition {c.dot} {draft.color === key ? 'scale-110 ring-2 ring-white ring-offset-2 ring-offset-ink-800' : 'opacity-70 hover:opacity-100'}"
              aria-label={key}
              onclick={() => (draft.color = key)}
            ></button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Version picker -->
    <div>
      <span class="label">Minecraft version</span>
      <div class="mb-2 flex flex-wrap items-center gap-2">
        {#each Object.keys(types) as t (t)}
          <button type="button" class="chip {types[t as VersionType] ? 'chip-active' : ''}" onclick={() => (types[t as VersionType] = !types[t as VersionType])}>
            {TYPE_LABEL[t as VersionType]}
          </button>
        {/each}
        <div class="relative ml-auto w-44">
          <Icon name="search" size={14} class="absolute top-1/2 left-2.5 -translate-y-1/2 text-zinc-500" />
          <input class="input py-1.5 pl-8 text-xs" placeholder="Search…" bind:value={query} />
        </div>
      </div>
      <div class="h-44 overflow-y-auto rounded-xl border border-white/10 bg-ink-900/70 p-1">
        {#if app.versions.length === 0}
          <div class="flex h-full items-center justify-center px-6 text-center text-xs text-zinc-500">
            {app.versionsLoading ? "Loading versions…" : app.versionsError || "No versions loaded."}
          </div>
        {:else}
          {#each filtered.slice(0, 250) as v (v.id)}
            {@const sel = v.id === draft.versionId}
            <button
              type="button"
              class="flex w-full items-center gap-3 rounded-lg px-3 py-1.5 text-left text-sm transition {sel ? 'bg-violet-500/25 text-white' : 'text-zinc-300 hover:bg-white/5'}"
              onclick={() => pickVersion(v.id)}
            >
              <span class="w-36 truncate font-mono text-[13px]">{v.id}</span>
              <TypeBadge type={v.type} />
              <span class="ml-auto text-xs text-zinc-500">{formatDate(v.releaseTime)}</span>
              {#if app.isInstalled(v.id)}<Icon name="check" size={14} class="text-emerald-400" />{/if}
            </button>
          {/each}
          {#if filtered.length === 0}
            <div class="p-6 text-center text-xs text-zinc-500">No versions match your filters.</div>
          {/if}
        {/if}
      </div>
      {#if draft.versionId}
        <p class="hint">
          Selected: <span class="font-mono text-zinc-300">{draft.versionId}</span>
          {#if selectedEntry}<span class="text-zinc-500">({TYPE_LABEL[selectedEntry.type]})</span>{/if}
          {#if !app.isInstalled(draft.versionId)}· will be downloaded on first launch{/if}
        </p>
      {/if}
    </div>

    <!-- Memory -->
    <div>
      <div class="mb-1 flex items-center justify-between">
        <span class="label mb-0">Memory</span>
        <span class="text-xs text-zinc-400 tabular-nums">{useDefaultMem ? `Default (${app.settings.defaultMemoryMb} MB)` : `${memory} MB`}</span>
      </div>
      <Toggle bind:checked={useDefaultMem} label="Use launcher default" />
      {#if !useDefaultMem}
        <input type="range" class="mt-1 w-full" min="512" max="16384" step="256" bind:value={memory} />
      {/if}
    </div>

    <details class="group rounded-xl border border-white/8 bg-white/[0.02]">
      <summary class="cursor-pointer list-none px-4 py-3 text-sm font-medium text-zinc-300 select-none">
        Advanced options
      </summary>
      <div class="space-y-4 border-t border-white/8 p-4">
        <div>
          <label class="label" for="inst-server">Auto-join server</label>
          <input id="inst-server" class="input" bind:value={draft.server} placeholder="play.example.com:25565 (optional)" />
        </div>

        <div>
          <Toggle bind:checked={customRes} label="Custom window size" />
          {#if customRes}
            <div class="mt-2 flex items-center gap-2">
              <input class="input" type="number" min="200" bind:value={draft.width} placeholder="854" />
              <span class="text-zinc-500">×</span>
              <input class="input" type="number" min="200" bind:value={draft.height} placeholder="480" />
            </div>
          {/if}
        </div>
        <Toggle bind:checked={draft.fullscreen} label="Start in fullscreen" />

        <div>
          <label class="label" for="inst-java">Java executable</label>
          <input id="inst-java" class="input font-mono text-xs" bind:value={draft.javaPath} placeholder="Automatic (recommended)" />
          <p class="hint">Leave empty to use the right Mojang-provided runtime for this version.</p>
        </div>
        <div>
          <label class="label" for="inst-jvm">Extra JVM arguments</label>
          <input id="inst-jvm" class="input font-mono text-xs" bind:value={draft.jvmArgs} placeholder="-XX:+UseG1GC -Dfoo=bar" />
        </div>
      </div>
    </details>
  </div>

  {#snippet footer()}
    <button class="btn-ghost" onclick={close}>Cancel</button>
    <button class="btn-primary" disabled={busy || !draft.name.trim() || !draft.versionId} onclick={save}>
      {isNew ? "Create instance" : "Save changes"}
    </button>
  {/snippet}
</Modal>
