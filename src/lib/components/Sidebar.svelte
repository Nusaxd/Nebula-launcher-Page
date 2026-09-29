<script lang="ts">
  import { app } from "../store.svelte";
  import type { Page } from "../types";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";

  const items: { page: Page; label: string; icon: string }[] = [
    { page: "library", label: "Library", icon: "library" },
    { page: "versions", label: "Versions", icon: "download" },
    { page: "accounts", label: "Accounts", icon: "user" },
    { page: "console", label: "Console", icon: "terminal" },
    { page: "settings", label: "Settings", icon: "settings" },
  ];

  let runningCount = $derived(Object.values(app.games).filter((g) => g.state === "running" || g.state === "preparing").length);
</script>

<aside class="flex w-60 shrink-0 flex-col border-r border-white/8 bg-black/25 px-3 py-4 backdrop-blur">
  <div class="mb-6 flex items-center gap-3 px-2">
    <img src="/logo.svg" alt="" class="size-9 rounded-xl shadow-lg shadow-violet-900/40" />
    <div class="leading-tight">
      <div class="text-[15px] font-semibold tracking-tight">Nebula</div>
      <div class="text-[11px] text-zinc-500">Minecraft Launcher</div>
    </div>
  </div>

  <nav class="flex flex-1 flex-col gap-1">
    {#each items as it (it.page)}
      {@const active = app.page === it.page}
      <button
        class="group relative flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition
          {active ? 'bg-white/10 text-white' : 'text-zinc-400 hover:bg-white/5 hover:text-zinc-100'}"
        onclick={() => app.go(it.page)}
      >
        {#if active}
          <span class="absolute top-2 bottom-2 left-0 w-1 rounded-full bg-gradient-to-b from-violet-400 to-sky-400"></span>
        {/if}
        <Icon name={it.icon} size={18} />
        <span>{it.label}</span>
        {#if it.page === "console" && runningCount > 0}
          <span class="ml-auto flex items-center gap-1.5 text-[11px] text-emerald-300">
            <span class="size-1.5 animate-pulse rounded-full bg-emerald-400"></span>{runningCount}
          </span>
        {/if}
      </button>
    {/each}
  </nav>

  <button
    class="mt-2 flex items-center gap-3 rounded-xl border border-white/8 bg-white/[0.04] p-2.5 text-left transition hover:bg-white/8"
    onclick={() => app.go("accounts")}
  >
    {#if app.currentAccount}
      <Avatar username={app.currentAccount.username} kind={app.currentAccount.kind} size={36} />
      <div class="min-w-0 leading-tight">
        <div class="truncate text-sm font-medium">{app.currentAccount.username}</div>
        <div class="text-[11px] text-zinc-500">{app.currentAccount.kind === "elyby" ? "Ely.by" : "Offline"}</div>
      </div>
    {:else}
      <div class="flex size-9 items-center justify-center rounded-lg bg-white/10 text-zinc-400"><Icon name="user" size={18} /></div>
      <div class="leading-tight">
        <div class="text-sm font-medium">No account</div>
        <div class="text-[11px] text-violet-300">Add one to play</div>
      </div>
    {/if}
  </button>
  <div class="mt-3 px-2 text-[11px] text-zinc-600">v{app.appVersion}</div>
</aside>
