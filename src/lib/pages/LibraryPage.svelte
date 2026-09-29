<script lang="ts">
  import { api } from "../api";
  import { app } from "../store.svelte";
  import { blankInstance, formatDuration, gradientFor, timeAgo } from "../util";
  import Icon from "../components/Icon.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import ProgressBar from "../components/ProgressBar.svelte";
  import TypeBadge from "../components/TypeBadge.svelte";

  let selected = $derived(app.currentInstance);
  let status = $derived(selected ? app.statusOf(selected.id) : "idle");
  let entry = $derived(selected ? app.versionById.get(selected.versionId) : undefined);

  function newInstance(versionId = app.latestRelease) {
    app.editing = blankInstance(versionId, versionId ? `Minecraft ${versionId}` : "");
  }
</script>

<PageHeader title="Library" subtitle="Your Minecraft instances. Each one has its own worlds, mods and settings.">
  {#snippet actions()}
    <button class="btn-primary" onclick={() => newInstance()}><Icon name="plus" size={16} /> New instance</button>
  {/snippet}
</PageHeader>

<div class="min-h-0 flex-1 overflow-y-auto px-8 pb-8">
  {#if app.instances.length === 0}
    <div class="card mx-auto mt-6 flex max-w-xl animate-slide-up flex-col items-center px-8 py-12 text-center">
      <div class="mb-5 flex size-16 items-center justify-center rounded-2xl bg-gradient-to-br from-violet-500 to-sky-500 shadow-xl shadow-violet-900/40">
        <Icon name="box" size={32} class="text-white" />
      </div>
      <h2 class="text-xl font-semibold">Welcome to Nebula</h2>
      <p class="mt-2 text-zinc-400">
        Create an instance to play any release, snapshot, beta or alpha version of Minecraft. Game files are downloaded
        straight from Mojang and verified.
      </p>
      <div class="mt-6 flex flex-wrap justify-center gap-2">
        <button class="btn-primary" onclick={() => newInstance()}>
          <Icon name="plus" size={16} /> Latest release {app.latestRelease}
        </button>
        {#if app.latestSnapshot && app.latestSnapshot !== app.latestRelease}
          <button class="btn-ghost" onclick={() => newInstance(app.latestSnapshot)}>Latest snapshot</button>
        {/if}
        <button class="btn-ghost" onclick={() => app.go("versions")}>Browse versions</button>
      </div>
      {#if !app.accounts.length}
        <button class="mt-5 text-xs text-violet-300 hover:text-violet-200" onclick={() => app.go("accounts")}>
          First, add an account →
        </button>
      {/if}
    </div>
  {:else}
    {#if selected}
      <!-- Hero for the selected instance -->
      <section class="card relative mb-7 animate-fade-in overflow-hidden">
        <div class="absolute inset-0 bg-gradient-to-br opacity-25 {gradientFor(selected.color)}"></div>
        <div class="absolute inset-0 bg-linear-to-t from-ink-950/80 via-transparent to-transparent"></div>
        <div class="relative flex flex-wrap items-center gap-6 p-6">
          <div class="flex size-20 shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br text-3xl font-bold text-white shadow-xl shadow-black/40 ring-1 ring-white/20 {gradientFor(selected.color)}">
            {selected.name.slice(0, 1).toUpperCase()}
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <h2 class="truncate text-2xl font-semibold tracking-tight">{selected.name}</h2>
              <TypeBadge type={entry?.type} />
            </div>
            <div class="mt-1 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-zinc-400">
              <span class="font-mono text-zinc-300">{selected.versionId}</span>
              <span class="flex items-center gap-1"><Icon name="clock" size={12} /> {formatDuration(selected.playTimeSecs)}</span>
              <span>Last played: {timeAgo(selected.lastPlayed)}</span>
              <span class="flex items-center gap-1"><Icon name="cpu" size={12} /> {selected.memoryMb ?? app.settings.defaultMemoryMb} MB</span>
              {#if !app.isInstalled(selected.versionId)}<span class="text-amber-300">Not downloaded yet</span>{/if}
            </div>

            {#if status === "preparing"}
              <div class="mt-4 max-w-md"><ProgressBar progress={app.progress[selected.id]} /></div>
            {/if}
          </div>

          <div class="flex items-center gap-2">
            {#if status === "running"}
              <button class="btn-danger px-6 py-3 text-base" onclick={() => app.kill(selected.id)}>
                <Icon name="stop" size={16} /> Stop
              </button>
            {:else if status === "preparing"}
              <button class="btn-ghost px-6 py-3 text-base" disabled>Preparing…</button>
            {:else}
              <button class="btn-primary px-7 py-3 text-base" onclick={() => app.launch(selected.id)}>
                <Icon name="play" size={16} />
                {app.isInstalled(selected.versionId) ? "Play" : "Install & Play"}
              </button>
            {/if}
            <button class="btn-icon" title="Console" aria-label="Console" onclick={() => app.openConsole(selected.id)}><Icon name="terminal" size={16} /></button>
            <button class="btn-icon" title="Open folder" aria-label="Open folder" onclick={() => api.openInstanceFolder(selected.id).catch((e) => app.toast("error", String(e)))}><Icon name="folder" size={16} /></button>
            <button class="btn-icon" title="Edit" aria-label="Edit" onclick={() => (app.editing = { ...selected })}><Icon name="edit" size={16} /></button>
            <button class="btn-icon" title="Duplicate" aria-label="Duplicate" onclick={() => app.duplicateInstance(selected.id)}><Icon name="layers" size={16} /></button>
            <button class="btn-icon hover:text-rose-300" title="Delete" aria-label="Delete" disabled={app.isBusy(selected.id)} onclick={() => (app.deleting = selected)}><Icon name="trash" size={16} /></button>
          </div>
        </div>
      </section>
    {/if}

    <h3 class="mb-3 text-xs font-semibold tracking-wider text-zinc-500 uppercase">All instances · {app.instances.length}</h3>
    <div class="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-4">
      {#each app.instances as inst (inst.id)}
        {@const st = app.statusOf(inst.id)}
        {@const ve = app.versionById.get(inst.versionId)}
        <div
          class="card group relative cursor-pointer overflow-hidden p-4 text-left transition hover:-translate-y-0.5 hover:bg-white/[0.07]
            {app.selectedInstanceId === inst.id ? 'ring-2 ring-violet-400/60' : ''}"
          role="button"
          tabindex="0"
          onclick={() => app.selectInstance(inst.id)}
          onkeydown={(e) => (e.key === "Enter" || e.key === " ") && app.selectInstance(inst.id)}
        >
          <div class="flex items-start gap-3">
            <div class="flex size-12 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br text-lg font-bold text-white shadow-lg shadow-black/30 {gradientFor(inst.color)}">
              {inst.name.slice(0, 1).toUpperCase()}
            </div>
            <div class="min-w-0 flex-1">
              <div class="truncate font-semibold">{inst.name}</div>
              <div class="mt-0.5 flex items-center gap-2">
                <span class="truncate font-mono text-xs text-zinc-400">{inst.versionId}</span>
                <TypeBadge type={ve?.type} />
              </div>
            </div>
          </div>
          <div class="mt-4 flex items-center justify-between text-xs text-zinc-500">
            <span>{inst.lastPlayed ? timeAgo(inst.lastPlayed) : "Never played"}</span>
            {#if st === "running"}
              <span class="flex items-center gap-1.5 text-emerald-300"><span class="size-1.5 animate-pulse rounded-full bg-emerald-400"></span> Running</span>
            {:else if st === "preparing"}
              <span class="text-sky-300">Preparing…</span>
            {:else if st === "crashed" || st === "error"}
              <span class="text-rose-300">Failed</span>
            {:else if app.isInstalled(inst.versionId)}
              <span class="flex items-center gap-1 text-zinc-500"><Icon name="check" size={12} /> Installed</span>
            {/if}
          </div>
          {#if st === "preparing"}
            <div class="mt-3"><ProgressBar progress={app.progress[inst.id]} compact /></div>
          {/if}
        </div>
      {/each}

      <button
        class="flex min-h-[112px] flex-col items-center justify-center gap-2 rounded-2xl border border-dashed border-white/15 text-zinc-500 transition hover:border-violet-400/50 hover:bg-violet-500/5 hover:text-violet-200"
        onclick={() => newInstance()}
      >
        <Icon name="plus" size={22} />
        <span class="text-sm font-medium">New instance</span>
      </button>
    </div>
  {/if}
</div>
