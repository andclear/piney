<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as Select from "$lib/components/ui/select";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { Tabs, TabsContent, TabsList, TabsTrigger } from "$lib/components/ui/tabs";
    import { Loader2 } from "lucide-svelte";
    import { convertCharacterBookToGlobal, type CharacterBookEntry, type GlobalWorldInfo } from "$lib/worldInfoConverter";

    let { open = $bindable(false), entries } = $props<{
        open: boolean;
        entries: CharacterBookEntry[];
    }>();

    let loading = $state(false);
    let worldInfos: { id: string; name: string }[] = $state([]);
    let selectedEntryIds: string[] = $state([]);
    
    // Form States
    let exportMode = $state<"new" | "existing">("new");
    let newName = $state("");
    let selectedExistingId = $state<string | undefined>();

    // Initial load
    async function loadWorldInfos() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch("/api/world_info", {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                worldInfos = await res.json();
            }
        } catch (e) {
            console.error(e);
        }
    }

    $effect(() => {
        if (open) {
            loadWorldInfos();
            selectedEntryIds = []; // Default clean, let user pick
            newName = "";
            selectedExistingId = undefined;
        }
    });

    function toggleEntry(uid: string | number) {
        const idStr = String(uid);
        if (selectedEntryIds.includes(idStr)) {
            selectedEntryIds = selectedEntryIds.filter(id => id !== idStr);
        } else {
            selectedEntryIds = [...selectedEntryIds, idStr];
        }
    }

    function selectAll() {
        selectedEntryIds = entries.map(e => String(e.id));
    }
    
    function deselectAll() {
        selectedEntryIds = [];
    }

    async function handleExport() {
        if (selectedEntryIds.length === 0) return;
        loading = true;

        try {
            // 1. Filter entries
            const entriesToExport = entries.filter(e => selectedEntryIds.includes(String(e.id)));
            // Create a temp partial book structure for conversion
            const partialBook = { name: "temp", entries: entriesToExport };
            const globalFormat = convertCharacterBookToGlobal(partialBook);
            
            // 2. Export logic
            if (exportMode === "new") {
                if (!newName) return;
                
                // Construct file
                globalFormat.name = newName;
                const blob = new Blob([JSON.stringify(globalFormat, null, 2)], { type: "application/json" });
                const formData = new FormData();
                formData.append("file", blob, `${newName}.json`);
                
                // Upload
                const token = localStorage.getItem("auth_token");
                const res = await fetch("/api/world_info/import", {
                    method: "POST",
                    headers: token ? { Authorization: `Bearer ${token}` } : {},
                    body: formData
                });
                
                if (!res.ok) throw new Error("Export failed");
                
            } else if (exportMode === "existing") {
                if (!selectedExistingId) return;

                const token = localStorage.getItem("auth_token");
                const headers = token ? { Authorization: `Bearer ${token}` } : {};

                // Update existing: Fetch first to merge
                const getRes = await fetch(`/api/world_info/${selectedExistingId}`, { headers });
                if (!getRes.ok) throw new Error("Failed to fetch existing world info");
                const existingData = await getRes.json();
                
                let existingJson: GlobalWorldInfo;
                if (typeof existingData.data === 'string') {
                    existingJson = JSON.parse(existingData.data);
                } else {
                    existingJson = existingData.data || { entries: {} };
                }
                
                // Ensure entries map exists
                if (!existingJson.entries || Array.isArray(existingJson.entries)) {
                     // Normalize to map if it was array (unlikely for global format source but robust)
                     const oldEntriesArray = Array.isArray(existingJson.entries) ? existingJson.entries : [];
                     existingJson.entries = {};
                     oldEntriesArray.forEach((e: any) => {
                         if (e.uid !== undefined) existingJson.entries[e.uid] = e;
                     });
                }

                // Calculate Max Values form existing data to ensure non-collision append
                let maxUid = 0;
                let maxDisplay = 0;
                
                const existingEntryValues: any[] = Object.values(existingJson.entries);
                existingEntryValues.forEach(e => {
                    const uid = Number(e.uid || 0);
                    const display = Number(e.displayIndex || 0);
                    if (uid > maxUid) maxUid = uid;
                    if (display > maxDisplay) maxDisplay = display;
                });

                // Merge: Append as NEW entries
                const newEntries = globalFormat.entries;
                for (const key in newEntries) {
                     const entry = newEntries[key];
                     
                     // Assign new unique properties
                     maxUid++;
                     
                     // For displayIndex, usually it's continuous.
                     maxDisplay++;

                     entry.uid = maxUid;
                     // entry.order is PRESERVED from the character book (insertion_order)
                     entry.displayIndex = maxDisplay;
                     
                     // Add to map with NEW UID key
                     existingJson.entries[String(maxUid)] = entry;
                }

                // Push Update
                const updateRes = await fetch(`/api/world_info/${selectedExistingId}`, {
                    method: "PATCH",
                    headers: { 
                        "Content-Type": "application/json",
                        ...headers
                    },
                    body: JSON.stringify({
                        data: existingJson
                    })
                });
                
                if (!updateRes.ok) throw new Error("Update failed");
            }
            
            open = false;

        } catch (e) {
            console.error("Export error", e);
            alert("导出失败，请检查控制台日志。");
        } finally {
            loading = false;
        }
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-[600px] flex flex-col max-h-[85vh]">
        <Dialog.Header>
            <Dialog.Title>导出到全局世界书</Dialog.Title>
            <Dialog.Description>
                将当前选中的角色卡内联条目导出为全局世界书，或合并到现有的世界书中。
            </Dialog.Description>
        </Dialog.Header>

        <div class="flex-1 overflow-hidden flex flex-col space-y-4 py-4">
             <!-- Entry Selection -->
             <div class="flex-1 border rounded-md p-2 overflow-hidden flex flex-col min-h-[200px]">
                <div class="flex justify-between items-center mb-2 px-2">
                     <span class="text-xs text-muted-foreground">
                         可选 {entries.length} 个条目，已选 {selectedEntryIds.length} 个
                     </span>
                     <div class="space-x-2">
                         <Button variant="ghost" size="sm" onclick={selectAll} class="h-6 text-xs">全选</Button>
                         <Button variant="ghost" size="sm" onclick={deselectAll} class="h-6 text-xs">清空</Button>
                     </div>
                </div>
                
                <ScrollArea class="flex-1 h-[200px]">
                    <div class="space-y-1 p-1">
                        {#each entries as entry (entry.id)}
                            <div class="flex items-start space-x-3 p-2 rounded hover:bg-muted/50 transition-colors">
                                <Checkbox 
                                    id="exp-entry-{entry.id}"
                                    checked={selectedEntryIds.includes(String(entry.id))}
                                    onCheckedChange={() => toggleEntry(entry.id)}
                                />
                                <div class="grid gap-1.5 leading-none">
                                    <label
                                        for="exp-entry-{entry.id}"
                                        class="text-sm font-medium leading-none cursor-pointer"
                                    >
                                        {entry.comment || `条目 ${entry.id}`}
                                        <span class="text-xs text-muted-foreground ml-2 font-normal">
                                            Keys: {(entry.keys || []).join(", ")}
                                        </span>
                                    </label>
                                    {#if entry.content}
                                        <p class="text-xs text-muted-foreground line-clamp-1">
                                            {entry.content}
                                        </p>
                                    {/if}
                                </div>
                            </div>
                        {/each}
                    </div>
                </ScrollArea>
             </div>

             <!-- Export Options -->
             <Tabs bind:value={exportMode}>
                 <TabsList class="grid w-full grid-cols-2">
                     <TabsTrigger value="new">新建世界书</TabsTrigger>
                     <TabsTrigger value="existing">合并到现有</TabsTrigger>
                 </TabsList>
                 
                 <TabsContent value="new" class="space-y-2 pt-2">
                     <Label for="new-name">世界书名称</Label>
                     <Input id="new-name" bind:value={newName} placeholder="例如：通用设定集..." />
                 </TabsContent>
                 
                 <TabsContent value="existing" class="space-y-2 pt-2">
                      <Label>选择目标世界书</Label>
                      <Select.Root type="single" bind:value={selectedExistingId}>
                        <Select.Trigger>
                            {worldInfos.find(w => w.id === selectedExistingId)?.name ?? "请选择..."}
                        </Select.Trigger>
                        <Select.Content>
                            {#each worldInfos as info}
                                <Select.Item value={info.id}>{info.name}</Select.Item>
                            {/each}
                        </Select.Content>
                    </Select.Root>
                    <p class="text-xs text-muted-foreground">
                        注意：所有导出条目将作为新条目追加到目标世界书中（自动分配新 ID），不会覆盖现有条目。
                    </p>
                 </TabsContent>
             </Tabs>
        </div>

        <Dialog.Footer>
             <Button variant="outline" onclick={() => open = false}>取消</Button>
             <Button 
                onclick={handleExport} 
                disabled={loading || selectedEntryIds.length === 0 || (exportMode === "new" && !newName) || (exportMode === "existing" && !selectedExistingId)}
             >
                {#if loading}
                    <Loader2 class="h-4 w-4 animate-spin mr-2" />
                {/if}
                导出
             </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
