<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import * as Select from "$lib/components/ui/select";
    import { Switch } from "$lib/components/ui/switch";
    import { Textarea } from "$lib/components/ui/textarea";
    import { Label } from "$lib/components/ui/label";
    import { Button } from "$lib/components/ui/button";
    import { Badge } from "$lib/components/ui/badge";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { untrack } from "svelte";
    import { cn } from "$lib/utils";
    import { 
        ChevronDown, 
        ChevronUp, 
        Trash2, 
        MoreVertical,
        Settings2,
        Terminal,
        GripVertical,
        Maximize2 
    } from "lucide-svelte";
    import * as Tooltip from "$lib/components/ui/tooltip";
    import RichTextarea from "$lib/components/character/RichTextarea.svelte";
    import { processContentWithScripts, type RegexScript } from "$lib/utils/regexProcessor";
    import { renderContent } from "$lib/utils/textRenderer";

    let { 
        script = $bindable(),
        onDelete,
        isOpen = $bindable(),
        // isDirty prop from parent is aggregate, but we can compute granular here too.
        // Actually parent aggregate is good for list status, local granular is good for fields.
        isDirty = false, 
        lastSaved = 0
    } = $props<{
        script: any,
        onDelete: () => void,
        isOpen?: boolean,
        isDirty?: boolean,
        isDirty?: boolean,
        lastSaved?: number
    }>();

    let isAdvanced = $state(false);

    // --- Granular Dirty Checking & Local State ---
    // We use local state for inputs to ensure stable comparison against snapshot and avoid direct deep mutation issues.
    
    // Initial Snapshot
    let originalScript = $state(JSON.parse(JSON.stringify(script)));

    // Reset snapshot logic
    $effect(() => {
        const _ls = lastSaved;
        const _id = script.id;
        untrack(() => {
            originalScript = JSON.parse(JSON.stringify(script));
        });
    });

    // Local State Variables
    let localName = $state(script.scriptName || "");
    let localDisabled = $state(!!script.disabled);
    let localFind = $state(script.findRegex || "");
    let localReplace = $state(script.replaceString || "");
    let localSubRegex = $state(script.substituteRegex || 0);
    let localTrim = $state(script.trimStrings || []);
    let localPlacement = $state(script.placement || [2]);
    let localMinDepth = $state(script.minDepth);
    let localMaxDepth = $state(script.maxDepth);
    let localMd = $state(!!script.markdownOnly);
    let localPmt = $state(!!script.promptOnly);
    let localRun = $state(!!script.runOnEdit);

    // Sync back to script prop
    $effect(() => { script.scriptName = localName; });
    $effect(() => { script.disabled = localDisabled; });
    $effect(() => { script.findRegex = localFind; });
    $effect(() => { script.replaceString = localReplace; });
    $effect(() => { script.substituteRegex = localSubRegex; });
    $effect(() => { script.trimStrings = localTrim; });
    $effect(() => { script.placement = localPlacement; });
    $effect(() => { script.minDepth = localMinDepth; });
    $effect(() => { script.maxDepth = localMaxDepth; });
    $effect(() => { script.markdownOnly = localMd; });
    $effect(() => { script.promptOnly = localPmt; });
    $effect(() => { script.runOnEdit = localRun; });

    // Computed Dirty States
    let isNameDirty = $derived(localName !== (originalScript.scriptName || ""));
    let isDisabledDirty = $derived(localDisabled !== !!originalScript.disabled);
    let isFindDirty = $derived(localFind !== (originalScript.findRegex || ""));
    let isReplaceDirty = $derived(localReplace !== (originalScript.replaceString || ""));
    let isSubRegexDirty = $derived(localSubRegex !== (originalScript.substituteRegex || 0));
    
    let isTrimDirty = $derived.by(() => {
        const orig = originalScript.trimStrings || [];
        return JSON.stringify(localTrim) !== JSON.stringify(orig);
    });
    
    let isPlacementDirty = $derived.by(() => {
        const curr = [...(localPlacement || [])].sort();
        const orig = [...(originalScript.placement || [])].sort();
        return JSON.stringify(curr) !== JSON.stringify(orig);
    });

    let isMinDepthDirty = $derived(localMinDepth !== originalScript.minDepth);
    let isMaxDepthDirty = $derived(localMaxDepth !== originalScript.maxDepth);
    
    let isMdDirty = $derived(localMd !== !!originalScript.markdownOnly);
    let isPmtDirty = $derived(localPmt !== !!originalScript.promptOnly);
    let isRunDirty = $derived(localRun !== !!originalScript.runOnEdit);

    let isAnyDirty = $derived(
        isNameDirty || isDisabledDirty || isFindDirty || isReplaceDirty || 
        isSubRegexDirty || isTrimDirty || isPlacementDirty || 
        isMinDepthDirty || isMaxDepthDirty || isMdDirty || isPmtDirty || isRunDirty
    );

    // Flag handlers
    function handleTrimChange(e: Event) {
        const val = (e.target as HTMLTextAreaElement).value;
        localTrim = val.split('\n').filter(Boolean);
    }

    // Placement Options
    const PLACEMENTS = [
        { value: 1, label: "用户输入", general: true },
        { value: 2, label: "AI输出", general: true },
        { value: 3, label: "快捷命令", general: false },
        { value: 5, label: "世界信息", general: false },
        { value: 6, label: "推理", general: false }
    ];

    // Substitute Regex Options
    const SUBSTITUTE_OPTIONS = [
        { value: 0, label: "不替换" },
        { value: 1, label: "原始" },
        { value: 2, label: "转义" }
    ];

    function togglePlacement(value: number, checked: boolean) {
        if (!localPlacement) localPlacement = [];
        if (checked) {
            if (!localPlacement.includes(value)) {
                localPlacement = [...localPlacement, value];
            }
        } else {
            localPlacement = localPlacement.filter((p: number) => p !== value);
        }
    }



    // Helper to get placement summary
    let placementSummary = $derived(() => {
        if (!script.placement || script.placement.length === 0) return "未设置";
        const labels = script.placement.map((p: number) => {
            const opt = PLACEMENTS.find(o => o.value === p);
            return opt ? (opt.label.split(' ')[0]) : p;
        });
        return labels.join(", ");
    });
    
    // Preview Processor for Advanced Preview
    function processPreview(input: string): string {
        if (!input) return "";
        let processed = input;
        
        // 1. Trim Strings (Local)
        if (localTrim && localTrim.length > 0) {
            for (const trimStr of localTrim) {
                 // Escape regex special chars in trim string? Usually trim strings are literal?
                 // ST logic usually trims literally per line or globally?
                 // Simple literal replacement for now
                 processed = processed.replaceAll(trimStr, "");
            }
        }
        
        // 2. Apply Regex (Using local values)
        // Construct temp script
        const tempScript: RegexScript = {
            id: "preview",
            scriptName: "Preview",
            findRegex: localFind,
            replaceString: localReplace,
            disabled: false,
            // flags map to what? processContentWithScripts handles it if we follow format.
        };
        
        // Use standard processor for regex part
        processed = processContentWithScripts(processed, [tempScript]);
        
        // 3. Render Markdown/HTML result
        return renderContent(processed, []);
    }
</script>

<div class={cn(
    "border rounded-lg bg-card text-card-foreground shadow-sm group transition-all hover:border-primary/50 relative"
)}>
    {#if isAnyDirty}
        <span class="absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full bg-amber-500 shadow-sm z-20 animate-pulse border border-background"></span>
    {/if}
    <!-- Header -->
    <div 
        class="flex items-center gap-3 p-3 select-none cursor-pointer"
        onclick={() => isOpen = !isOpen}
    >
        <!-- Drag Handle (Visual only here, actual drag logic in parent) -->
        <div class="cursor-grab active:cursor-grabbing text-muted-foreground hover:text-foreground">
            <GripVertical class="h-4 w-4" />
        </div>

        <!-- Enable Switch (Left) -->
        <div class="flex items-center gap-2" onclick={(e) => e.stopPropagation()}>
             <Switch 
                checked={!localDisabled} 
                onCheckedChange={(v) => localDisabled = !v}
                class="data-[state=checked]:bg-primary data-[state=unchecked]:bg-input scale-75 origin-left"
            />
        </div>

        <!-- Collapsible Toggle Content -->
        <div class="flex items-center gap-2 flex-1 text-left min-w-0">
            <div class={cn("transition-transform duration-200", isOpen && "rotate-180")}>
                <ChevronDown class="h-4 w-4 text-muted-foreground" />
            </div>
            
            <div class="flex items-center gap-2 min-w-0">
                <span class={cn(
                    "font-medium truncate max-w-[150px] md:max-w-[300px]", 
                    localDisabled && "text-muted-foreground line-through decoration-muted-foreground/50",
                    isAnyDirty && "text-amber-500"
                )}>
                    {localName || "未命名脚本"}
                </span>
            </div>
        </div>

        <!-- Quick Actions -->
        <div class="flex items-center gap-2">
             <!-- Mode Switch (Segmented) -->
             <div 
                class="flex items-center bg-secondary/50 rounded-lg p-0.5 border border-border/50 h-7 mr-2"
                onclick={(e) => e.stopPropagation()}
             >
                 <button 
                    class={cn("px-2 text-[10px] rounded-md transition-all h-full flex items-center", !isAdvanced ? "bg-background shadow-sm text-foreground font-medium" : "text-muted-foreground hover:text-foreground")}
                    onclick={() => isAdvanced = false}
                 >
                    通用
                 </button>
                 <button 
                    class={cn("px-2 text-[10px] rounded-md transition-all h-full flex items-center", isAdvanced ? "bg-background shadow-sm text-foreground font-medium" : "text-muted-foreground hover:text-foreground")}
                    onclick={() => isAdvanced = true}
                 >
                    高级
                 </button>
             </div>

            <Button 
                variant="ghost" 
                size="icon" 
                class="h-8 w-8 text-destructive hover:text-destructive hover:bg-destructive/10" 
                onclick={(e) => {
                    e.stopPropagation();
                    onDelete();
                }}
            >
                <Trash2 class="h-4 w-4" />
            </Button>
        </div>
    </div>

    <!-- Content -->
    {#if isOpen}
        <div class="p-4 border-t space-y-4 animate-in slide-in-from-top-2 duration-200">


            <div class="grid gap-4 sm:grid-cols-2">
                <!-- Script Name -->
                <div class="space-y-2 sm:col-span-2">
                    <Label>脚本名称</Label>
                    <Input 
                        bind:value={localName} 
                        placeholder="例如: 移除多余空行" 
                        class={cn(
                            "flex-1 bg-background/50 border ring-1 ring-border/50 focus-visible:ring-primary/50",
                            isNameDirty && "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5"
                        )}
                    />
                </div>

                <!-- Find Regex -->
                <div class="space-y-2 sm:col-span-2">
                    <div class="flex items-center justify-between">
                         <Label>正则表达式</Label>
                         {#if isAdvanced}
                            <div class="text-xs text-muted-foreground">
                                宏: 
                                <select 
                                    class="bg-transparent border-none outline-none text-primary cursor-pointer hover:underline"
                                    bind:value={script.substituteRegex}
                                >
                                    {#each SUBSTITUTE_OPTIONS as opt}
                                        <option value={opt.value}>{opt.label}</option>
                                    {/each}
                                </select>
                            </div>
                         {/if}
                    </div>
                    <Textarea 
                        bind:value={localFind} 
                        class={cn(
                            "font-mono text-xs min-h-[60px] bg-background/50 border ring-1 ring-border/50 focus-visible:ring-primary/50", 
                            isFindDirty && "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5"
                        )}
                        rows={2}
                        placeholder="在这里写正则表达式..."
                    />
                </div>

                <!-- Replace String -->
                <div class="space-y-2 sm:col-span-2">
                    <RichTextarea 
                        bind:value={localReplace} 
                        label="替换为"
                        class="font-mono text-xs" 
                        placeholder="在这里写替换内容..."
                        rows={3}
                        isDirty={isReplaceDirty}
                        useCodeEditor={true}
                        allowPreview={true}
                        advancedPreview={true}
                        previewProcessor={processPreview}
                        regexDebugInfo={{
                            regex: localFind,
                            flags: "g", // Assuming 'g' for now, or fetch from where it is stored? In ST it's usually just g or none?
                            replace: localReplace
                        }}
                    />
                </div>

                <!-- Trim Strings (Advanced Only) -->
                {#if isAdvanced}
                    <div class="space-y-2 sm:col-span-2">
                        <Label class={cn(isTrimDirty && "text-amber-500")}>修剪掉</Label>
                        <Textarea 
                            value={localTrim.join('\n')} 
                            oninput={handleTrimChange}
                            class={cn(
                                "font-mono text-xs min-h-[60px] bg-background/50 border ring-1 ring-border/50 focus-visible:ring-primary/50",
                                isTrimDirty && "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5"
                            )} 
                            placeholder="每行一个字符串"
                        />
                        <p class="text-[10px] text-muted-foreground">在执行正则替换前，先移除这些内容。</p>
                    </div>
                {/if}

                <!-- Placement -->
                <div class="space-y-2 sm:col-span-2">
                    <Label class={cn(isPlacementDirty && "text-amber-500")}>作用范围</Label>
                    <div class={cn("border rounded-md p-3", isPlacementDirty && "border-amber-500/50 bg-amber-500/5")}>
                        <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
                            {#each PLACEMENTS as p}
                                {#if isAdvanced || p.general}
                                    {@const isOptionDirty = (localPlacement?.includes(p.value) ?? false) !== (originalScript.placement?.includes(p.value) ?? false)}
                                    <div class="flex items-center space-x-2">
                                        <Checkbox 
                                            id={`placement-${script.id}-${p.value}`}
                                            checked={localPlacement?.includes(p.value)}
                                            onCheckedChange={(checked) => togglePlacement(p.value, checked as boolean)}
                                        />
                                        <Label 
                                            for={`placement-${script.id}-${p.value}`} 
                                            class={cn(
                                                "text-xs font-normal cursor-pointer text-muted-foreground peer-aria-checked:text-foreground",
                                                isOptionDirty && "text-amber-500 font-medium"
                                            )}
                                        >
                                            {p.label}
                                        </Label>
                                    </div>
                                {/if}
                            {/each}
                        </div>
                    </div>
                </div>

                <!-- Depth -->
                <div class="space-y-2">
                    <Label>最小深度</Label>
                    <Input 
                        type="number" 
                        bind:value={localMinDepth} 
                        placeholder="无限制" 
                        class={cn(
                            "bg-background/50 border ring-1 ring-border/50 focus-visible:ring-primary/50",
                            isMinDepthDirty && "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5"
                        )}
                    />
                </div>
                <div class="space-y-2">
                    <Label>最大深度</Label>
                    <Input 
                        type="number" 
                        bind:value={localMaxDepth} 
                        placeholder="无限制" 
                        class={cn(
                            "bg-background/50 border ring-1 ring-border/50 focus-visible:ring-primary/50",
                            isMaxDepthDirty && "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5"
                        )}
                    />
                </div>

                <!-- Flags (Switch Group) -->
                <div class="sm:col-span-2 space-y-3 pt-2">
                    {#if isAdvanced}
                         <div class="flex flex-wrap gap-6 border-t pt-3">
                            <div class="flex items-center space-x-2">
                                <Checkbox 
                                    id={`adv-md-${script.id}`} 
                                    checked={localMd} 
                                    onCheckedChange={(v) => localMd = !!v} 
                                />
                                <Label class={cn("text-sm cursor-pointer font-normal", isMdDirty && "text-amber-500")} for={`adv-md-${script.id}`}>仅格式显示 (Markdown Only)</Label>
                            </div>
                            <div class="flex items-center space-x-2">
                                <Checkbox 
                                    id={`adv-pmt-${script.id}`} 
                                    checked={localPmt} 
                                    onCheckedChange={(v) => localPmt = !!v} 
                                />
                                <Label class={cn("text-sm cursor-pointer font-normal", isPmtDirty && "text-amber-500")} for={`adv-pmt-${script.id}`}>仅格式提示词 (Prompt Only)</Label>
                            </div>
                            <div class="flex items-center space-x-2">
                                <Checkbox 
                                    id={`adv-run-${script.id}`} 
                                    checked={localRun} 
                                    onCheckedChange={(v) => localRun = !!v} 
                                />
                                <Label class={cn("text-sm cursor-pointer font-normal", isRunDirty && "text-amber-500")} for={`adv-run-${script.id}`}>在编辑时运行 (Run On Edit)</Label>
                            </div>
                        </div>
                    {:else}
                         <!-- General Mode: Only combined "Transient" toggles -->
                         <div class="flex flex-wrap gap-4">
                            <div class="flex items-center space-x-2">
                                <Checkbox 
                                    id={`temp-md-${script.id}`} 
                                    checked={localMd} 
                                    onCheckedChange={(v) => localMd = !!v} 
                                />
                                <Label for={`temp-md-${script.id}`} class={cn("text-sm font-normal cursor-pointer", isMdDirty && "text-amber-500")}>仅格式显示</Label>
                            </div>
                            <div class="flex items-center space-x-2">
                                <Checkbox 
                                    id={`temp-prompt-${script.id}`} 
                                    checked={localPmt} 
                                    onCheckedChange={(v) => localPmt = !!v} 
                                />
                                <Label for={`temp-prompt-${script.id}`} class={cn("text-sm font-normal cursor-pointer", isPmtDirty && "text-amber-500")}>仅格式提示词</Label>
                            </div>
                         </div>
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</div>
