<script lang="ts">
    import { Label } from "$lib/components/ui/label";
    import { cn } from "$lib/utils";
    import type { HTMLLabelAttributes } from "svelte/elements";

    let {
        isDirty = false,
        class: className,
        children,
        ...rest
    }: HTMLLabelAttributes & { isDirty?: boolean; children?: any } = $props();

    const dirtyStyles = ""; // No text style changes
</script>

<Label
    class={cn(
        "transition-colors duration-200 inline-flex items-center gap-1.5",
        className,
    )}
    {...rest}
>
    {@render children?.()}
    {#if isDirty}
        <span class="h-1.5 w-1.5 rounded-full bg-amber-500 animate-pulse"
        ></span>
    {/if}
</Label>
