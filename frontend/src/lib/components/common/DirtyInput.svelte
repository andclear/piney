<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import { cn } from "$lib/utils";
    import type { HTMLInputAttributes } from "svelte/elements";

    let {
        value = $bindable(),
        isDirty = false,
        class: className,
        ...rest
    }: HTMLInputAttributes & { isDirty?: boolean; value?: any } = $props();

    const dirtyStyles =
        "border-amber-500/50 bg-amber-500/10 focus-visible:ring-amber-500";
</script>

<div class="relative group w-full">
    <Input
        bind:value
        class={cn(
            "transition-all duration-200",
            className,
            isDirty && dirtyStyles,
        )}
        {...rest}
    />

    {#if isDirty}
        <span
            class="absolute top-0 right-0 -mt-1 -mr-1 h-2 w-2 rounded-full bg-amber-500 shadow-sm animate-in fade-in zoom-in duration-300"
        ></span>
    {/if}
</div>
