<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, subtitle = "", actions }: { title: string; subtitle?: string; actions?: Snippet } = $props();
</script>

<header class="flex items-end justify-between gap-4 px-8 pt-7 pb-5">
  <div>
    <h1 class="text-2xl font-semibold tracking-tight">{title}</h1>
    {#if subtitle}<p class="mt-1 text-sm text-zinc-500">{subtitle}</p>{/if}
  </div>
  {#if actions}<div class="flex items-center gap-2">{@render actions()}</div>{/if}
</header>
