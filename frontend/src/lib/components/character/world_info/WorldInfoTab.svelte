<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import DirtyInput from "$lib/components/common/DirtyInput.svelte";
    import { Button } from "$lib/components/ui/button";
    import { Label } from "$lib/components/ui/label";
    import { Plus, Globe, Search, ArrowUpDown, Download, ChevronDown, FileDown, FileUp, MoreHorizontal } from "lucide-svelte";
    import WorldInfoEntry from "./WorldInfoEntry.svelte";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { cn } from "$lib/utils";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
    import ImportWorldInfoDialog from "./ImportWorldInfoDialog.svelte";
    import ExportWorldInfoDialog from "./ExportWorldInfoDialog.svelte";
    import { type CharacterBookEntry } from "$lib/worldInfoConverter";
    import { untrack } from "svelte";

    import DirtyLabel from "$lib/components/common/DirtyLabel.svelte";

    let {
        data = $bindable({ character_book: { entries: [] }, extensions: {} }),
        lastSaved = 0,
        onChange = () => {},
        mode = "character", // "character" | "global"
        name = $bindable(), // For Global Mode (DB Name)
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

    // Drag & Drop State
    let draggedItemIdx: number | null = $state(null);

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
    let dragOverItemIdx: number | null = $state(null);

    function handleDragStart(e: DragEvent, idx: number) {
        if (searchTerm) {
            e.preventDefault();
            return;
        }
        draggedItemIdx = idx;
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = "move";
            e.dataTransfer.dropEffect = "move";
            // Optional: set drag image
        }
    }

    function handleDragOver(e: DragEvent, idx: number) {
        e.preventDefault(); // allow drop
        if (searchTerm) return;
        dragOverItemIdx = idx;
    }

    function handleDrop(e: DragEvent, dropIdx: number) {
        e.preventDefault();
        if (searchTerm || draggedItemIdx === null) return;

        const currentEntries = [...filteredEntries]; // Sorted list
        const [draggedItem] = currentEntries.splice(draggedItemIdx, 1);
        currentEntries.splice(dropIdx, 0, draggedItem);

        // Update display_index for ALL items and update main data
        const updatedEntries = currentEntries.map((item, index) => {
            if (mode === "global") {
                item.displayIndex = index;
                // item.order = index; // Optional: sync order too?
            } else {
                if (!item.extensions) item.extensions = {};
                item.extensions.display_index = index;
            }
            return item;
        });

        // Write back to data
        if (Array.isArray(data.character_book.entries)) {
            data.character_book = {
                ...data.character_book,
                entries: updatedEntries,
            };
        } else {
            // Convert Array back to Object (Map)
            const entriesMap: Record<string, any> = {};
            updatedEntries.forEach((e) => {
                const key = e.uid !== undefined ? e.uid : e.id;
                entriesMap[key] = e;
            });
            data.character_book = {
                ...data.character_book,
                entries: entriesMap,
            };
        }

        if (onChange) onChange();

        draggedItemIdx = null;
        dragOverItemIdx = null;
    }

    function handleDragEnd() {
        draggedItemIdx = null;
        dragOverItemIdx = null;
    }

    function addEntry() {
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

        data.character_book = { ...data.character_book }; // Trigger top level
        if (onChange) onChange();
    }

    function deleteEntry(id: number) {
        if (Array.isArray(data.character_book.entries)) {
            data.character_book = {
                ...data.character_book,
                entries: data.character_book.entries.filter(
                    (e: any) => e.id !== id,
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
                        if (data.character_book) data.character_book.name = val;
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
    <div class="space-y-4 min-h-[300px]">
        {#if filteredEntries.length > 0}
            {#each filteredEntries as entry, index (entry.id || entry.uid)}
                <div
                    role="group"
                    draggable={!searchTerm &&
                        !openEntries[entry.id || entry.uid]}
                    ondragstart={(e) => handleDragStart(e, index)}
                    ondragover={(e) => handleDragOver(e, index)}
                    ondrop={(e) => handleDrop(e, index)}
                    ondragend={handleDragEnd}
                    class={cn(
                        "transition-all duration-200",
                        draggedItemIdx === index && "opacity-50 scale-95",
                        dragOverItemIdx === index &&
                            "border-t-2 border-primary pt-2",
                    )}
                >
                    <WorldInfoEntry
                        {entry}
                        {lastSaved}
                        bind:isOpen={openEntries[entry.id || entry.uid]}
                        onDelete={() => deleteEntry(entry.id || entry.uid)}
                        {onChange}
                        onUpdate={(mutator) => {
                            mutator(entry);
                            if (onChange) onChange();
                        }}
                        {mode}
                    />
                </div>
            {/each}
        {:else}
            <div
                class="text-center py-20 text-muted-foreground border border-dashed rounded-xl"
            >
                {#if searchTerm}
                    没找到匹配的条目
                {:else}
                    暂无世界书条目，点击右上角添加
                {/if}
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
