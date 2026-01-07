<script lang="ts">
    import { Textarea } from "$lib/components/ui/textarea";
    import { Label } from "$lib/components/ui/label";
    import { Button } from "$lib/components/ui/button";
    import {
        Maximize2,
        Minimize2,
        WrapText,
        Type,
        X,
        Fullscreen,
    } from "lucide-svelte";
    import * as Dialog from "$lib/components/ui/dialog";
    import { cn } from "$lib/utils";
    import { tick } from "svelte";

    let {
        value = $bindable(),
        label = undefined,
        placeholder,
        rows = 3,
        class: className,
        isZenMode = $bindable(false),
        isDirty = false,
        icon: Icon = undefined as any,
    } = $props();

    let zenTextarea: HTMLTextAreaElement;

    $effect(() => {
        if (isZenMode) {
            tick().then(() => {
                if (zenTextarea) {
                    zenTextarea.scrollTop = 0;
                    zenTextarea.setSelectionRange(0, 0);
                    zenTextarea.focus();
                }
            });
        }
    });
</script>

<div class={cn("space-y-2 relative group/dirty", className)}>
    {#if isDirty}
        <span
            class="absolute -right-1 -top-1 w-2 h-2 rounded-full bg-amber-500 z-10 shadow-sm"
            title="未保存的更改"
        ></span>
    {/if}
    {#if label}
        <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
                {#if Icon}
                    <Icon class="h-4 w-4 text-muted-foreground" />
                {/if}
                <Label>{label}</Label>
            </div>
            <Button
                variant="ghost"
                size="sm"
                class="h-5 text-[10px] px-2 text-muted-foreground hover:text-primary"
                onclick={() => (isZenMode = true)}
            >
                <Maximize2 class="mr-1 h-3 w-3" /> 全屏编辑
            </Button>
        </div>
    {/if}

    <div class="relative">
        <Textarea
            bind:value
            {placeholder}
            class={cn(
                "resize-none transition-all duration-300 bg-background/50 focus:bg-background",
                isDirty &&
                    "border-amber-500/50 focus:border-amber-500 bg-amber-500/5",
                "min-h-[80px]",
            )}
            {rows}
        />
    </div>
</div>

<Dialog.Root bind:open={isZenMode}>
    <Dialog.Content class="max-w-[90vw] w-[1200px] h-[90vh] flex flex-col p-6">
        <Dialog.Header class="flex-shrink-0">
            <Dialog.Title class="flex items-center gap-2">
                <Fullscreen class="h-4 w-4" />
                {label || "编辑内容"}
            </Dialog.Title>
            <Dialog.Description>全屏专注模式。按 ESC 退出。</Dialog.Description>
        </Dialog.Header>
        <div class="flex-1 min-h-0 mt-4">
            <textarea
                bind:this={zenTextarea}
                bind:value
                class="flex w-full h-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 resize-none font-mono text-base leading-relaxed p-6"
                placeholder={placeholder || "输入内容..."}
            />
        </div>
        <Dialog.Footer class="flex-shrink-0 mt-4">
            <Button variant="outline" onclick={() => (isZenMode = false)}>
                完成
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
