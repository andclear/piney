<script lang="ts">
    import type { HTMLTextareaAttributes } from "svelte/elements";
    import { cn, type WithElementRef } from "$lib/utils.js";

    type Props = WithElementRef<HTMLTextareaAttributes>;

    let {
        ref = $bindable(null),
        value = $bindable(),
        class: className,
        "data-slot": dataSlot = "textarea",
        ...restProps
    }: Props = $props();
</script>

<textarea
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
        "border-input bg-background selection:bg-primary dark:bg-input/30 selection:text-primary-foreground ring-offset-background placeholder:text-muted-foreground flex min-h-[60px] w-full min-w-0 rounded-md border px-3 py-2 text-base shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
        "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
        className,
    )}
    bind:value
    {...restProps}
></textarea>
