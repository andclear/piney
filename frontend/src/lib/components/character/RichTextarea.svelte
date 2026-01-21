<script lang="ts">
    import { Textarea } from "$lib/components/ui/textarea";
    import { Switch } from "$lib/components/ui/switch";
    import { Label } from "$lib/components/ui/label";
    import { Button } from "$lib/components/ui/button";
    import { Badge } from "$lib/components/ui/badge";
    import {
        Maximize2,
        Minimize2,
        WrapText,
        Type,
        X,
        Fullscreen,
        Wand,
        ChevronDown,
        ChevronRight
    } from "lucide-svelte";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as Collapsible from "$lib/components/ui/collapsible";
    import { cn } from "$lib/utils";
    import { tick, type Snippet } from "svelte";
    import CodeEditor from "./CodeEditor.svelte";
    import HTMLRender from "$lib/components/render/HTMLRender.svelte";
    import { renderContent } from "$lib/utils/textRenderer";
    import { AiFeature } from "$lib/ai/types";
    import TextAiActions from "$lib/components/ai/TextAiActions.svelte";

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
        allowPreview = false,
        advancedPreview = false,
        previewProcessor = undefined as ((input: string) => string) | undefined,
        regexDebugInfo = undefined as { regex: string, flags: string, replace: string } | undefined,
        regexScripts = [] as any[],
        aiFeature = undefined as AiFeature | undefined,
        extraActions = undefined as Snippet | undefined
    } = $props();


    let zenTextarea: HTMLTextAreaElement;

    let isPreviewMode = $state(false);
    let isAdvancedPreviewMode = $state(false);
    let hasAppliedRegex = $state(false);
    let testInput = $state("");
    let isDebugOpen = $state(false);
    
    let previewContent = $derived.by(() => {
        if (!isPreviewMode) return "";
        if (isAdvancedPreviewMode && hasAppliedRegex && previewProcessor) {
             return previewProcessor(testInput);
        }
        return renderContent(value, regexScripts);
    });

    $effect(() => {
        if (!isAdvancedPreviewMode) {
            hasAppliedRegex = false;
        }
    });

    // Reset state when closing Zen Mode
    $effect(() => {
        if (!isZenMode) {
            isPreviewMode = false;
            isAdvancedPreviewMode = false;
            hasAppliedRegex = false;
            testInput = "";
            isDebugOpen = false;
        }
    });

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
            <div class="flex items-center gap-2">
                {#if extraActions}
                    {@render extraActions()}
                {/if}
                <Button
                    variant="ghost"
                    size="sm"
                    class="h-5 text-[10px] px-2 text-muted-foreground hover:text-primary"
                    onclick={() => (isZenMode = true)}
                >
                    <Maximize2 class="mr-1 h-3 w-3" /> 全屏{useCodeEditor ? "代码" : ""}编辑
                </Button>
            </div>
        </div>
    {/if}

    <div class="relative group/textarea">
        {#if aiFeature}
            <div class="absolute top-2 right-8 z-20 opacity-100 lg:opacity-0 lg:group-hover/textarea:opacity-100 transition-opacity duration-200">
                <TextAiActions bind:value feature={aiFeature} />
            </div>
        {/if}
        <Textarea
            bind:value
            {placeholder}
            class={cn(
                "resize-none transition-all duration-300 min-h-[80px] pr-12",
                "bg-background/50 border ring-1 ring-border/50 focus-visible:ring-primary/50",
                isDirty &&
                    "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5"
            )}
            {rows}
        />
    </div>
</div>

<Dialog.Root bind:open={isZenMode}>
    <Dialog.Content class={cn(
        "!max-w-none h-[95vh] flex flex-col p-6",
        isPreviewMode ? "!w-[95vw]" : "!w-[80vw]"
    )}>
        <Dialog.Header class="flex-shrink-0 flex-row items-center justify-between space-y-0">
            <div>
                <Dialog.Title class="flex items-center gap-2">
                <Fullscreen class="h-4 w-4" />
                {label || "编辑内容"}
            </Dialog.Title>
            <Dialog.Description>全屏编辑模式。</Dialog.Description>
            </div>
        </Dialog.Header>
        <div class={cn(
            "flex-1 min-h-0 mt-4 overflow-hidden gap-4",
            isPreviewMode ? "flex flex-col md:flex-row" : "flex flex-col"
        )}>
        <div class="flex-1 min-h-0 overflow-hidden flex flex-col gap-4">
             <div class="flex-1 min-h-0 overflow-hidden rounded-md border">
                {#if useCodeEditor}
                    <CodeEditor bind:value class="border-0 rounded-none h-full" language="html" placeholder={placeholder || "输入代码..."} toolbarActions={previewButton} />
                {:else}
                    <textarea
                        bind:this={zenTextarea}
                        bind:value
                        class="flex w-full h-full border-none bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-0 disabled:cursor-not-allowed disabled:opacity-50 resize-none font-mono text-base leading-relaxed p-6"
                        placeholder={placeholder || "输入内容..."}
                    />
                {/if}
            </div>
            {#if isPreviewMode && isAdvancedPreviewMode}
                <div class="h-1/3 min-h-[150px] flex flex-col gap-2 animate-in slide-in-from-bottom-2">
                    <Label>测试文本（输入原始文本后，记得点击预览窗口中的"应用正则"）</Label>
                    <textarea
                        bind:value={testInput}
                        class="flex-1 w-full rounded-md border-2 border-blue-500/50 focus:border-blue-500 bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 font-mono"
                        placeholder="在此输入原始文本进行测试..."
                    />
                </div>
            {/if}
        </div>
        {#if isPreviewMode}
            <div class="flex-1 min-h-0 overflow-hidden rounded-md border-2 border-primary/20 bg-muted/10 relative flex flex-col">
                 <div class="flex items-center justify-between px-3 py-2 border-b bg-background/50 min-h-[44px]">
                    <div class="flex items-center gap-2">
                        <Badge variant="outline" class="bg-background/80 backdrop-blur">预览</Badge>
                        {#if advancedPreview}
                             <div class="flex items-center gap-2 ml-2">
                                <Switch 
                                    id="adv-preview-switch"
                                    checked={isAdvancedPreviewMode}
                                    onCheckedChange={(v) => isAdvancedPreviewMode = v}
                                    class="scale-75"
                                />
                                <Label for="adv-preview-switch" class="text-xs cursor-pointer select-none">高级预览</Label>
                             </div>
                             {#if isAdvancedPreviewMode}
                                <Button 
                                    size="sm" 
                                    variant="secondary" 
                                    class="h-6 text-xs ml-2 px-2"
                                    onclick={() => hasAppliedRegex = true}
                                    disabled={!testInput || hasAppliedRegex}
                                >
                                    {hasAppliedRegex ? "已应用正则" : "应用正则"}
                                </Button>
                             {/if}
                        {/if}
                    </div>
                 </div>
                 {#if isAdvancedPreviewMode && hasAppliedRegex && regexDebugInfo}
                    <div class="border-b bg-background/30">
                        <Collapsible.Root bind:open={isDebugOpen}>
                            <Collapsible.Trigger class="flex items-center gap-2 w-full px-3 py-1.5 hover:bg-white/5 text-xs text-muted-foreground">
                                {#if isDebugOpen}
                                    <ChevronDown class="h-3 w-3" />
                                {:else}
                                    <ChevronRight class="h-3 w-3" />
                                {/if}
                                应用的正则表达式
                            </Collapsible.Trigger>
                            <Collapsible.Content>
                                <div class="px-3 pb-2 text-xs font-mono space-y-1">
                                    <div class="flex gap-2">
                                        <span class="text-muted-foreground w-8 shrink-0">Find:</span>
                                        <span class="text-amber-500 break-all">/{regexDebugInfo.regex}/{regexDebugInfo.flags}</span>
                                    </div>
                                </div>
                            </Collapsible.Content>
                        </Collapsible.Root>
                    </div>
                 {/if}
                 <div class="flex-1 overflow-y-auto p-4">
                    <HTMLRender content={previewContent} />
                 </div>
            </div>
        {/if}
        </div> 
        <Dialog.Footer class="flex-shrink-0 mt-4">
            <Button variant="outline" onclick={() => (isZenMode = false)}>
                完成
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>

{#snippet previewButton()}
    {#if allowPreview}
        <Button 
            variant="ghost" 
            size="sm" 
            class={cn(
                "h-7 text-xs gap-1.5 hover:bg-white/10 hover:text-white", 
                isPreviewMode && "bg-white/20 text-white"
            )}
            onclick={() => isPreviewMode = !isPreviewMode}
            title="实时预览"
        >
            <Wand class="h-3.5 w-3.5" />
            <span class="hidden sm:inline">预览</span>
        </Button>
    {/if}
{/snippet}
