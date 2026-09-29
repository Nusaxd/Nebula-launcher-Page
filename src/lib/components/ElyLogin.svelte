<script lang="ts">
  import { api } from "../api";
  import { app } from "../store.svelte";
  import { errorText } from "../util";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  let login = $state("");
  let password = $state("");
  let totp = $state("");
  let needTotp = $state(false);
  let error = $state("");
  let busy = $state(false);

  async function submit(e?: Event) {
    e?.preventDefault();
    error = "";
    busy = true;
    try {
      await app.loginEly(login, password, needTotp ? totp : null);
      app.showElyLogin = false;
    } catch (err) {
      const msg = errorText(err);
      if (/two.?factor/i.test(msg)) {
        needTotp = true;
        error = "This account is protected with two-factor authentication. Enter your 6-digit code.";
      } else {
        error = msg;
        if (needTotp && /invalid/i.test(msg)) error = "Invalid credentials or wrong two-factor code.";
      }
    } finally {
      busy = false;
    }
  }
</script>

<Modal title="Sign in with Ely.by" onclose={() => (app.showElyLogin = false)} width="max-w-md">
  <form class="space-y-4" onsubmit={submit}>
    <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/5 p-3 text-xs text-zinc-400">
      <Icon name="shield" size={20} class="shrink-0 text-emerald-300" />
      <span>
        Your password is sent only to <span class="text-zinc-200">authserver.ely.by</span>. The launcher stores an access
        token, never your password.
      </span>
    </div>

    <div>
      <label class="label" for="ely-login">Username or e-mail</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="ely-login" class="input" bind:value={login} autocomplete="username" autofocus disabled={needTotp} />
    </div>
    <div>
      <label class="label" for="ely-pass">Password</label>
      <input id="ely-pass" type="password" class="input" bind:value={password} autocomplete="current-password" disabled={needTotp} />
    </div>
    {#if needTotp}
      <div class="animate-slide-up">
        <label class="label" for="ely-totp">Two-factor code</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input id="ely-totp" class="input font-mono tracking-[0.4em]" bind:value={totp} inputmode="numeric" maxlength="8" placeholder="123456" autofocus />
      </div>
    {/if}

    {#if error}
      <div class="selectable rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs text-rose-200">{error}</div>
    {/if}

    <button type="submit" class="hidden">submit</button>
  </form>

  {#snippet footer()}
    <button class="mr-auto text-xs text-zinc-500 hover:text-zinc-200" onclick={() => api.openUrl("https://account.ely.by/register")}>
      Create an Ely.by account
    </button>
    <button class="btn-ghost" onclick={() => (app.showElyLogin = false)}>Cancel</button>
    <button class="btn-primary" disabled={busy || !login || !password || (needTotp && !totp)} onclick={() => submit()}>
      {busy ? "Signing in…" : "Sign in"}
    </button>
  {/snippet}
</Modal>
