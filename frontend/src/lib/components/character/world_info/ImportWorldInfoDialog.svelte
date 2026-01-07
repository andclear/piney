<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as Select from "$lib/components/ui/select";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { Loader2 } from "lucide-svelte";
    import { onMount } from "svelte";
    import { convertGlobalToCharacterBook, type CharacterBookEntry, type GlobalWorldInfo } from "$lib/worldInfoConverter";

    let { open = $bindable(false), onImport } = $props<{
        open: boolean;
        onImport: (entries: CharacterBookEntry[]) => void;
    }>();

    let loading = $state(false);
    let worldInfos: { id: string; name: string }[] = $state([]);
    let selectedWorldId: string | undefined = $state();
    let selectedWorldData: GlobalWorldInfo | null = $state(null);
    let selectedEntryIds: string[] = $state([]);

    // Load World Info List
    async function loadWorldInfos() {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch("/api/world_info", {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                worldInfos = await res.json();
            }
        } catch (e) {
            console.error("Failed to load world infos", e);
        } finally {
            loading = false;
        }
    }

    // Load Specific World Info Data
    async function loadWorldData(id: string) {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`/api/world_info/${id}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                const data = await res.json();
                // Parse 'data' JSON string if it's a string, otherwise use as is
                let info = data;
                if (typeof data.data === 'string') {
                    try {
                        const parsedData = JSON.parse(data.data);
                        // Merge parsed data into root for converter compatibility if needed, 
                        // or just pass parsedData if it matches GlobalWorldInfo structure.
                        // Based on api/world_info.rs, 'data' field stores the JSON content.
                        // But the converter expects GlobalWorldInfo object (entries, name).
                        
                        // Let's assume parsedData IS the GlobalWorldInfo object structure (entries, etc.)
                        // But we also have top-level name.
                        info = { ...parsedData, name: data.name };
                    } catch (e) {
                        console.error("Failed to parse world info data JSON", e);
                    }
                }
                selectedWorldData = info;
                selectedEntryIds = []; // Reset selection
            }
        } catch (e) {
            console.error("Failed to load world data", e);
        } finally {
            loading = false;
        }
    }

    $effect(() => {
        if (open) {
            loadWorldInfos();
            selectedWorldId = undefined;
            selectedWorldData = null;
            selectedEntryIds = [];
        }
    });

    $effect(() => {
        if (selectedWorldId) {
            loadWorldData(selectedWorldId);
        }
    });

    function handleImport() {
        if (!selectedWorldData) return;
        
        // Convert the whole book first (easier to reuse logic)
        const charBook = convertGlobalToCharacterBook(selectedWorldData);
        
        // Filter by selected IDs
        // Note: GlobalWorldInfo entries are map, so ID is the key. 
        // Our converter preserves global UID as 'id'.
        // We matched checkbox values to entry keys/uids.
        
        // However, conversion loses the original map key if it wasn't in the object?
        // convertGlobalToCharacterBook converts map values to array. 
        // Let's check IDs.
        
        const importedEntries = charBook.entries.filter(entry => 
            selectedEntryIds.includes(String(entry.id))
        );

        onImport(importedEntries);
        open = false;
    }

    function toggleEntry(uid: string | number) {
        const idStr = String(uid);
        if (selectedEntryIds.includes(idStr)) {
            selectedEntryIds = selectedEntryIds.filter(id => id !== idStr);
        } else {
            selectedEntryIds = [...selectedEntryIds, idStr];
        }
    }

    function selectAll() {
        if (!selectedWorldData?.entries) return;
        const entries = Array.isArray(selectedWorldData.entries) 
            ? selectedWorldData.entries 
            : Object.values(selectedWorldData.entries);
            
        selectedEntryIds = entries.map(e => String(e.uid));
    }
    
    function deselectAll() {
        selectedEntryIds = [];
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-[600px] flex flex-col max-h-[85vh]">
        <Dialog.Header>
            <Dialog.Title>从全局世界书导入</Dialog.Title>
            <Dialog.Description>
                选择一个全局世界书，并勾选需要导入到当前角色的条目。
            </Dialog.Description>
        </Dialog.Header>

        <div class="space-y-4 py-4 flex-1 overflow-hidden flex flex-col">
            <!-- World Info Selection -->
            <div class="flex flex-col space-y-2">
                <span class="text-sm font-medium">选择世界书</span>
                <Select.Root type="single" bind:value={selectedWorldId}>
                    <Select.Trigger>
                        {worldInfos.find(w => w.id === selectedWorldId)?.name ?? "请选择..."}
                    </Select.Trigger>
                    <Select.Content>
                        {#each worldInfos as info}
                            <Select.Item value={info.id}>{info.name}</Select.Item>
                        {/each}
                    </Select.Content>
                </Select.Root>
            </div>

            <!-- Entries List -->
            {#if selectedWorldData}
                 <div class="flex-1 border rounded-md p-2 overflow-hidden flex flex-col">
                    <div class="flex justify-between items-center mb-2 px-2">
                         <span class="text-xs text-muted-foreground">
                             共 {Object.keys(selectedWorldData.entries || {}).length} 个条目，已选 {selectedEntryIds.length} 个
                         </span>
                         <div class="space-x-2">
                             <Button variant="ghost" size="sm" onclick={selectAll} class="h-6 text-xs">全选</Button>
                             <Button variant="ghost" size="sm" onclick={deselectAll} class="h-6 text-xs">清空</Button>
                         </div>
                    </div>
                    
                    <ScrollArea class="flex-1 h-[300px]">
                        <div class="space-y-1 p-1">
                            {#each Object.values(selectedWorldData.entries || {}) as entry (entry.uid)}
                                <div class="flex items-start space-x-3 p-2 rounded hover:bg-muted/50 transition-colors">
                                    <Checkbox 
                                        id="entry-{entry.uid}"
                                        checked={selectedEntryIds.includes(String(entry.uid))}
                                        onCheckedChange={() => toggleEntry(entry.uid)}
                                    />
                                    <div class="grid gap-1.5 leading-none">
                                        <label
                                            for="entry-{entry.uid}"
                                            class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 cursor-pointer"
                                        >
                                            {entry.comment || "未命名条目"}
                                            <span class="text-xs text-muted-foreground ml-2 font-normal">
                                                Keys: {(entry.key || []).join(", ")}
                                            </span>
                                        </label>
                                        {#if entry.content}
                                            <p class="text-xs text-muted-foreground line-clamp-2">
                                                {entry.content}
                                            </p>
                                        {/if}
                                    </div>
                                </div>
                            {/each}
                        </div>
                    </ScrollArea>
                 </div>
            {:else if loading && selectedWorldId}
                <div class="flex items-center justify-center p-8 text-muted-foreground">
                    <Loader2 class="h-6 w-6 animate-spin mr-2" />
                    加载中...
                </div>
            {/if}
        </div>

        <Dialog.Footer>
             <Button variant="outline" onclick={() => open = false}>取消</Button>
             <Button onclick={handleImport} disabled={selectedEntryIds.length === 0}>
                导入选中的条目 ({selectedEntryIds.length})
             </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
