<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import DirtyInput from "$lib/components/common/DirtyInput.svelte";
    import { Button } from "$lib/components/ui/button";
    import { Label } from "$lib/components/ui/label";
    import { Plus, Globe, Search, ArrowUpDown, Download, ChevronDown, FileDown, FileUp, MoreHorizontal, Bot, Sparkles, Loader2 } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import { AiFeature } from "$lib/ai/types";
    import { AiService } from "$lib/ai/service";
    import * as Dialog from "$lib/components/ui/dialog";
    import { Textarea } from "$lib/components/ui/textarea";
    import WorldInfoEntry from "./WorldInfoEntry.svelte";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { cn } from "$lib/utils";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
    import ImportWorldInfoDialog from "./ImportWorldInfoDialog.svelte";
    import ExportWorldInfoDialog from "./ExportWorldInfoDialog.svelte";
    import { type CharacterBookEntry } from "$lib/worldInfoConverter";
    import { untrack } from "svelte";
    import { dndzone, TRIGGERS } from "svelte-dnd-action";
    import { flip } from "svelte/animate";

    import DirtyLabel from "$lib/components/common/DirtyLabel.svelte";

    const FLIP_DURATION_MS = 200;
    const TOUCH_DELAY_MS = 300; // 长按300ms后才能拖拽（移动端防误触）

    // 拖拽专用状态（防止拖拽过程中触发脏状态）
    let dndItems: any[] = $state([]);
    let isDragging = $state(false);

    // 当任意条目展开时禁用拖拽
    let isDragDisabled = $derived(openEntries && Object.values(openEntries).some(v => v));

    let {
        data = $bindable({ character_book: { entries: [] }, extensions: {} }),
        lastSaved = 0,
        onChange = () => {},
        mode = "character", // "character" | "global"
        name = $bindable(), // For Global Mode (DB Name)
        source = "local",
    } = $props();

    // Ensure structure exists
    $effect(() => {
        if (!data.character_book)
            data.character_book = { entries: [], name: "" };
        if (!data.extensions) data.extensions = {};
        if (!data.character_book.entries) {
            data.character_book.entries = mode === "global" ? {} : [];
        }
    });

    let searchTerm = $state("");
    let openEntries: Record<number, boolean> = $state({});

    // Dirty State Logic for World Name
    let originalName = $state("");

    // Initialize/Reset original name on load or save
    $effect(() => {
        const _ls = lastSaved; // dependency
        untrack(() => {
            if (mode === "global") {
                originalName = name || "";
            } else {
                originalName = data.extensions?.world || "";
            }
        });
    });

    let isNameDirty = $derived.by(() => {
        let current = "";
        if (mode === "global") {
            current = name || "";
        } else {
            current = data.extensions?.world || "";
        }
        return current !== originalName;
    });

    // Filtered entries (Sorted by display_index)
    let filteredEntries = $derived.by(() => {
        let entriesRaw = data?.character_book?.entries;
        if (!entriesRaw) return [];

        let entries = [];
        if (Array.isArray(entriesRaw)) {
            entries = [...entriesRaw];
        } else {
            // Object/Map (Global)
            entries = Object.values(entriesRaw);
        }

        // Ensure 'id' exists for dndzone (Global uses 'uid')
        entries = entries.map((e: any) => {
            if (e.id === undefined && e.uid !== undefined) {
                return { ...e, id: e.uid };
            }
            return e;
        });

        // Default Sort by display_index / order / insertion_order
        entries.sort((a: any, b: any) => {
            if (mode === "global") {
                const da = a.displayIndex ?? a.order ?? 9999;
                const db = b.displayIndex ?? b.order ?? 9999;
                return da - db;
            } else {
                const da = a.extensions?.display_index ?? 9999;
                const db = b.extensions?.display_index ?? 9999;
                return da - db;
            }
        });

        if (!searchTerm) return entries;

        const low = searchTerm.toLowerCase();
        return entries.filter(
            (e: any) =>
                (e.comment || "").toLowerCase().includes(low) ||
                (e.keys || e.key || []).some((k: string) =>
                    k.toLowerCase().includes(low),
                ) ||
                (e.content || "").toLowerCase().includes(low),
        );
    });

    // Import/Export State
    let importDialogOpen = $state(false);
    let exportDialogOpen = $state(false);

    function handleImportEntries(newEntries: CharacterBookEntry[]) {
        if (!data.character_book.entries) data.character_book.entries = [];
        const currentEntries = data.character_book.entries as any[];
        
        // Calculate max values to ensure append-only
        let maxId = 0;
        let maxDisplayIndex = 0;
        
        currentEntries.forEach(e => {
            const id = Number(e.id || 0);
            const display = Number(e.extensions?.display_index || 0);
            if (id > maxId) maxId = id;
            if (display > maxDisplayIndex) maxDisplayIndex = display;
        });
        
        newEntries.forEach(entry => {
             // Clone to avoid reference issues
             const newEntry = JSON.parse(JSON.stringify(entry));
             
             // Assign new unique properties
             maxId++;
             maxDisplayIndex++;
             
             newEntry.id = maxId;
             if (!newEntry.extensions) newEntry.extensions = {};
             newEntry.extensions.display_index = maxDisplayIndex;
             
             currentEntries.push(newEntry);
        });
        
        data.character_book = { ...data.character_book }; // Trigger reactivity
        if (onChange) onChange();
    }

    // Drag and Drop Logic (svelte-dnd-action)
    // 记录拖拽前的顺序，用于判断是否真正改变了位置
    let orderBeforeDrag: (number | string)[] = [];

    function handleDndConsider(e: CustomEvent<{ items: any[], info: { trigger: string } }>) {
        // 开始拖拽时记录原始顺序并初始化拖拽状态
        if (e.detail.info.trigger === TRIGGERS.DRAG_STARTED) {
            orderBeforeDrag = filteredEntries.map((ent: any) => ent.id ?? ent.uid);
            isDragging = true;
        }
        // 只更新拖拽专用状态，不触发脏检查
        dndItems = e.detail.items;
    }

    function handleDndFinalize(e: CustomEvent<{ items: any[], info: { trigger: string } }>) {
        isDragging = false;
        const newItems = e.detail.items;
        
        // 只有顺序真正改变时才更新数据并触发 onChange
        const newOrder = newItems.map((ent: any) => ent.id ?? ent.uid);
        const orderChanged = orderBeforeDrag.length > 0 && 
            (orderBeforeDrag.length !== newOrder.length || 
             orderBeforeDrag.some((id, i) => id !== newOrder[i]));
        
        if (orderChanged) {
            // 只有顺序改变时才同步到真实数据
            updateEntriesFromDnd(newItems);
            if (onChange) onChange();
        }
        
        dndItems = [];
        orderBeforeDrag = [];
    }

    function updateEntriesFromDnd(items: any[]) {
        // 移除 svelte-dnd-action 添加的内部属性并更新 display_index
        const cleanedItems = items.map((item, index) => {
            const { isDndShadowItem, ...cleanItem } = item;
            if (mode === "global") {
                cleanItem.displayIndex = index;
            } else {
                if (!cleanItem.extensions) cleanItem.extensions = {};
                cleanItem.extensions.display_index = index;
            }
            return cleanItem;
        });

        if (Array.isArray(data.character_book.entries)) {
            data.character_book = {
                ...data.character_book,
                entries: cleanedItems,
            };
        } else {
            const entriesMap: Record<string, any> = {};
            cleanedItems.forEach((e) => {
                const key = e.uid !== undefined ? e.uid : e.id;
                entriesMap[key] = e;
            });
            data.character_book = {
                ...data.character_book,
                entries: entriesMap,
            };
        }
    }

    // 获取用于显示的列表（拖拽中用 dndItems，否则用 filteredEntries）
    let displayEntries = $derived(isDragging ? dndItems : filteredEntries);

    function addEntry(input?: any) {
        let initialData: any = {};
        // If input is not an event, treat as data
        if (input && !input.preventDefault && !input.bubbles) {
            initialData = input;
        }
        if (!data.character_book)
            data.character_book = { entries: mode === "global" ? {} : [] };
        // Ensure type correctness
        if (mode === "global" && Array.isArray(data.character_book.entries))
            data.character_book.entries = {};
        if (mode === "character" && !Array.isArray(data.character_book.entries))
            data.character_book.entries = [];

        let currentList = [];
        if (Array.isArray(data.character_book.entries)) {
            currentList = data.character_book.entries;
        } else {
            currentList = Object.values(data.character_book.entries || {});
        }

        // Generate new ID (max + 1)
        const maxId = currentList.reduce(
            (max: number, e: any) => Math.max(max, (e.uid ?? e.id) || 0),
            0,
        );
        const newId = maxId + 1;

        let newEntry: any = {};
        if (initialData.comment) {
            // If ID present in initialData, we might overwrite? No, keep generated ID
        }
        if (mode === "global") {
            // GLOBAL SCHEMA
            newEntry = {
                uid: newId,
                key: [],
                keysecondary: [],
                comment: "（请修改）条目名称",
                content: "",
                constant: false,
                vectorized: false,
                selective: true,
                selectiveLogic: 0,
                addMemo: false,
                order: 100,
                position: 0,
                disable: false,
                ignoreBudget: false,
                excludeRecursion: false,
                preventRecursion: false,
                matchPersonaDescription: false,
                matchCharacterDescription: false,
                matchCharacterPersonality: false,
                matchCharacterDepthPrompt: false,
                matchScenario: false,
                matchCreatorNotes: false,
                delayUntilRecursion: 0,
                probability: 100,
                useProbability: true,
                depth: 4,
                outletName: "",
                group: "",
                groupOverride: false,
                groupWeight: 100,
                scanDepth: null,
                caseSensitive: null,
                matchWholeWords: null,
                useGroupScoring: null,
                automationId: "",
                role: null,
                sticky: null,
                cooldown: null,
                delay: null,
                triggers: [],
                displayIndex: currentList.length,
                characterFilter: {
                    isExclude: false,
                    names: [],
                    tags: [],
                },
            };
        } else {
            // CHARACTER SCHEMA
            newEntry = {
                id: newId,
                keys: [],
                secondary_keys: [],
                comment: "（请修改）条目名称",
                content: "",
                constant: false,
                selective: true,
                insertion_order: 100,
                enabled: true,
                position: "before_char",
                use_regex: true,
                extensions: {
                    position: 0,
                    exclude_recursion: false,
                    display_index: currentList.length,
                    probability: 100,
                    useProbability: true,
                    depth: 4,
                    selectiveLogic: 0,
                    outlet_name: "",
                    group: "",
                    group_override: false,
                    group_weight: 100,
                    prevent_recursion: false,
                    delay_until_recursion: false,
                    scan_depth: null,
                    match_whole_words: null,
                    use_group_scoring: false,
                    case_sensitive: null,
                    automation_id: "",
                    role: 0,
                    vectorized: false,
                    sticky: 0,
                    cooldown: 0,
                    delay: 0,
                    match_persona_description: false,
                    match_character_description: false,
                    match_character_personality: false,
                    match_character_depth_prompt: false,
                    match_scenario: false,
                    match_creator_notes: false,
                    triggers: [],
                    ignore_budget: false,
                },
            };
        }

        if (Object.keys(initialData).length > 0) {
             newEntry = { ...newEntry, ...initialData };
        }

        if (Array.isArray(data.character_book.entries)) {
            data.character_book.entries = [
                ...data.character_book.entries,
                newEntry,
            ];
        } else {
            // Map
            data.character_book.entries = {
                ...data.character_book.entries,
                [newId]: newEntry,
            };
        }



        // Re-write back to structure (redundant but safe)
        if (Array.isArray(data.character_book.entries)) {
             // We pushed newEntry above but we modified it locally? 
             // Wait, I need to push the MERGED entry.
             // Original code:
             // if (mode === "global") { ... newEntry = ... } else { ... newEntry = ... }
             // if array -> entries = [...entries, newEntry]
             // So I should merge BEFORE pushing.
        }

        data.character_book = { ...data.character_book }; // Trigger top level
        if (onChange) onChange();
    }

    function deleteEntry(id: number) {
        if (Array.isArray(data.character_book.entries)) {
            data.character_book = {
                ...data.character_book,
                entries: data.character_book.entries.filter(
                    (e: any) => (e.id ?? e.uid) !== id,
                ),
            };
        } else {
            const newEntries = { ...data.character_book.entries };
            delete newEntries[id]; // ID matches key in Global usually
            data.character_book = {
                ...data.character_book,
                entries: newEntries,
            };
        }
        if (onChange) onChange();
    }

    function exportWorldBook() {
        const exportName =
            (mode === "global" ? name : data.extensions?.world) || "WorldInfo";
        const filename = `${exportName}.json`;

        // Ensure structure matches mode
        let content = "";
        if (mode === "global") {
            // For global, we usually export the whole object { entries: ... }
            // data.character_book IS the object
            content = JSON.stringify(data.character_book, null, 2);
        } else {
            // For character, we export { entries: ... } wrapped?
            // Normally export only makes sense for Global books in this context.
            // But if user wants to export embedded...
            content = JSON.stringify(data.character_book, null, 2);
        }

        const blob = new Blob([content], { type: "application/octet-stream" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }

    // AI Generation Logic
    let isGenDialogOpen = $state(false);
    let genInput = $state("");
    let isGenerating = $state(false);

    async function handleGenerateWorldInfo() {
        if (!genInput.trim()) {
            toast.error("请输入描述内容");
            return;
        }
        
        isGenerating = true;

        try {
            // Build Context
            let entriesData = [];
            if (Array.isArray(data.character_book?.entries)) {
                entriesData = data.character_book.entries;
            } else {
                entriesData = Object.values(data.character_book?.entries || {});
            }
            
            const enabledEntries = entriesData.filter((e: any) => e.enabled !== false && e.disable !== true);
            const currentWorldInfo = enabledEntries.map((e: any) => {
                return `Name: ${e.comment}\nContent: ${e.content}`;
            }).join("\n---\n");
            
            const newEntriesData = await AiService.generateWorldInfo(genInput, currentWorldInfo);
            
            if (newEntriesData && Array.isArray(newEntriesData)) {
                let count = 0;
                newEntriesData.forEach(item => {
                    addEntry({
                        comment: item.comment || "AI生成条目",
                        content: item.content || "",
                        constant: true
                    });
                    count++;
                });
                
                // Success
                toast.success(`已生成 ${count} 个条目`);
                isGenDialogOpen = false;
                genInput = "";
                
            } else {
                 toast.error("生成格式异常: 结果不是数组");
            }
        } catch (e: any) {
            toast.error("生成失败: " + (e.message || "Unknown error"));
        } finally {
            isGenerating = false;
        }
    }

</script>

<div
    class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 pb-10"
>
    <!-- Entries Toolbar -->
    <div class="flex items-center gap-4">
        <div class="relative flex-1 ml-1 border border-1 rounded-md">
            <Search
                class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground"
            />
            <Input
                placeholder="搜索条目..."
                bind:value={searchTerm}
                class="pl-9 bg-background/50 border-border/40"
            />
        </div>
        <div class="flex items-center gap-2">
            {#if mode === "character"}
                <DropdownMenu.Root>
                    <DropdownMenu.Trigger class="inline-flex items-center justify-center rounded-md text-sm font-medium whitespace-nowrap ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground h-9 w-9 border border-input shadow-sm">
                        <MoreHorizontal class="h-4 w-4" />
                    </DropdownMenu.Trigger>
                    <DropdownMenu.Content align="start">
                        <DropdownMenu.Item onclick={() => importDialogOpen = true}>
                            <FileDown class="mr-2 h-4 w-4" />
                            从全局世界书导入...
                        </DropdownMenu.Item>
                        <DropdownMenu.Item onclick={() => exportDialogOpen = true}>
                            <FileUp class="mr-2 h-4 w-4" />
                            导出到全局世界书...
                        </DropdownMenu.Item>
                        {#if source === "local"}
                            <DropdownMenu.Separator />
                             <DropdownMenu.Item onclick={() => isGenDialogOpen = true}>
                                <Bot class="mr-2 h-4 w-4" />
                                AI 生成条目...
                            </DropdownMenu.Item>
                        {/if}
                    </DropdownMenu.Content>
                </DropdownMenu.Root>
            {/if}
            <Button
                onclick={addEntry}
                class="gap-2 border-primary bg-background text-foreground hover:bg-primary/10"
                variant="outline"
            >
                <Plus class="h-4 w-4" /> 添加条目
            </Button>
            {#if mode === "global"}
                 <Button
                    onclick={exportWorldBook}
                    class="gap-2 border-primary/20 bg-background text-foreground hover:bg-primary/10 ml-2"
                    variant="outline"
                >
                    <Download class="h-4 w-4" /> 导出 JSON
                </Button>           
            {/if}
        </div>


    </div>
    <!-- Global Settings -->
    <div
        class="space-y-4 p-4 rounded-xl border border-border/40 bg-card/50 shadow-sm"
    >
        <div class="space-y-2">
            <DirtyLabel
                isDirty={isNameDirty}
                class="text-xs font-medium text-muted-foreground uppercase tracking-wider"
                >世界书名称</DirtyLabel
            >
            {#if mode === "global"}
                <DirtyInput
                    bind:value={name}
                    isDirty={isNameDirty}
                    placeholder="世界书名称..."
                    class="border-1 bg-secondary/20 h-10 text-lg font-medium focus-visible:ring-1 focus-visible:bg-background transition-all shadow-none"
                    oninput={() => onChange && onChange()}
                />
            {:else if data.extensions}
                <DirtyInput
                    value={data.extensions.world}
                    isDirty={isNameDirty}
                    placeholder="给世界书起个名字..."
                    class="border-1 bg-secondary/20 h-10 text-lg font-medium focus-visible:ring-1 focus-visible:bg-background transition-all shadow-none"
                    oninput={(e) => {
                        const val = e.currentTarget.value;
                        data.extensions.world = val;
                        // 确保 character_book 结构存在
                        if (!data.character_book) {
                            data.character_book = { entries: [], name: val };
                        } else {
                            data.character_book.name = val;
                        }
                        if (onChange) onChange();
                    }}
                />
            {:else}
                <div class="text-sm text-red-500">
                    Extensions struct missing
                </div>
            {/if}
        </div>
    </div>

    <!-- Entries List -->
    <div class="space-y-4 min-h-[300px] p-4">
        {#if filteredEntries.length === 0}
            <div
                class="text-center py-20 text-muted-foreground border border-dashed rounded-xl"
            >
                {#if searchTerm}
                    没找到匹配的条目
                {:else}
                    暂无世界书条目，点击右上角添加
                {/if}
            </div>
        {:else if searchTerm}
            <!-- 搜索模式：禁用拖拽 -->
            {#each filteredEntries as entry (entry.id || entry.uid)}
                <div class="transition-all duration-200 relative" class:z-20={openEntries[entry.id || entry.uid]}>
                    <WorldInfoEntry
                        {entry}
                        {lastSaved}
                        bind:isOpen={openEntries[entry.id || entry.uid]}
                        onDelete={() => deleteEntry(entry.id ?? entry.uid)}
                        {onChange}
                        onUpdate={(mutator: (e: any) => void) => {
                            // 1. Mutate the visual copy (for immediate feedback)
                            mutator(entry);

                            // 2. Mutate the source of truth
                            const targetId = entry.id ?? entry.uid;
                            if (Array.isArray(data.character_book.entries)) {
                                const realEntry = data.character_book.entries.find((e: any) => (e.id ?? e.uid) === targetId);
                                if (realEntry) mutator(realEntry);
                            } else {
                                // Map (Global)
                                if (data.character_book.entries[targetId]) {
                                    mutator(data.character_book.entries[targetId]);
                                }
                            }
                            
                            // 3. Trigger Svelte Reactivity
                            data.character_book = data.character_book;

                            if (onChange) onChange();
                        }}
                        {mode}
                    />
                </div>
            {/each}
        {:else}
            <!-- 正常模式：启用拖拽 -->
            <div
                use:dndzone={{
                    items: displayEntries,
                    flipDurationMs: FLIP_DURATION_MS,
                    delayTouchStart: TOUCH_DELAY_MS,
                    dragDisabled: isDragDisabled,
                    dropTargetStyle: {},
                    type: 'world-info-entries'
                }}
                onconsider={handleDndConsider}
                onfinalize={handleDndFinalize}
                class="space-y-4"
            >
                {#each displayEntries as entry (entry.id || entry.uid)}
                    <div
                        animate:flip={{ duration: FLIP_DURATION_MS }}
                        class={cn(
                            "transition-all duration-200 relative",
                            entry.isDndShadowItem && "h-16 rounded-xl border-2 border-dashed border-primary/50 bg-primary/5",
                            !entry.isDndShadowItem && openEntries[entry.id || entry.uid] ? "z-20" : "z-0 hover:!z-50"
                        )}
                    >
                        {#if !entry.isDndShadowItem}
                            <WorldInfoEntry
                                {entry}
                                {lastSaved}
                                bind:isOpen={openEntries[entry.id || entry.uid]}
                                onDelete={() => deleteEntry(entry.id ?? entry.uid)}
                                {onChange}
                                onUpdate={(mutator: (e: any) => void) => {
                                    mutator(entry);
                                    
                                    const targetId = entry.id ?? entry.uid;
                                    if (Array.isArray(data.character_book.entries)) {
                                        const realEntry = data.character_book.entries.find((e: any) => (e.id ?? e.uid) === targetId);
                                        if (realEntry) mutator(realEntry);
                                    } else {
                                        if (data.character_book.entries[targetId]) {
                                            mutator(data.character_book.entries[targetId]);
                                        }
                                    }

                                    data.character_book = data.character_book;
                                    if (onChange) onChange();
                                }}
                                {mode}
                            />
                        {/if}
                    </div>
                {/each}
            </div>
        {/if}
    </div>
</div>

    <ImportWorldInfoDialog 
        bind:open={importDialogOpen} 
        onImport={handleImportEntries} 
    />
    
    <ExportWorldInfoDialog 
        bind:open={exportDialogOpen}
        entries={Array.isArray(data.character_book.entries) ? data.character_book.entries : Object.values(data.character_book.entries)}
    />

    <Dialog.Root bind:open={isGenDialogOpen}>
        <Dialog.Content class="sm:max-w-[500px]">
            <Dialog.Header>
                <Dialog.Title class="flex items-center gap-2">
                    <Bot class="h-5 w-5 text-primary" />
                    AI 世界书生成
                </Dialog.Title>
                <Dialog.Description>
                    输入设定灵感，AI 将基于当前世界观自动扩展相关条目。
                </Dialog.Description>
                <div class="text-xs text-orange-600 dark:text-orange-400 mt-1">
                    条目确认使用后，请记得点保存，否则在下次生成时，之前生成的内容不会作为上下文。
                </div>
            </Dialog.Header>
            
            <div class="py-4">
                <Label class="mb-2 block">设定描述</Label>
                <Textarea 
                    bind:value={genInput} 
                    placeholder="请清晰描述你想要的世界书方向和简要内容，并且建议进行限制（免得AI放飞自我）"
                    rows={4}
                />
            </div>

            <Dialog.Footer>
                <Button variant="outline" onclick={() => isGenDialogOpen = false}>取消</Button>
                <Button onclick={handleGenerateWorldInfo} disabled={isGenerating}>
                    {#if isGenerating}
                        <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                        生成中，别关...
                    {:else}
                        <Sparkles class="mr-2 h-4 w-4" />
                        开始生成
                    {/if}
                </Button>
            </Dialog.Footer>
        </Dialog.Content>
    </Dialog.Root>


