<script lang="ts">
  import { app } from "../store.svelte";
  import Modal from "./Modal.svelte";

  let inst = $derived(app.deleting);
  let deleteFiles = $state(false);
  let busy = $state(false);

  async function confirm() {
    if (!inst) return;
    busy = true;
    await app.deleteInstance(inst, deleteFiles);
    busy = false;
    app.deleting = null;
  }
</script>

{#if inst}
  <Modal title="Delete instance" onclose={() => (app.deleting = null)} width="max-w-md">
    <p class="text-zinc-300">
      Delete <span class="font-semibold text-white">{inst.name}</span>? This only removes the launcher entry unless you also
      delete its files.
    </p>
    <label class="mt-4 flex cursor-pointer items-start gap-3 rounded-xl border border-white/10 bg-white/5 p-3">
      <input type="checkbox" class="mt-0.5 size-4 accent-rose-500" bind:checked={deleteFiles} />
      <span>
        <span class="block font-medium text-zinc-100">Also delete game files</span>
        <span class="block text-xs text-zinc-500">Worlds, screenshots, resource packs and settings will be erased forever.</span>
      </span>
    </label>
    {#snippet footer()}
      <button class="btn-ghost" onclick={() => (app.deleting = null)}>Cancel</button>
      <button class="btn-danger" disabled={busy} onclick={confirm}>Delete</button>
    {/snippet}
  </Modal>
{/if}
