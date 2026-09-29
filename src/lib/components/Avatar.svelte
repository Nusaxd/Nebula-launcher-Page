<script lang="ts">
  import type { AccountKind } from "../types";

  let {
    username,
    kind = "offline",
    size = 40,
  }: { username: string; kind?: AccountKind; size?: number } = $props();

  let skinUrl = $derived(`https://skinsystem.ely.by/skins/${encodeURIComponent(username)}.png`);
  let skinOk = $state(false);

  $effect(() => {
    skinOk = false;
    if (kind !== "elyby") return;
    const img = new Image();
    const url = skinUrl;
    img.onload = () => {
      if (url === skinUrl) skinOk = true;
    };
    img.src = url;
  });

  const palette = [
    "from-violet-500 to-fuchsia-600",
    "from-sky-500 to-indigo-600",
    "from-emerald-400 to-teal-600",
    "from-rose-500 to-orange-500",
    "from-amber-400 to-orange-600",
    "from-cyan-400 to-blue-600",
  ];
  let gradient = $derived(
    palette[[...username].reduce((a, c) => a + c.charCodeAt(0), 0) % palette.length],
  );
</script>

{#if skinOk}
  <div
    class="relative shrink-0 overflow-hidden rounded-lg ring-1 ring-white/15"
    style="width:{size}px;height:{size}px"
  >
    <div
      class="absolute inset-0"
      style="background-image:url({skinUrl});background-size:{size * 8}px auto;background-position:-{size}px -{size}px;image-rendering:pixelated"
    ></div>
    <div
      class="absolute inset-0"
      style="background-image:url({skinUrl});background-size:{size * 8}px auto;background-position:-{size * 5}px -{size}px;image-rendering:pixelated"
    ></div>
  </div>
{:else}
  <div
    class="flex shrink-0 items-center justify-center rounded-lg bg-gradient-to-br font-bold text-white ring-1 ring-white/15 {gradient}"
    style="width:{size}px;height:{size}px;font-size:{size * 0.45}px"
  >
    {username.slice(0, 1).toUpperCase()}
  </div>
{/if}
