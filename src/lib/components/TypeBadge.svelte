<script lang="ts">
  import type { VersionType } from "../types";
  import { TYPE_BADGE, TYPE_LABEL } from "../util";

  let { type }: { type: VersionType | undefined } = $props();
</script>

{#if type}
  <span class="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold tracking-wide uppercase ring-1 {TYPE_BADGE[type]}">
    {TYPE_LABEL[type]}
  </span>
{/if}
