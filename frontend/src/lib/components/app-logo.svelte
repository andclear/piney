<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";
  import Squirrel from "@lucide/svelte/icons/squirrel";

  let { class: className, ...restProps }: HTMLAttributes<HTMLDivElement> =
    $props();
</script>

<div
  class={cn(
    "bg-primary text-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg",
    className,
  )}
  {...restProps}
>
  <!-- Piney Logo -->
  <Squirrel class="size-4" />
</div>
