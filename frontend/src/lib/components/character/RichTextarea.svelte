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
    import CodeEditor from "./CodeEditor.svelte";

    let {
        value = $bindable(),
        label = undefined,
        placeholder,
        rows = 3,
        class: className,
        isZenMode = $bindable(false),
        isDirty = false,
        icon: Icon = undefined as any,
        useCodeEditor = false,
    } = $props();

    let zenTextarea: HTMLTextAreaElement;

    $effect(() => {
        if (isZenMode && !useCodeEditor) {
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
                <Maximize2 class="mr-1 h-3 w-3" /> 全屏{useCodeEditor ? "代码" : ""}编辑
            </Button>
        </div>
    {/if}

    <div class="relative">
        <Textarea
            bind:value
            {placeholder}
            class={cn(
                "resize-none transition-all duration-300 min-h-[80px]",
                "bg-background/50 border ring-1 ring-border/50 focus-visible:ring-primary/50",
                isDirty &&
                    "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5"
            )}
            {rows}
        />
    </div>
</div>

<Dialog.Root bind:open={isZenMode}>
    <Dialog.Content class="!max-w-none !w-[80vw] h-[95vh] flex flex-col p-6">
        <Dialog.Header class="flex-shrink-0">
            <Dialog.Title class="flex items-center gap-2">
                <Fullscreen class="h-4 w-4" />
                {label || "编辑内容"}
            </Dialog.Title>
            <Dialog.Description>代码编辑器模式，不编辑代码也行。</Dialog.Description>
        </Dialog.Header>
        <div class="flex-1 min-h-0 mt-4 overflow-hidden rounded-md border">
            {#if useCodeEditor}
                <CodeEditor bind:value class="border-0 rounded-none h-full" language="html" placeholder={placeholder || "输入代码..."} />
            {:else}
                <textarea
                    bind:this={zenTextarea}
                    bind:value
                    class="flex w-full h-full border-none bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-0 disabled:cursor-not-allowed disabled:opacity-50 resize-none font-mono text-base leading-relaxed p-6"
                    placeholder={placeholder || "输入内容..."}
                />
            {/if}
        </div>
        <Dialog.Footer class="flex-shrink-0 mt-4">
            <Button variant="outline" onclick={() => (isZenMode = false)}>
                完成
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
