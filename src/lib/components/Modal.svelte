<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    title,
    onclose,
    children,
    footer,
    width = "max-w-xl",
  }: {
    title: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
    width?: string;
  } = $props();
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div
  class="fixed inset-0 z-40 flex animate-fade-in items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
  role="presentation"
  onmousedown={(e) => e.target === e.currentTarget && onclose()}
>
  <div
    class="flex max-h-full w-full animate-pop-in flex-col overflow-hidden rounded-2xl border border-white/10 bg-ink-800 shadow-2xl shadow-black/60 {width}"
    role="dialog"
    aria-modal="true"
    aria-label={title}
  >
    <div class="flex items-center justify-between border-b border-white/8 px-5 py-4">
      <h2 class="text-base font-semibold">{title}</h2>
      <button class="btn-icon size-8" onclick={onclose} aria-label="Close"><Icon name="x" size={16} /></button>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
      {@render children()}
    </div>
    {#if footer}
      <div class="flex items-center justify-end gap-2 border-t border-white/8 bg-black/20 px-5 py-3">
        {@render footer()}
      </div>
    {/if}
  </div>
</div>
