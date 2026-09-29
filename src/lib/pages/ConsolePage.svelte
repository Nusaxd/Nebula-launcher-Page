<script lang="ts">
  import { tick } from "svelte";
  import { api } from "../api";
  import { app } from "../store.svelte";
  import type { LogLine } from "../types";
  import Icon from "../components/Icon.svelte";
  import PageHeader from "../components/PageHeader.svelte";

  let id = $derived(app.consoleInstanceId);
  let lines = $derived<LogLine[]>(id ? (app.logs[id] ?? []) : []);
  let status = $derived(id ? app.statusOf(id) : "idle");
  let follow = $state(true);
  let box = $state<HTMLDivElement | null>(null);

  $effect(() => {
    void lines.length;
    if (follow && box) {
      tick().then(() => {
        if (box) box.scrollTop = box.scrollHeight;
      });
    }
  });

  function onScroll() {
    if (!box) return;
    follow = box.scrollHeight - box.scrollTop - box.clientHeight < 40;
  }

  function pick(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    app.consoleInstanceId = v;
    follow = true;
    void app.loadLogs(v);
  }

  function color(l: LogLine): string {
    if (l.stream === "launcher") return "text-violet-300";
    const t = l.line;
    if (/\b(ERROR|FATAL|SEVERE)\b|Exception|^\s+at /.test(t)) return "text-rose-300";
    if (/\bWARN(ING)?\b/.test(t)) return "text-amber-300";
    if (l.stream === "stderr") return "text-orange-200";
    return "text-zinc-300";
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(lines.map((l) => l.line).join("\n"));
      app.toast("success", "Log copied to clipboard", 1800);
    } catch {
      app.toast("error", "Could not access the clipboard");
    }
  }

  const statusText: Record<string, string> = {
    idle: "Idle",
    preparing: "Preparing",
    running: "Running",
    stopped: "Stopped",
    crashed: "Crashed",
    error: "Error",
  };
  const statusColor: Record<string, string> = {
    idle: "bg-zinc-500",
    preparing: "bg-sky-400 animate-pulse",
    running: "bg-emerald-400 animate-pulse",
    stopped: "bg-zinc-400",
    crashed: "bg-rose-400",
    error: "bg-rose-400",
  };
</script>

<PageHeader title="Console" subtitle="Live output of the game and the launcher.">
  {#snippet actions()}
    {#if app.instances.length}
      <select class="input w-56" value={id ?? ""} onchange={pick}>
        {#each app.instances as i (i.id)}
          <option value={i.id}>{i.name}</option>
        {/each}
      </select>
    {/if}
  {/snippet}
</PageHeader>

<div class="flex min-h-0 flex-1 flex-col px-8 pb-8">
  {#if !id}
    <div class="card p-10 text-center text-zinc-500">Create an instance and launch it to see its output here.</div>
  {:else}
    <div class="mb-3 flex items-center gap-2">
      <span class="chip"><span class="size-2 rounded-full {statusColor[status]}"></span> {statusText[status]}</span>
      <span class="text-xs text-zinc-600">{lines.length} lines</span>
      <div class="ml-auto flex items-center gap-2">
        {#if status === "running"}
          <button class="btn-danger px-3 py-1.5 text-xs" onclick={() => app.kill(id!)}><Icon name="stop" size={12} /> Kill</button>
        {/if}
        <button class="btn-ghost px-3 py-1.5 text-xs" onclick={() => api.openInstanceFolder(id!)}><Icon name="folder" size={13} /> Folder</button>
        <button class="btn-ghost px-3 py-1.5 text-xs" onclick={copy} disabled={!lines.length}><Icon name="copy" size={13} /> Copy</button>
        <button class="btn-ghost px-3 py-1.5 text-xs" onclick={() => app.clearLogs(id!)} disabled={!lines.length}><Icon name="trash" size={13} /> Clear</button>
      </div>
    </div>

    <div
      bind:this={box}
      onscroll={onScroll}
      class="selectable card min-h-0 flex-1 overflow-auto bg-black/40 p-4 font-mono text-[12px] leading-5"
    >
      {#if lines.length === 0}
        <div class="flex h-full items-center justify-center text-zinc-600">No output yet.</div>
      {:else}
        {#each lines as l, i (i)}
          <div class="whitespace-pre-wrap break-all {color(l)}">{l.line}</div>
        {/each}
      {/if}
    </div>
    {#if !follow && lines.length}
      <div class="pointer-events-none relative">
        <button
          class="btn-primary pointer-events-auto absolute -top-14 right-6 px-3 py-1.5 text-xs"
          onclick={() => {
            follow = true;
            if (box) box.scrollTop = box.scrollHeight;
          }}
        >
          Jump to latest
        </button>
      </div>
    {/if}
  {/if}
</div>
