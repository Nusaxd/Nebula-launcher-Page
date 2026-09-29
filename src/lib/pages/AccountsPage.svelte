<script lang="ts">
  import { app } from "../store.svelte";
  import Avatar from "../components/Avatar.svelte";
  import Icon from "../components/Icon.svelte";
  import PageHeader from "../components/PageHeader.svelte";

  let username = $state("");
  let busy = $state(false);

  async function addOffline(e?: Event) {
    e?.preventDefault();
    busy = true;
    const ok = await app.addOffline(username);
    busy = false;
    if (ok) username = "";
  }

  function short(uuid: string) {
    if (uuid.length !== 32) return uuid;
    return `${uuid.slice(0, 8)}-${uuid.slice(8, 12)}-${uuid.slice(12, 16)}-${uuid.slice(16, 20)}-${uuid.slice(20)}`;
  }
</script>

<PageHeader title="Accounts" subtitle="Play offline or sign in with Ely.by to get your skin and join Ely.by servers." />

<div class="min-h-0 flex-1 overflow-y-auto px-8 pb-8">
  <div class="grid gap-4 lg:grid-cols-2">
    <section class="card p-5">
      <div class="mb-3 flex items-center gap-3">
        <div class="flex size-10 items-center justify-center rounded-xl bg-white/10"><Icon name="user" size={20} /></div>
        <div>
          <h2 class="font-semibold">Offline account</h2>
          <p class="text-xs text-zinc-500">Just pick a name. Works in single player and offline-mode servers.</p>
        </div>
      </div>
      <form class="flex gap-2" onsubmit={addOffline}>
        <input class="input" placeholder="Username (3–16 letters, numbers, _)" maxlength="16" bind:value={username} />
        <button class="btn-primary shrink-0" type="submit" disabled={busy || username.trim().length < 3}>
          <Icon name="plus" size={16} /> Add
        </button>
      </form>
      <p class="hint">The UUID is generated the same way vanilla does, so your offline worlds and inventories stay consistent.</p>
    </section>

    <section class="card p-5">
      <div class="mb-3 flex items-center gap-3">
        <div class="flex size-10 items-center justify-center rounded-xl bg-emerald-500/15 text-emerald-300"><Icon name="key" size={20} /></div>
        <div>
          <h2 class="font-semibold">Ely.by</h2>
          <p class="text-xs text-zinc-500">Sign in with your Ely.by account. Supports two-factor authentication.</p>
        </div>
      </div>
      <button class="btn-primary" onclick={() => (app.showElyLogin = true)}>
        <Icon name="shield" size={16} /> Sign in with Ely.by
      </button>
      <p class="hint">Skins are served through authlib-injector, which is downloaded automatically on first launch.</p>
    </section>
  </div>

  <h3 class="mt-8 mb-3 text-xs font-semibold tracking-wider text-zinc-500 uppercase">Your accounts · {app.accounts.length}</h3>

  {#if app.accounts.length === 0}
    <div class="card p-10 text-center text-zinc-500">No accounts yet. Add one above to start playing.</div>
  {:else}
    <div class="space-y-2">
      {#each app.accounts as acc (acc.id)}
        {@const active = app.currentAccount?.id === acc.id}
        <div class="card flex animate-fade-in items-center gap-4 p-3.5 {active ? 'ring-2 ring-violet-400/50' : ''}">
          <Avatar username={acc.username} kind={acc.kind} size={44} />
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="truncate font-semibold">{acc.username}</span>
              <span
                class="rounded-md px-1.5 py-0.5 text-[10px] font-semibold uppercase ring-1
                  {acc.kind === 'elyby' ? 'bg-emerald-500/15 text-emerald-300 ring-emerald-400/30' : 'bg-zinc-500/15 text-zinc-300 ring-zinc-400/30'}"
              >
                {acc.kind === "elyby" ? "Ely.by" : "Offline"}
              </span>
              {#if active}<span class="text-xs text-violet-300">Selected</span>{/if}
            </div>
            <div class="selectable truncate font-mono text-[11px] text-zinc-500">{short(acc.uuid)}</div>
          </div>
          {#if !active}
            <button class="btn-ghost px-3 py-1.5 text-xs" onclick={() => app.selectAccount(acc.id)}>Use</button>
          {/if}
          <button class="btn-icon size-8 hover:text-rose-300" title="Remove" aria-label="Remove account" onclick={() => app.removeAccount(acc.id)}>
            <Icon name="trash" size={14} />
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>
