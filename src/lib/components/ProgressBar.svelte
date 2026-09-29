<script lang="ts">
  import type { Progress } from "../types";
  import { formatBytes } from "../util";

  let { progress, compact = false }: { progress: Progress | undefined; compact?: boolean } = $props();

  let pct = $derived(
    progress && progress.totalBytes > 0
      ? Math.min(100, (progress.doneBytes / progress.totalBytes) * 100)
      : null,
  );
</script>

{#if progress}
  <div class="w-full">
    <div class="mb-1.5 flex items-center justify-between gap-3 text-xs">
      <span class="truncate font-medium text-zinc-200">{progress.stage}…</span>
      {#if !compact}
        <span class="shrink-0 text-zinc-400 tabular-nums">
          {#if progress.totalFiles > 0}
            {progress.doneFiles}/{progress.totalFiles} files · {formatBytes(progress.doneBytes)} / {formatBytes(progress.totalBytes)}
          {/if}
        </span>
      {:else if pct !== null}
        <span class="shrink-0 text-zinc-400 tabular-nums">{pct.toFixed(0)}%</span>
      {/if}
    </div>
    <div class="h-2 overflow-hidden rounded-full bg-white/10">
      {#if pct === null}
        <div class="h-full w-1/3 animate-pulse rounded-full bg-gradient-to-r from-violet-500 to-sky-500"></div>
      {:else}
        <div
          class="h-full rounded-full bg-gradient-to-r from-violet-500 to-sky-500 transition-[width] duration-150"
          style="width:{pct}%"
        ></div>
      {/if}
    </div>
  </div>
{/if}
