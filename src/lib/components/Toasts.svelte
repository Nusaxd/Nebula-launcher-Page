<script lang="ts">
  import { app } from "../store.svelte";
  import Icon from "./Icon.svelte";

  const styles = {
    info: "border-sky-400/30 text-sky-200",
    success: "border-emerald-400/30 text-emerald-200",
    error: "border-rose-400/40 text-rose-200",
  } as const;
  const icons = { info: "alert", success: "check", error: "alert" } as const;
</script>

<div class="pointer-events-none fixed right-5 bottom-5 z-50 flex w-96 max-w-[calc(100vw-2.5rem)] flex-col gap-2">
  {#each app.toasts as t (t.id)}
    <div
      class="pointer-events-auto flex animate-slide-up items-start gap-3 rounded-xl border bg-ink-800/95 px-4 py-3 text-sm shadow-xl shadow-black/40 backdrop-blur {styles[t.kind]}"
      role="status"
    >
      <Icon name={icons[t.kind]} size={18} class="mt-0.5 shrink-0" />
      <span class="selectable min-w-0 flex-1 wrap-break-word text-zinc-100">{t.text}</span>
      <button class="text-zinc-500 hover:text-white" onclick={() => app.dismissToast(t.id)} aria-label="Dismiss">
        <Icon name="x" size={14} />
      </button>
    </div>
  {/each}
</div>
