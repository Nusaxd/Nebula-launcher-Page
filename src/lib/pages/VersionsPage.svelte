<script lang="ts">
  import { app } from "../store.svelte";
  import type { VersionType } from "../types";
  import { TYPE_LABEL, blankInstance, formatDate } from "../util";
  import Icon from "../components/Icon.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import ProgressBar from "../components/ProgressBar.svelte";
  import TypeBadge from "../components/TypeBadge.svelte";

  const ALL: VersionType[] = ["release", "snapshot", "old_beta", "old_alpha"];
  let types = $state<Record<VersionType, boolean>>({ release: true, snapshot: true, old_beta: true, old_alpha: true });
  let query = $state("");
  let installedOnly = $state(false);
  let limit = $state(80);

  let counts = $derived.by(() => {
    const c: Record<VersionType, number> = { release: 0, snapshot: 0, old_beta: 0, old_alpha: 0 };
    for (const v of app.versions) c[v.type]++;
    return c;
  });

  let filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return app.versions.filter(
      (v) => types[v.type] && (!q || v.id.toLowerCase().includes(q)) && (!installedOnly || app.isInstalled(v.id)),
    );
  });

  // reset paging whenever the filter changes
  $effect(() => {
    void filtered;
    limit = 80;
  });

  function create(id: string) {
    app.editing = blankInstance(id, `Minecraft ${id}`);
  }
</script>

<PageHeader
  title="Versions"
  subtitle="Every official Mojang version: releases, snapshots, old betas and old alphas."
>
  {#snippet actions()}
    <button class="btn-ghost" disabled={app.versionsLoading} onclick={() => app.refreshVersions(true)}>
      <Icon name="refresh" size={16} class={app.versionsLoading ? "animate-spin" : ""} /> Refresh
    </button>
  {/snippet}
</PageHeader>

<div class="flex flex-wrap items-center gap-2 px-8 pb-4">
  {#each ALL as t (t)}
    <button class="chip {types[t] ? 'chip-active' : ''}" onclick={() => (types[t] = !types[t])}>
      {TYPE_LABEL[t]}
      <span class="text-[10px] opacity-60">{counts[t]}</span>
    </button>
  {/each}
  <button class="chip {installedOnly ? 'chip-active' : ''}" onclick={() => (installedOnly = !installedOnly)}>
    <Icon name="check" size={12} /> Installed
  </button>
  <div class="relative ml-auto w-64">
    <Icon name="search" size={15} class="absolute top-1/2 left-3 -translate-y-1/2 text-zinc-500" />
    <input class="input pl-9" placeholder="Search versions (e.g. 1.8, b1.7, a1.2)" bind:value={query} />
  </div>
</div>

{#if app.versionsOffline}
  <div class="mx-8 mb-3 flex items-center gap-2 rounded-xl border border-amber-400/30 bg-amber-500/10 px-4 py-2 text-xs text-amber-200">
    <Icon name="wifioff" size={14} /> You appear to be offline — showing the last cached version list.
  </div>
{/if}

<div class="min-h-0 flex-1 overflow-y-auto px-8 pb-8">
  {#if app.versions.length === 0}
    <div class="card flex flex-col items-center gap-3 p-10 text-center text-zinc-400">
      {#if app.versionsLoading}
        <Icon name="refresh" size={22} class="animate-spin" /> Loading version list from Mojang…
      {:else}
        <Icon name="wifioff" size={22} />
        <div>Couldn't load versions.</div>
        {#if app.versionsError}<div class="selectable max-w-lg text-xs text-zinc-500">{app.versionsError}</div>{/if}
        <button class="btn-primary" onclick={() => app.refreshVersions(true)}>Try again</button>
      {/if}
    </div>
  {:else}
    <div class="card divide-y divide-white/5 overflow-hidden">
      {#each filtered.slice(0, limit) as v (v.id)}
        {@const installed = app.isInstalled(v.id)}
        {@const busy = app.installing[v.id]}
        <div class="flex items-center gap-4 px-4 py-2.5 transition hover:bg-white/[0.04]">
          <div class="w-44 min-w-0 shrink-0">
            <div class="flex items-center gap-2">
              <span class="truncate font-mono text-sm">{v.id}</span>
              {#if v.id === app.latestRelease}
                <span class="rounded bg-violet-500/25 px-1.5 py-0.5 text-[10px] font-semibold text-violet-200 uppercase">Latest</span>
              {:else if v.id === app.latestSnapshot}
                <span class="rounded bg-amber-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-amber-200 uppercase">Newest</span>
              {/if}
            </div>
          </div>
          <div class="w-20 shrink-0"><TypeBadge type={v.type} /></div>
          <div class="w-28 shrink-0 text-xs text-zinc-500">{formatDate(v.releaseTime)}</div>

          <div class="min-w-0 flex-1">
            {#if busy}
              <ProgressBar progress={app.progress[v.id]} compact />
            {:else if installed}
              <span class="inline-flex items-center gap-1.5 text-xs text-emerald-300"><Icon name="check" size={13} /> Installed</span>
            {/if}
          </div>

          <div class="flex shrink-0 items-center gap-2">
            {#if installed && !busy}
              <button class="btn-icon size-8 hover:text-rose-300" title="Remove downloaded files" aria-label="Remove" onclick={() => app.uninstallVersion(v.id)}>
                <Icon name="trash" size={14} />
              </button>
            {:else if !busy}
              <button class="btn-ghost px-3 py-1.5 text-xs" onclick={() => app.installVersion(v.id)}>
                <Icon name="download" size={14} /> Install
              </button>
            {/if}
            <button class="btn-ghost px-3 py-1.5 text-xs" onclick={() => create(v.id)}>
              <Icon name="plus" size={14} /> Instance
            </button>
          </div>
        </div>
      {/each}
      {#if filtered.length === 0}
        <div class="p-10 text-center text-zinc-500">No versions match your filters.</div>
      {/if}
    </div>
    {#if filtered.length > limit}
      <div class="mt-4 flex justify-center">
        <button class="btn-ghost" onclick={() => (limit += 120)}>
          Show more ({filtered.length - limit} remaining)
        </button>
      </div>
    {/if}
  {/if}
</div>
