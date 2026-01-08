<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import { Button } from "$lib/components/ui/button";
    import { Label } from "$lib/components/ui/label";
    import { Plus, Search, Regex, ArrowUpDown } from "lucide-svelte";
    import RegexItem from "./RegexItem.svelte";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { cn } from "$lib/utils";
    import { untrack } from "svelte";
    import { toast } from "svelte-sonner";
    import { beforeNavigate, goto } from "$app/navigation";
    import * as Dialog from "$lib/components/ui/dialog";
    import { AlertTriangle } from "lucide-svelte";

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
    // Ensure regex_scripts array exists
    $effect(() => {
        if (!data.extensions) data.extensions = {};
        if (!data.extensions.regex_scripts) data.extensions.regex_scripts = [];
        // Init snapshot on mount
        if (originalScripts === null && data.extensions.regex_scripts.length >= 0) {
             updateSnapshot();
        }
    });

    let searchTerm = $state("");
    let draggedIndex: number | null = $state(null);
    let dragOverItemIdx: number | null = $state(null);
    let openScripts: Record<string, boolean> = $state({});

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
            if (!script.id) return; // New/Temp
            const orig = original.find((s: any) => s.id === script.id);
            if (!orig) {
                dirtySet.add(script.id); // New script
            } else {
                if (JSON.stringify(script) !== JSON.stringify(orig)) {
                    dirtySet.add(script.id);
                }
            }
        });

        // Deleted scripts (tracked by existence diff, but here we just need ID set for UI)
        // Global dirty handles deletions.
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
        // Revert changes? No, just navigate away.
        // If we stay, re-snap? No.
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
            scriptName: "新脚本",
            findRegex: "",
            replaceString: "",
            trimStrings: [],
            placement: [2], // AI Output by default
            disabled: false,
            markdownOnly: true, // Default per specs
            promptOnly: false,
            runOnEdit: true, // Default
            substituteRegex: 0,
            minDepth: null,
            maxDepth: null
        };
        
        if (!data.extensions.regex_scripts) data.extensions.regex_scripts = [];
        // Use assignment to trigger reactivity (Svelte 5 legacy/hybrid mode safety)
        data.extensions.regex_scripts = [...data.extensions.regex_scripts, newScript];
        onChange();
        toast.success("已添加新脚本");
    }

    function deleteScript(scriptId: string) {
        // No confirmation dialog as requested.
        // Deletion marks state as dirty (unsaved), so user can revert by not saving.
        data.extensions.regex_scripts = data.extensions.regex_scripts.filter((s: any) => s.id !== scriptId);
        onChange();
        toast.success("脚本已删除");
    }

    // Drag and Drop Logic
    function handleDragStart(e: DragEvent, index: number) {
        draggedIndex = index;
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = 'move';
            e.dataTransfer.dropEffect = 'move';
            // Optional: Set drag image
        }
    }

    function handleDragOver(e: DragEvent, index: number) {
        e.preventDefault(); // Necessary to allow dropping
        e.dataTransfer!.dropEffect = 'move';
        
        if (draggedIndex === null || draggedIndex === index) return;
        
        dragOverItemIdx = index;
    }

    function handleDragEnd() {
        draggedIndex = null;
        dragOverItemIdx = null;
    }

    function handleDrop(e: DragEvent, targetIndex: number) {
        e.preventDefault();
        dragOverItemIdx = null;
        if (draggedIndex === null) return;
        if (draggedIndex === targetIndex) {
            draggedIndex = null;
            return;
        }

        // Reorder
        const list = [...data.extensions.regex_scripts];
        const [removed] = list.splice(draggedIndex, 1);
        list.splice(targetIndex, 0, removed);
        
        data.extensions.regex_scripts = list;
        draggedIndex = null;
        onChange();
    }
</script>

<div class="h-full flex flex-col space-y-4">
    <!-- Toolbar -->
    <div class="flex items-center justify-between gap-4 px-1">
        <div class="relative flex-1 max-w-sm">
            <Search class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
                type="search"
                placeholder="搜索脚本名称..."
                class="pl-9 h-9"
                bind:value={searchTerm}
            />
        </div>
        <div class="flex items-center gap-2">
            <Button size="sm" class="h-9 gap-1 border-primary !hover:border-primary/10" onclick={addScript} variant="outline">
                <Plus class="h-4 w-4" />
                <span class="hidden sm:inline">添加脚本</span>
            </Button>
        </div>
    </div>

    <!-- List -->
    <ScrollArea class="flex-1 -mx-2 px-2">
        <div class="space-y-3 pb-8">
            {#if filteredScripts.length === 0}
                <div class="flex flex-col items-center justify-center py-12 text-muted-foreground text-sm border border-dashed rounded-lg bg-muted/30">
                    <Regex class="h-8 w-8 mb-2 opacity-50" />
                    {#if searchTerm}
                        <p>未找到匹配的脚本</p>
                    {:else}
                        <p>暂无正则脚本</p>
                        <Button variant="link" onclick={addScript} class="h-auto p-0 text-primary">立即创建</Button>
                    {/if}
</div>
            {:else}
                {#each filteredScripts as script, index (script.id)}
                    <!-- 
                        Note: Index here is index in filtered list. 
                        Ideally we want index in original list for handling drag target accurately 
                        if we support drag on filtered list (usually disabled or tricky).
                        For now, assuming search clears drag.
                    -->
                    {@const realIndex = scripts.findIndex((s: any) => s.id === script.id)}
                    <div
                        role="listitem"
                        draggable={!searchTerm && !openScripts[script.id]} 
                        ondragstart={(e) => handleDragStart(e, realIndex)}
                        ondragover={(e) => handleDragOver(e, realIndex)}
                        ondrop={(e) => handleDrop(e, realIndex)}
                        ondragend={handleDragEnd}
                        class={cn(
                            "transition-all duration-200", 
                            draggedIndex === realIndex && "opacity-50 scale-95",
                            dragOverItemIdx === realIndex && "border-t-2 border-primary pt-2"
                        )}
                    >
                        <RegexItem 
                            bind:script={data.extensions.regex_scripts[realIndex]} 
                            bind:isOpen={openScripts[script.id]}
                            isDirty={dirtyScriptIds.has(script.id)}
                            {lastSaved}
                            onChange={onChange}
                            onDelete={() => deleteScript(script.id)}
                        />
                    </div>
                {/each}
            {/if}
        </div>
    </ScrollArea>
    
    <!-- Footer Hint -->
    <div class="text-[10px] text-muted-foreground text-center border-t pt-2">
        {#if !searchTerm && scripts.length > 1}
            <span class="flex items-center justify-center gap-1">
                <ArrowUpDown class="h-3 w-3" /> 拖拽可调整脚本执行顺序
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
                您有未保存的正则脚本更改。离开页面将丢失这些更改。确定要离开吗？
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
