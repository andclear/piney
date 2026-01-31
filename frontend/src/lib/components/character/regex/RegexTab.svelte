<script lang="ts">
    // Trigger HMR update
    import { Input } from "$lib/components/ui/input";
    import { Button } from "$lib/components/ui/button";
    import { Label } from "$lib/components/ui/label";
    import { Plus, Search, Regex, ArrowUpDown, FileJson } from "lucide-svelte";
    import RegexItem from "./RegexItem.svelte";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { cn } from "$lib/utils";
    import { untrack } from "svelte";
    import { toast } from "svelte-sonner";
    import { beforeNavigate, goto } from "$app/navigation";
    import * as Dialog from "$lib/components/ui/dialog";
    import { AlertTriangle } from "lucide-svelte";
    import { dndzone, TRIGGERS } from "svelte-dnd-action";
    import { flip } from "svelte/animate";

    let {
        data = $bindable({ extensions: {} }),
        lastSaved = 0,
        onChange = () => {}
    } = $props<{
        data: any,
        lastSaved?: number,
        onChange?: () => void
    }>();

    // Ensure regex_scripts array exists
    $effect(() => {
        // Init snapshot on mount
        if (originalScripts === null && data.extensions?.regex_scripts) {
             updateSnapshot();
        }
    });

    let searchTerm = $state("");
    let openScripts: Record<string, boolean> = $state({});
    const FLIP_DURATION_MS = 200;
    const TOUCH_DELAY_MS = 300; // 长按300ms后才能拖拽（移动端防误触）
    let fileInput: HTMLInputElement;

    // 拖拽专用状态（防止拖拽过程中触发脏状态）
    let dndItems: any[] = $state([]);
    let isDragging = $state(false);

    // 当任意条目展开时禁用拖拽
    let isDragDisabled = $derived(Object.values(openScripts).some(v => v));

    // --- Dirty Checking Logic ---
    let originalScripts: any = $state(null);
    
    function updateSnapshot() {
        originalScripts = JSON.parse(JSON.stringify(data.extensions.regex_scripts || []));
    }

    $effect(() => {
        if (lastSaved) {
            updateSnapshot();
        }
    });

    // Compute dirty items
    let dirtyScriptIds = $derived.by(() => {
        const current = data.extensions.regex_scripts || [];
        const original = originalScripts || [];
        const dirtySet = new Set<string>();

        // Check for modifications
        current.forEach((script: any) => {
            if (!script.id) return;
            const orig = original.find((s: any) => s.id === script.id);
            if (!orig) {
                dirtySet.add(script.id);
            } else {
                if (JSON.stringify(script) !== JSON.stringify(orig)) {
                    dirtySet.add(script.id);
                }
            }
        });

        return dirtySet;
    });

    let isGlobalDirty = $derived.by(() => {
        const current = data.extensions.regex_scripts || [];
        const original = originalScripts || [];
        return JSON.stringify(current) !== JSON.stringify(original);
    });

    // Navigation Guard
    let showUnsavedDialog = $state(false);
    let pendingTarget: string | null = null;
    let bypassCheck = false;

    beforeNavigate(({ cancel, to }) => {
        if (bypassCheck) return;
        if (isGlobalDirty) {
            cancel();
            pendingTarget = to?.url?.href || null;
            showUnsavedDialog = true;
        }
    });

    $effect(() => {
        const handleBeforeUnload = (e: BeforeUnloadEvent) => {
            if (isGlobalDirty) {
                e.preventDefault();
                e.returnValue = "";
            }
        };
        window.addEventListener("beforeunload", handleBeforeUnload);
        return () => window.removeEventListener("beforeunload", handleBeforeUnload);
    });

    function confirmDiscard() {
        bypassCheck = true;
        showUnsavedDialog = false;
        if (pendingTarget) {
            goto(pendingTarget);
        }
    }

    // Scripts List (Reactive)
    let scripts = $derived(data.extensions?.regex_scripts || []);

    // Filtered Scripts
    let filteredScripts = $derived.by(() => {
        if (!searchTerm) return scripts;
        const low = searchTerm.toLowerCase();
        return scripts.filter((s: any) => 
            (s.scriptName || "").toLowerCase().includes(low) || 
            (s.findRegex || "").toLowerCase().includes(low)
        );
    });

    function generateUUID() {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
            var r = Math.random() * 16 | 0, v = c == 'x' ? r : (r & 0x3 | 0x8);
            return v.toString(16);
        });
    }

    function addScript() {
        const newScript = {
            id: generateUUID(),
            scriptName: "新正则",
            findRegex: "",
            replaceString: "",
            trimStrings: [],
            placement: [2],
            disabled: false,
            markdownOnly: true,
            promptOnly: false,
            runOnEdit: true,
            substituteRegex: 0,
            minDepth: null,
            maxDepth: null
        };
        
        if (!data.extensions.regex_scripts) data.extensions.regex_scripts = [];
        data.extensions.regex_scripts = [...data.extensions.regex_scripts, newScript];
        onChange();
        toast.success("已添加新正则");
    }

    function triggerImport() {
        fileInput?.click();
    }

    function handleImportScripts(event: Event) {
        const input = event.target as HTMLInputElement;
        const file = input.files?.[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = (e) => {
            try {
                const content = e.target?.result as string;
                const json = JSON.parse(content);
                
                // Allow single object or array
                const items = Array.isArray(json) ? json : [json];
                let importedCount = 0;

                const validScripts = items.filter((item: any) => {
                    return (
                        typeof item === 'object' &&
                        item !== null &&
                        'scriptName' in item &&
                        'findRegex' in item
                    );
                }).map((item: any) => {
                    return {
                        id: generateUUID(),
                        scriptName: item.scriptName || "导入的正则",
                        findRegex: item.findRegex || "",
                        replaceString: item.replaceString || "",
                        trimStrings: Array.isArray(item.trimStrings) ? item.trimStrings : [],
                        placement: Array.isArray(item.placement) ? item.placement : [2],
                        disabled: item.disabled === true,
                        markdownOnly: item.markdownOnly !== false, // Default true if missing? or strict?
                        promptOnly: item.promptOnly === true,
                        runOnEdit: item.runOnEdit !== false,
                        substituteRegex: typeof item.substituteRegex === 'number' ? item.substituteRegex : 0,
                        minDepth: typeof item.minDepth === 'number' ? item.minDepth : null,
                        maxDepth: typeof item.maxDepth === 'number' ? item.maxDepth : null
                    };
                });

                if (validScripts.length === 0) {
                    toast.error("未找到有效的正则");
                    return;
                }

                if (!data.extensions.regex_scripts) data.extensions.regex_scripts = [];
                data.extensions.regex_scripts = [...data.extensions.regex_scripts, ...validScripts];
                onChange();
                toast.success(`成功导入 ${validScripts.length} 个正则`);
                
            } catch (err) {
                console.error("Import failed", err);
                toast.error("导入失败：无效的 JSON 文件");
            } finally {
                input.value = "";
            }
        };
        reader.readAsText(file);
    }

    function deleteScript(scriptId: string) {
        data.extensions.regex_scripts = data.extensions.regex_scripts.filter((s: any) => s.id !== scriptId);
        onChange();
        toast.success("正则已删除");
    }

    // Drag and Drop Logic (svelte-dnd-action)
    // 记录拖拽前的顺序，用于判断是否真正改变了位置
    let orderBeforeDrag: string[] = [];

    function handleDndConsider(e: CustomEvent<{ items: any[], info: { trigger: string } }>) {
        // 开始拖拽时记录原始顺序并初始化拖拽状态
        if (e.detail.info.trigger === TRIGGERS.DRAG_STARTED) {
            orderBeforeDrag = scripts.map((s: any) => s.id);
            isDragging = true;
        }
        // 只更新拖拽专用状态，不触发脏检查
        dndItems = e.detail.items;
    }

    function handleDndFinalize(e: CustomEvent<{ items: any[], info: { trigger: string } }>) {
        isDragging = false;
        
        // 只有顺序真正改变时才更新数据并触发 onChange
        const newOrder = e.detail.items.map((s: any) => s.id);
        const orderChanged = orderBeforeDrag.length > 0 && 
            (orderBeforeDrag.length !== newOrder.length || 
             orderBeforeDrag.some((id, i) => id !== newOrder[i]));
        
        if (orderChanged) {
            // 只有顺序改变时才同步到真实数据
            data.extensions.regex_scripts = e.detail.items.map(item => {
                // 移除 svelte-dnd-action 添加的内部属性
                const { isDndShadowItem, ...cleanItem } = item;
                return cleanItem;
            });
            onChange();
        }
        
        dndItems = [];
        orderBeforeDrag = [];
    }

    // 获取用于显示的列表（拖拽中用 dndItems，否则用 scripts）
    let displayScripts = $derived(isDragging ? dndItems : scripts);
</script>

<div class="h-full flex flex-col space-y-4">
    <!-- Toolbar -->
    <div class="flex items-center justify-between gap-4 px-1">
        <div class="relative flex-1 max-w-sm">
            <Search class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
                type="search"
                placeholder="搜索正则名称..."
                class="pl-9 h-9"
                bind:value={searchTerm}
            />
        </div>
        <div class="flex items-center gap-2">
            <input
                bind:this={fileInput}
                type="file"
                accept=".json"
                class="hidden"
                onchange={handleImportScripts}
            />
            <Button size="sm" class="gap-2 border-dashed border-muted-foreground/50 bg-background text-foreground hover:bg-muted" onclick={triggerImport} variant="outline">
                <FileJson class="h-4 w-4" />
                <span class="hidden sm:inline">导入正则</span>
            </Button>
            <Button size="sm" class="gap-2 border-primary bg-background text-foreground hover:bg-primary/10" onclick={addScript} variant="outline">
                <Plus class="h-4 w-4" />
                <span class="hidden sm:inline">添加正则</span>
            </Button>
        </div>
    </div>

    <!-- List -->
    <ScrollArea class="flex-1 -mx-2 px-2">
        <div class="space-y-3 p-3 pb-8">
            {#if filteredScripts.length === 0}
                <div class="flex flex-col items-center justify-center py-12 text-muted-foreground text-sm border border-dashed rounded-lg bg-muted/30">
                    <Regex class="h-8 w-8 mb-2 opacity-50" />
                    {#if searchTerm}
                        <p>未找到匹配的正则</p>
                    {:else}
                        <p>暂无正则</p>
                    {/if}
                </div>
            {:else if searchTerm}
                <!-- 搜索模式：禁用拖拽 -->
                {#each filteredScripts as script (script.id)}
                    {@const realIndex = scripts.findIndex((s: any) => s.id === script.id)}
                    <div class="transition-all duration-200 relative" class:z-20={openScripts[script.id]}>
                        <!-- svelte-ignore binding_property_non_reactive -->
                        <RegexItem 
                            bind:script={data.extensions.regex_scripts[realIndex]} 
                            bind:isOpen={openScripts[script.id]}
                            isDirty={dirtyScriptIds.has(script.id)}
                            {lastSaved}
                            onDelete={() => deleteScript(script.id)}
                        />
                    </div>
                {/each}
            {:else}
                <!-- 正常模式：启用拖拽 -->
                <div
                    use:dndzone={{
                        items: displayScripts,
                        flipDurationMs: FLIP_DURATION_MS,
                        delayTouchStart: TOUCH_DELAY_MS,
                        dragDisabled: isDragDisabled,
                        dropTargetStyle: {},
                        type: 'regex-scripts'
                    }}
                    onconsider={handleDndConsider}
                    onfinalize={handleDndFinalize}
                    class="space-y-3"
                >
                    {#each displayScripts as script (script.id)}
                        {@const realIndex = script.isDndShadowItem ? -1 : data.extensions.regex_scripts.findIndex((s: any) => s.id === script.id)}
                        <div
                            animate:flip={{ duration: FLIP_DURATION_MS }}
                            class={cn(
                                "transition-all duration-200 relative",
                                script.isDndShadowItem && "h-16 rounded-xl border-2 border-dashed border-primary/50 bg-primary/5",
                                !script.isDndShadowItem && openScripts[script.id] ? "z-20" : "z-0 hover:!z-50"
                            )}
                        >
                            {#if !script.isDndShadowItem && realIndex !== -1}
                                <!-- svelte-ignore binding_property_non_reactive -->
                                <RegexItem 
                                    bind:script={data.extensions.regex_scripts[realIndex]} 
                                    bind:isOpen={openScripts[script.id]}
                                    isDirty={dirtyScriptIds.has(script.id)}
                                    {lastSaved}
                                    onDelete={() => deleteScript(script.id)}
                                />
                            {/if}
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    </ScrollArea>
    
    <!-- Footer Hint -->
    <div class="text-[10px] text-muted-foreground text-center border-t pt-2">
        {#if !searchTerm && scripts.length > 1}
            <span class="flex items-center justify-center gap-1">
                <ArrowUpDown class="h-3 w-3" /> 拖拽可调整正则执行顺序
            </span>
        {/if}
    </div>
</div>

<Dialog.Root bind:open={showUnsavedDialog}>
    <Dialog.Content>
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2 text-amber-600">
                <AlertTriangle class="h-5 w-5" /> 未保存的更改
            </Dialog.Title>
            <Dialog.Description>
                您有未保存的正则更改。离开页面将丢失这些更改。确定要离开吗？
            </Dialog.Description>
        </Dialog.Header>
        <Dialog.Footer>
            <Button variant="outline" onclick={() => (showUnsavedDialog = false)}>
                取消 (留在页面)
            </Button>
            <Button variant="destructive" onclick={confirmDiscard}>
                丢弃更改并离开
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
