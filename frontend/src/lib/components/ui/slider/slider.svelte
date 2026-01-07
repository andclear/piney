<script lang="ts">
    import { Slider as SliderPrimitive } from "bits-ui";
    import { cn } from "$lib/utils.js";

    let {
        ref = $bindable(null),
        value = $bindable([0]),
        class: className,
        ...restProps
    }: SliderPrimitive.RootProps = $props();
</script>

<SliderPrimitive.Root
    bind:ref
    bind:value
    class={cn(
        "relative flex w-full touch-none select-none items-center",
        className,
    )}
    {...restProps}
>
    <span
        class="relative h-2 w-full grow overflow-hidden rounded-full bg-secondary"
    >
        <SliderPrimitive.Range class="absolute h-full bg-primary" />
    </span>
    {#each value as _}
        <SliderPrimitive.Thumb
            class="block h-5 w-5 rounded-full border-2 border-primary bg-background ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
        />
    {/each}
</SliderPrimitive.Root>
