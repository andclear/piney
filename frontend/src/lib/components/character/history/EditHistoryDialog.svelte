<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import { Switch } from "$lib/components/ui/switch";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { Loader2, FileJson, AlertCircle, RotateCcw, Regex, Plus, Import, GripVertical, Trash2, Download } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";
    import { convertJsonlToTxt, scanTags } from "$lib/utils/exportUtils";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { dndzone } from "svelte-dnd-action";
    import { flip } from "svelte/animate";
    import { untrack } from "svelte";

    let { open = $bindable(false), history, cardId, onUpdate } = $props();

    // svelte-ignore state_referenced_locally
    let name = $state(history.display_name.replace(/\.(txt|jsonl)$/i, ''));
    let isProcessing = $state(false);
    let step: 'main' | 'tags' | 'regex' = $state('main');

    // Tag Reselection State
    let rawSource = "";
    let availableTags: string[] = $state([]);
    let selectedTags: string[] = $state([]);

    // Regex State
    let regexScripts: any[] = $state([]);
    let characterName = ""; 
    
    // Delete Confirmation State
    let showDeleteDialog = $state(false);
    let pendingDeleteId: string | null = $state(null);
    let dontAskDelete = $state(false); 
    
    // Init from local storage
    $effect(() => {
        if (typeof localStorage !== 'undefined') {
            dontAskDelete = localStorage.getItem("regex_delete_dont_ask") === "true";
        }
    }); 

    $effect(() => {
        if (open && history.id) {
            untrack(() => {
                name = history.display_name.replace(/\.(txt|jsonl)$/i, '');
                step = 'main';
                try {
                    regexScripts = JSON.parse(history.regex_scripts || "[]");
                } catch {
                    regexScripts = [];
                }
                fetchCharacterInfo();
            });
        }
    });
    
    async function fetchCharacterInfo() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                const data = await res.json();
                characterName = data.name || "character";
            }
        } catch {}
    }

    async function fetchSource() {
        isProcessing = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}/history/${history.id}/content?source=true`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (!res.ok) throw new Error("Failed to fetch source file");
            
            const text = await res.text();
            rawSource = text;
            availableTags = scanTags(text);
            selectedTags = [...availableTags];
            step = 'tags';
        } catch (e) {
            toast.error("无法获取源文件，可能源文件不存在");
        } finally {
            isProcessing = false;
        }
    }

    function toggleTag(tag: string) {
        if (selectedTags.includes(tag)) {
            selectedTags = selectedTags.filter(t => t !== tag);
        } else {
            selectedTags = [...selectedTags, tag];
        }
    }

    function toggleAllTags() {
        if (selectedTags.length === availableTags.length) {
            selectedTags = [];
        } else {
            selectedTags = [...availableTags];
        }
    }

    // Regex Utils
     function generateUUID() {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
            var r = Math.random() * 16 | 0, v = c == 'x' ? r : (r & 0x3 | 0x8);
            return v.toString(16);
        });
    }

    async function updateRegexToBackend(scripts: any[]) {
        const token = localStorage.getItem("auth_token");
        const payload = { regex_scripts: JSON.stringify(scripts) };
        const res = await fetch(`${API_BASE}/api/cards/${cardId}/history/${history.id}`, {
            method: 'PATCH',
            headers: {
                'Content-Type': 'application/json',
                ...(token ? { Authorization: `Bearer ${token}` } : {}),
            },
            body: JSON.stringify(payload)
        });
        if (!res.ok) throw new Error("Failed to auto-save regex");
        
        history.regex_scripts = JSON.stringify(scripts);
    }

    // Dnd Handlers
    function handleSortConsider(e: CustomEvent<DndEvent<any>>) {
        regexScripts = e.detail.items;
    }

    function handleSortFinalize(e: CustomEvent<DndEvent<any>>) {
        regexScripts = e.detail.items;
        updateRegexToBackend(regexScripts)
            .catch(() => toast.error("顺序保存失败"));
    }

    async function handleImportRegex(e: Event) {
        const input = e.target as HTMLInputElement;
        if (!input.files || input.files.length === 0) return;
        
        // Limit to 30 files
        const files = Array.from(input.files).slice(0, 30);
        let newScripts: any[] = [];
        let errorCount = 0;
        let duplicateCount = 0;
        
        // Create lookup set for existing scripts to speed up duplicate check
        // Key: id (if present) OR content_hash
        const existingIds = new Set(regexScripts.map(s => s.id).filter(Boolean));
        const existingContentHashes = new Set(regexScripts.map(s => 
            `${s.scriptName}|${s.findRegex}|${s.replaceString}|${s.placement?.join(',')}`
        ));

        for (const file of files) {
            try {
                const text = await file.text();
                let data = JSON.parse(text);
                let fileScripts: any[] = [];
                
                if (Array.isArray(data)) {
                    fileScripts = data;
                } else if (data.extensions && Array.isArray(data.extensions.regex_scripts)) {
                    fileScripts = data.extensions.regex_scripts;
                } else if (data.data?.extensions?.regex_scripts && Array.isArray(data.data.extensions.regex_scripts)) {
                     fileScripts = data.data.extensions.regex_scripts;
                } else if (data.scriptName && data.findRegex) {
                     fileScripts = [data];
                } else {
                     errorCount++;
                     continue;
                }
                
                // Parse scripts in this file
                for (const s of fileScripts) {
                    if (s.scriptName && s.findRegex) {
                        if (s.id && existingIds.has(s.id)) {
                            duplicateCount++;
                            continue;
                        }
                        
                        // Check content hash
                        const hash = `${s.scriptName}|${s.findRegex}|${s.replaceString}|${s.placement?.join(',')}`;
                        if (existingContentHashes.has(hash)) {
                            duplicateCount++;
                            continue;
                        }

                        // Treat imports as new local copies with unique IDs to avoid potential collisions
                        const newScript = {
                            ...s,
                            id: generateUUID()
                        };
                        newScripts.push(newScript);
                        
                        // Add to temp lookups to prevent duplicates WITHIN the import batch
                        if (s.id) existingIds.add(s.id);
                        existingContentHashes.add(hash);
                    } 
                }
            } catch (e) {
                console.error("Parse error", file.name, e);
                errorCount++;
            }
        }
        
        if (newScripts.length > 0) {
            const combined = [...regexScripts, ...newScripts];
            
            // Sort by scriptName (numeric + Chinese Pinyin support)
            combined.sort((a, b) => {
                const nameA = a.scriptName || "";
                const nameB = b.scriptName || "";
                return nameA.localeCompare(nameB, "zh-CN", { numeric: true });
            });
            
            regexScripts = combined;
            
            await updateRegexToBackend(combined);
            let msg = `成功导入 ${newScripts.length} 条规则`;
            if (duplicateCount > 0) msg += ` (跳过 ${duplicateCount} 条重复)`;
            if (errorCount > 0) msg += ` (跳过 ${errorCount} 个无效文件)`;
            toast.success(msg);
        } else {
            if (duplicateCount > 0 && errorCount === 0) {
                toast.info(`所有规则 (${duplicateCount}条) 均为重复，已跳过`);
            } else if (errorCount > 0) {
                toast.error("未找到有效的正则配置");
            }
        }
        
        input.value = "";
    }

    function deleteScript(id: string) {
        if (dontAskDelete) {
            performDelete(id);
        } else {
            pendingDeleteId = id;
            showDeleteDialog = true;
        }
    }

    function confirmDelete(dontAsk: boolean) {
        if (pendingDeleteId) {
            performDelete(pendingDeleteId);
            if (dontAsk) {
                dontAskDelete = true;
                localStorage.setItem("regex_delete_dont_ask", "true");
            }
            showDeleteDialog = false;
            pendingDeleteId = null;
        }
    }

    function performDelete(id: string) {
        regexScripts = regexScripts.filter(s => s.id !== id);
        updateRegexToBackend(regexScripts);
    }
    
    function handleExportRegex() {
        if (regexScripts.length === 0) return;
        const jsonStr = JSON.stringify(regexScripts, null, 2);
        const blob = new Blob([jsonStr], { type: "application/json" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        // Filename: [CharacterName]_regex_group.json
        const safeName = (characterName || "character").replace(/[\\/:*?"<>|]/g, "_");
        a.download = `${safeName}_regex_group.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }

    async function handleSave() {
        isProcessing = true;
        try {
            const token = localStorage.getItem("auth_token");
            
            // 1. If in 'tags' mode, convert and upload content (Only for .txt target)
            if (step === 'tags') {
                const txt = convertJsonlToTxt(rawSource, selectedTags);
                const txtBlob = new Blob([txt], { type: "text/plain;charset=utf-8" });
                
                const formData = new FormData();
                formData.append('file', txtBlob, history.file_name); 

                const resContent = await fetch(`${API_BASE}/api/cards/${cardId}/history/${history.id}/content`, {
                    method: 'PUT',
                    headers: token ? { Authorization: `Bearer ${token}` } : {},
                    body: formData
                });
                if (!resContent.ok) throw new Error("Failed to update content");
            }

            // 2. Update metadata (Name, Regex)
            const payload: any = {};
            if (name !== history.display_name) {
                payload.display_name = name;
            }
            // Always check/update regex if we modified it? Or just compare?
            // Simple: just send it if JSON string different.
            const newRegexStr = JSON.stringify(regexScripts);
            if (newRegexStr !== (history.regex_scripts || "[]")) {
                payload.regex_scripts = newRegexStr;
            }

            if (Object.keys(payload).length > 0) {
                const resMeta = await fetch(`${API_BASE}/api/cards/${cardId}/history/${history.id}`, {
                    method: 'PATCH',
                    headers: {
                        'Content-Type': 'application/json',
                        ...(token ? { Authorization: `Bearer ${token}` } : {}),
                    },
                    body: JSON.stringify(payload)
                });
                if (!resMeta.ok) throw new Error("Global update failed");
            }

            // Notify parent
            onUpdate();
            toast.success("更新成功");
            open = false;
        } catch (e) {
            console.error(e);
            toast.error("更新失败");
        } finally {
            isProcessing = false;
        }
    }
    
    // API BASE fallback
    import { API_BASE } from "$lib/api";

</script>

<Dialog.Root bind:open={open}>
    <Dialog.Content class="sm:max-w-[600px] max-h-[90vh] flex flex-col">
        <Dialog.Header>
            <Dialog.Title>编辑记录</Dialog.Title>
        </Dialog.Header>

        <div class="py-4 space-y-6 flex-1 overflow-y-auto px-1">
            {#if step === 'main'}
                <div class="space-y-4">
                    <div class="space-y-2">
                        <Label>记录名称</Label>
                        <Input bind:value={name} placeholder="请输入名称" />
                    </div>

                    <!-- Config Items -->
                    {#if history.format === 'jsonl'}
                        <div class="space-y-2">
                             <Label>聊天记录正则</Label>
                             <div class="flex items-center justify-between p-3 border rounded-md bg-muted/30">
                                <div class="flex items-center gap-2 text-sm">
                                    <Regex class="h-4 w-4 text-purple-500" />
                                    <span>{regexScripts.length} 个规则</span>
                                </div>
                                <Button variant="outline" size="sm" onclick={() => step = 'regex'}>
                                    配置
                                </Button>
                             </div>
                             <p class="text-xs text-muted-foreground">
                                 仅对此聊天记录生效的显示规则 (应用顺序优先于角色卡正则)
                             </p>
                        </div>
                    {/if}

                    <div class="space-y-2">
                        <Label>源文件操作</Label>
                        {#if history.source_file_name && history.format !== 'jsonl'}
                            <div class="flex items-center justify-between gap-2 p-3 border rounded-md bg-muted/30">
                                <div class="flex items-center gap-2 text-sm min-w-0 flex-1">
                                    <FileJson class="h-4 w-4 text-blue-500 shrink-0" />
                                    <span class="truncate" title={history.source_file_name}>
                                        {history.source_file_name}
                                    </span>
                                </div>
                                <Button variant="outline" size="sm" onclick={fetchSource} disabled={isProcessing} class="shrink-0">
                                    {#if isProcessing}
                                        <Loader2 class="mr-2 h-3 w-3 animate-spin" />
                                    {/if}
                                    <RotateCcw class="mr-2 h-3 w-3" />
                                    重选标签
                                </Button>
                            </div>
                            <p class="text-xs text-muted-foreground">
                                可以通过源文件重新生成显示内容
                            </p>
                        {:else if history.format === 'jsonl'}
                             <div class="p-3 border rounded-md bg-muted/20 text-sm text-muted-foreground">
                                当前为随风模式 (Jsonl)，直接读取源文件，支持正则配置。
                             </div>
                        {:else}
                             <div class="p-3 border rounded-md bg-muted/50 text-sm text-muted-foreground text-center">
                                无关联的源文件
                             </div>
                        {/if}
                    </div>
                </div>
            {:else if step === 'tags'}
                <!-- Tag Reselection UI -->
                <div class="space-y-4">
                     <div class="flex items-center justify-between">
                         <h4 class="text-sm font-semibold">选择保留的标签</h4>
                         <Button variant="ghost" size="sm" class="h-auto p-0 text-xs" onclick={toggleAllTags}>
                            {selectedTags.length === availableTags.length ? '全不选' : '全选'}
                         </Button>
                     </div>

                     <div class="border rounded-md p-4 bg-muted/20 space-y-3 max-h-[300px] overflow-y-auto">
                        {#if availableTags.length === 0}
                            <p class="text-xs text-muted-foreground text-center py-4">未检测到任何 XML 标签对。</p>
                        {:else}
                            <div class="flex flex-wrap gap-2">
                                {#each availableTags as tag}
                                    <button 
                                        class={cn(
                                            "px-3 py-1 rounded-full text-xs border transition-colors",
                                            selectedTags.includes(tag) 
                                                ? "bg-primary text-primary-foreground border-primary" 
                                                : "bg-muted text-muted-foreground border-transparent hover:bg-muted/80"
                                        )}
                                        onclick={() => toggleTag(tag)}
                                    >
                                        {tag}
                                    </button>
                                {/each}
                            </div>
                        {/if}
                     </div>
                </div>
            {:else if step === 'regex'}
                  <!-- Regex Editor UI -->
                  <div class="flex items-center justify-between mb-2">
                      <h4 class="text-sm font-medium">正则列表 ({regexScripts.length})</h4>
                      <div class="flex items-center gap-2">
                            <Button size="sm" variant="outline" onclick={handleExportRegex} disabled={regexScripts.length === 0}>
                                <Download class="h-4 w-4 mr-1" /> 导出配置
                            </Button>
                            <input 
                                    type="file" 
                                    id="regex-import" 
                                    class="hidden" 
                                    accept=".json" 
                                    multiple
                                    onchange={handleImportRegex}
                            />
                            <Button size="sm" variant="outline" onclick={() => document.getElementById('regex-import')?.click()}>
                                <Import class="h-4 w-4 mr-1" /> 批量导入
                            </Button>
                      </div>
                  </div>
                  
                  <div class="h-[300px] border rounded-md overflow-y-auto p-2" use:dndzone={{items: regexScripts, flipDurationMs: 300, delayTouchStart: 300, dropTargetStyle: {outline: 'none'}}} onconsider={handleSortConsider} onfinalize={handleSortFinalize}>
                      {#if regexScripts.length === 0}
                          <div class="text-center text-muted-foreground text-sm py-8">暂无规则，请导入</div>
                      {:else}
                          {#each regexScripts as script (script.id)}
                              <div 
                                animate:flip={{duration: 300}}
                                class={cn(
                                    "mb-2 transition-all duration-200 relative",
                                    script.isDndShadowItem && "h-16 rounded-xl border-2 border-dashed border-primary/50 bg-primary/5",
                                    !script.isDndShadowItem && "bg-card border rounded-md shadow-sm"
                                )}
                              >
                                  {#if !script.isDndShadowItem}
                                      <div class="flex items-center justify-between p-3 cursor-grab active:cursor-grabbing">
                                          <div class="flex items-center gap-3 min-w-0">
                                              <GripVertical class="h-4 w-4 text-muted-foreground/50 shrink-0" />
                                              <div class="flex flex-col min-w-0">
                                                  <span class={cn("text-sm font-medium truncate", script.disabled && "text-muted-foreground line-through")}>
                                                      {script.scriptName || "未命名规则"}
                                                  </span>
                                                  <span class="text-xs text-muted-foreground truncate font-mono opacity-70">
                                                      {script.findRegex ? script.findRegex.substring(0, 30) : ''}...
                                                  </span>
                                              </div>
                                          </div>
                                          <div class="flex items-center gap-2 shrink-0">
                                              <Switch 
                                                  checked={!script.disabled} 
                                                  onCheckedChange={(v) => {
                                                      const idx = regexScripts.findIndex(s => s.id === script.id);
                                                      if (idx !== -1) {
                                                          regexScripts[idx].disabled = !v;
                                                          // Auto-save on toggle
                                                          updateRegexToBackend(regexScripts);
                                                      }
                                                  }}
                                                  class="scale-90"
                                              />
                                              <Button variant="ghost" size="icon" class="h-7 w-7 text-muted-foreground hover:text-destructive" onclick={() => deleteScript(script.id)}>
                                                  <Trash2 class="h-3 w-3" />
                                              </Button>
                                          </div>
                                      </div>
                                  {/if}
                              </div>
                          {/each}
                      {/if}
                  </div>
                  <p class="text-xs text-muted-foreground mt-2 flex items-center gap-1">
                      <AlertCircle class="h-3 w-3" />
                      拖拽可调整顺序，松手即自动保存。越靠前的规则越先执行。
                  </p>
            {/if}
        </div>

        <Dialog.Footer>
            {#if step === 'main'}
                <Button variant="outline" onclick={() => open = false} disabled={isProcessing}>取消</Button>
                <Button onclick={handleSave} disabled={isProcessing}>
                    {#if isProcessing}<Loader2 class="mr-2 h-4 w-4 animate-spin" />{/if}
                    保存修改
                </Button>
            {:else}
                <Button variant="outline" onclick={() => step = 'main'} disabled={isProcessing}>返回</Button>
            {/if}
        </Dialog.Footer>
    </Dialog.Content>

</Dialog.Root>

<AlertDialog.Root open={showDeleteDialog} onOpenChange={(v) => { if(!v) showDeleteDialog = false; }}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>删除确认</AlertDialog.Title>
      <AlertDialog.Description>
        您确定要删除此正则规则吗？此操作无法撤销。
      </AlertDialog.Description>
    </AlertDialog.Header>
    <div class="flex items-center space-x-2 py-4">
      <Checkbox id="dont-ask" bind:checked={dontAskDelete} onCheckedChange={(v) => dontAskDelete = v} />
      <Label for="dont-ask" class="text-sm font-normal cursor-pointer">不再询问 (仅本次会话或直到清除缓存)</Label>
    </div>
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => { showDeleteDialog = false; pendingDeleteId = null; }}>取消</AlertDialog.Cancel>
      <AlertDialog.Action onclick={() => confirmDelete(dontAskDelete)}>删除</AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
