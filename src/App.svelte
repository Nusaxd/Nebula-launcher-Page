<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "./lib/store.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import InstanceEditor from "./lib/components/InstanceEditor.svelte";
  import ElyLogin from "./lib/components/ElyLogin.svelte";
  import DeleteDialog from "./lib/components/DeleteDialog.svelte";
  import LibraryPage from "./lib/pages/LibraryPage.svelte";
  import VersionsPage from "./lib/pages/VersionsPage.svelte";
  import AccountsPage from "./lib/pages/AccountsPage.svelte";
  import ConsolePage from "./lib/pages/ConsolePage.svelte";
  import SettingsPage from "./lib/pages/SettingsPage.svelte";

  onMount(() => {
    void app.init();
  });
</script>

<div class="nebula-bg flex h-full text-sm">
  <Sidebar />
  <main class="relative flex min-w-0 flex-1 flex-col overflow-hidden">
    {#if !app.ready}
      <div class="flex flex-1 items-center justify-center text-zinc-500">Loading…</div>
    {:else if app.page === "library"}
      <LibraryPage />
    {:else if app.page === "versions"}
      <VersionsPage />
    {:else if app.page === "accounts"}
      <AccountsPage />
    {:else if app.page === "console"}
      <ConsolePage />
    {:else if app.page === "settings"}
      <SettingsPage />
    {/if}
  </main>
</div>

{#if app.editing}<InstanceEditor />{/if}
{#if app.showElyLogin}<ElyLogin />{/if}
{#if app.deleting}<DeleteDialog />{/if}
<Toasts />
