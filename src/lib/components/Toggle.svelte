<script lang="ts">
  let {
    checked = $bindable(false),
    label,
    description = "",
  }: { checked?: boolean; label: string; description?: string } = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  class="flex w-full items-center justify-between gap-4 rounded-xl px-1 py-2 text-left"
  onclick={() => (checked = !checked)}
>
  <span class="min-w-0">
    <span class="block text-sm font-medium text-zinc-100">{label}</span>
    {#if description}<span class="block text-xs text-zinc-500">{description}</span>{/if}
  </span>
  <span
    class="relative h-6 w-11 shrink-0 rounded-full transition {checked ? 'bg-gradient-to-r from-violet-500 to-sky-500' : 'bg-white/15'}"
  >
    <span
      class="absolute top-0.5 left-0.5 size-5 rounded-full bg-white shadow transition-transform {checked ? 'translate-x-5' : ''}"
    ></span>
  </span>
</button>
