<script lang="ts">
    import { Select as SelectPrimitive } from "bits-ui";
    import { cn } from "$lib/utils.js";

    let {
        ref = $bindable(null),
        class: className,
        placeholder,
        ...restProps
    }: SelectPrimitive.ValueProps = $props();
</script>

<SelectPrimitive.Value
    bind:ref
    class={cn("data-[placeholder]:text-muted-foreground", className)}
    data-slot="select-value"
    {placeholder}
    {...restProps}
/>
