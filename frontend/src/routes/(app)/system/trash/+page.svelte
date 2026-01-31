<script lang="ts">
    import { onMount } from "svelte";
    import { fade } from "svelte/transition";
    import Trash2 from "lucide-svelte/icons/trash-2";
    import RefreshCcw from "lucide-svelte/icons/refresh-ccw";
    import AlertTriangle from "lucide-svelte/icons/alert-triangle";
    import CheckSquare from "lucide-svelte/icons/check-square";
    import Square from "lucide-svelte/icons/square";
    import { toast } from "svelte-sonner";
    import { Button } from "$lib/components/ui/button/index.js";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import { cn } from "$lib/utils.js";
    import { API_BASE, resolveUrl } from "$lib/api";
    import { breadcrumbs } from "$lib/stores/breadcrumb";

    // defined locally to avoid dependency issues, or import if available
    interface DeletedCard {
        id: string;
        name: string;
        avatar?: string;
        description?: string;
        tags?: string[];
        version?: string;
        deleted_at: string | null;
    }

    let deletedCards: DeletedCard[] = $state([]);
    let loading = $state(true);
    let cardToDelete: string | null = $state(null); // ID of card to permanently delete
    
    // 多选模式
    let selectMode = $state(false);
    let selectedIds = $state<Set<string>>(new Set());
    let showBatchDeleteDialog = $state(false);
    let showClearAllDialog = $state(false);

    async function fetchDeletedCards() {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/trash/cards`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                const data = await res.json();
                deletedCards = data;
            } else {
                toast.error("加载回收站失败");
            }
        } catch (e) {
            console.error(e);
            toast.error("加载出错");
        } finally {
            loading = false;
        }
    }

    async function restoreCard(id: string) {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/trash/cards/${id}/restore`, {
                method: "POST",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });

            if (res.ok) {
                toast.success("已恢复角色卡");
                // Remove from list
                deletedCards = deletedCards.filter((c) => c.id !== id);
            } else {
                toast.error("恢复失败");
            }
        } catch (e) {
            console.error(e);
            toast.error("恢复出错");
        }
    }

    async function permanentDelete() {
        if (!cardToDelete) return;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/trash/cards/${cardToDelete}`, {
                method: "DELETE",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });

            if (res.ok) {
                toast.success("已永久删除");
                deletedCards = deletedCards.filter(
                    (c) => c.id !== cardToDelete,
                );
                cardToDelete = null;
            } else {
                toast.error("永久删除失败");
            }
        } catch (e) {
            console.error(e);
            toast.error("删除出错");
        }
    }

    // 多选相关函数
    function toggleSelectMode() {
        selectMode = !selectMode;
        if (!selectMode) {
            selectedIds = new Set();
        }
    }

    function toggleSelect(id: string) {
        const newSet = new Set(selectedIds);
        if (newSet.has(id)) {
            newSet.delete(id);
        } else {
            newSet.add(id);
        }
        selectedIds = newSet;
    }

    function selectAll() {
        selectedIds = new Set(deletedCards.map(c => c.id));
    }

    function deselectAll() {
        selectedIds = new Set();
    }

    async function batchDelete() {
        if (selectedIds.size === 0) return;
        
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/trash/cards/batch-delete`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ ids: Array.from(selectedIds) }),
            });

            if (res.ok) {
                toast.success(`已永久删除 ${selectedIds.size} 个角色卡`);
                deletedCards = deletedCards.filter(c => !selectedIds.has(c.id));
                selectedIds = new Set();
                selectMode = false;
                showBatchDeleteDialog = false;
            } else {
                toast.error("批量删除失败");
            }
        } catch (e) {
            console.error(e);
            toast.error("删除出错");
        }
    }

    async function clearAll() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/trash/cards/clear`, {
                method: "DELETE",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });

            if (res.ok) {
                const data = await res.json();
                toast.success(`已清空回收站，删除了 ${data.deleted_count} 个角色卡`);
                deletedCards = [];
                showClearAllDialog = false;
            } else {
                toast.error("清空回收站失败");
            }
        } catch (e) {
            console.error(e);
            toast.error("清空出错");
        }
    }

    async function batchRecover() {
        if (selectedIds.size === 0) return;
        
        try {
            // Parallel restore using existing single API
            // (Assuming backend loop might be complex to add right now, frontend loop is safer)
            const promises = Array.from(selectedIds).map(id => restoreCard(id));
            await Promise.all(promises);

            toast.success(`已批量恢复 ${selectedIds.size} 个角色卡`);
            // deletedCards is updated in restoreCard individually, but we might need to reset selection
            selectedIds = new Set();
            selectMode = false;
        } catch (e) {
            console.error(e);
            toast.error("批量恢复部分或全部失败");
        }
    }

    onMount(() => {
        breadcrumbs.set([
            { label: '回收站' }
        ]);
        fetchDeletedCards();
    });
</script>

<div class="h-full flex flex-col p-4 sm:p-6 space-y-4 sm:space-y-6">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-xl sm:text-2xl font-bold tracking-tight">回收站</h1>
            <p class="text-sm text-muted-foreground hidden sm:block">
                恢复误删的角色卡或永久清理数据。
            </p>
        </div>
        <div class="flex items-center gap-2">
            {#if deletedCards.length > 0}
                <Button 
                    variant={selectMode ? "default" : "outline"} 
                    size="sm" 
                    onclick={toggleSelectMode}
                >
                    {#if selectMode}
                        <CheckSquare class="mr-2 h-4 w-4" />
                        <span class="hidden sm:inline">退出多选</span>
                        <span class="sm:hidden">退出</span>
                    {:else}
                        <Square class="mr-2 h-4 w-4" />
                        <span class="hidden sm:inline">多选模式</span>
                        <span class="sm:hidden">多选</span>
                    {/if}
                </Button>
                <!-- Mobile: hide clear all to save space if needed, or keep icon only? let's keep it but simplified text -->
                <Button 
                    variant="destructive" 
                    size="sm" 
                    onclick={() => showClearAllDialog = true}
                >
                    <Trash2 class="sm:mr-2 h-4 w-4" />
                    <span class="hidden sm:inline">清空回收站</span>
                    <span class="sm:hidden">清空</span>
                </Button>
            {/if}
            <Button variant="outline" size="sm" class="w-9 px-0 sm:w-auto sm:px-3" onclick={fetchDeletedCards}>
                <RefreshCcw class="sm:mr-2 h-4 w-4" />
                <span class="hidden sm:inline">刷新</span>
            </Button>
        </div>
    </div>

    <!-- Multi-select Action Bar -->
    {#if selectMode && deletedCards.length > 0}
        <div class="flex flex-wrap items-center gap-2 sm:gap-4 p-2 sm:p-3 rounded-lg bg-muted/50 border">
            <div class="flex items-center gap-1 sm:gap-2">
                <Button size="sm" variant="ghost" onclick={selectAll} class="px-2">
                    全选
                </Button>
                <Button size="sm" variant="ghost" onclick={deselectAll} class="px-2">
                    取消
                </Button>
            </div>
            <div class="flex-1 text-xs sm:text-sm text-muted-foreground whitespace-nowrap">
                已选 {selectedIds.size}
            </div>
            <div class="flex items-center gap-2">
                <Button 
                    size="sm" 
                    variant="secondary"
                    disabled={selectedIds.size === 0}
                    onclick={batchRecover}
                >
                    <RefreshCcw class="mr-2 h-4 w-4" />
                    <span class="hidden sm:inline">批量恢复</span>
                    <span class="sm:hidden">恢复</span>
                </Button>
                <Button 
                    size="sm" 
                    variant="destructive" 
                    disabled={selectedIds.size === 0}
                    onclick={() => showBatchDeleteDialog = true}
                >
                    <Trash2 class="mr-2 h-4 w-4" />
                    <span class="hidden sm:inline">批量删除</span>
                    <span class="sm:hidden">删除</span>
                </Button>
            </div>
        </div>
    {/if}

    <!-- Alert Dialogs (Unchanged logic, compact structure) -->
    <!-- Alert Dialog for Single Permanent Delete -->
    <AlertDialog.Root open={!!cardToDelete} onOpenChange={(open) => !open && (cardToDelete = null)}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>确定要永久删除吗？</AlertDialog.Title>
                <AlertDialog.Description>此操作无法撤销。该角色卡及其所有相关数据将被永久移除。</AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={() => (cardToDelete = null)}>取消</AlertDialog.Cancel>
                <AlertDialog.Action class="bg-destructive hover:bg-destructive/90 text-white" onclick={permanentDelete}>确认永久删除</AlertDialog.Action>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>

    <!-- Alert Dialog for Batch Delete -->
    <AlertDialog.Root open={showBatchDeleteDialog} onOpenChange={(open) => !open && (showBatchDeleteDialog = false)}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>批量永久删除</AlertDialog.Title>
                <AlertDialog.Description>确定要永久删除选中的 {selectedIds.size} 个角色卡吗？此操作无法撤销。</AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={() => (showBatchDeleteDialog = false)}>取消</AlertDialog.Cancel>
                <AlertDialog.Action class="bg-destructive hover:bg-destructive/90 text-white" onclick={batchDelete}>确认删除 {selectedIds.size} 项</AlertDialog.Action>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>

    <!-- Alert Dialog for Clear All -->
    <AlertDialog.Root open={showClearAllDialog} onOpenChange={(open) => !open && (showClearAllDialog = false)}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title><AlertTriangle class="inline h-5 w-5 mr-2 text-destructive" />清空回收站</AlertDialog.Title>
                <AlertDialog.Description>确定要清空回收站吗？这将永久删除回收站中的全部 {deletedCards.length} 个角色卡。此操作无法撤销！</AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={() => (showClearAllDialog = false)}>取消</AlertDialog.Cancel>
                <AlertDialog.Action class="bg-destructive hover:bg-destructive/90 text-white" onclick={clearAll}>清空回收站</AlertDialog.Action>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>

    <!-- List -->
    {#if loading}
        <div class="flex-1 flex items-center justify-center text-muted-foreground">加载中...</div>
    {:else if deletedCards.length === 0}
        <div class="flex-1 flex flex-col items-center justify-center text-muted-foreground space-y-4">
            <Trash2 class="h-16 w-16 opacity-20" />
            <p>回收站是空的</p>
        </div>
    {:else}
        <div class="space-y-2 max-w-7xl mx-auto w-full pb-20 sm:pb-0">
            {#each deletedCards as card (card.id)}
                <div
                    transition:fade={{ duration: 200 }}
                    class={cn(
                        "flex items-center gap-3 p-3 rounded-lg border bg-card transition-colors group relative",
                        selectMode ? "cursor-pointer" : "hover:bg-accent/50",
                        selectMode && selectedIds.has(card.id) ? "bg-primary/10 border-primary/50" : ""
                    )}
                    onclick={() => selectMode && toggleSelect(card.id)}
                    role={selectMode ? "button" : undefined}
                >
                    <!-- Checkbox (in select mode) -->
                    {#if selectMode}
                        <div class="flex-shrink-0">
                            {#if selectedIds.has(card.id)}
                                <CheckSquare class="h-5 w-5 text-primary" />
                            {:else}
                                <Square class="h-5 w-5 text-muted-foreground" />
                            {/if}
                        </div>
                    {/if}
                    
                    <!-- Avatar -->
                    <div class="w-10 h-10 sm:w-10 sm:h-14 rounded overflow-hidden bg-muted flex-shrink-0 relative border">
                        <img
                            src={resolveUrl(card.avatar)}
                            alt={card.name}
                            class="w-full h-full object-cover grayscale opacity-70"
                        />
                    </div>

                    <!-- Info -->
                    <div class="flex-1 min-w-0 flex flex-col justify-center">
                        <div class="flex items-center gap-2">
                            <h3 class="font-medium truncate text-sm sm:text-base sm:max-w-xs" title={card.name}>
                                {card.name}
                            </h3>
                            {#if card.version}
                                <span class="text-[10px] px-1 py-0.5 rounded bg-muted text-muted-foreground font-mono shrink-0">
                                    v{card.version}
                                </span>
                            {/if}
                        </div>
                        <!-- Mobile Date (Optional, maybe hidden or very small) -->
                        <div class="text-xs text-muted-foreground/60 sm:hidden">
                            {card.deleted_at ? new Date(card.deleted_at).toLocaleDateString() : ""}
                        </div>
                    </div>

                    <!-- Right Side: Desktop Date + Actions -->
                    <!-- Desktop Date -->
                    <div class="hidden sm:flex flex-col items-end text-xs text-muted-foreground mr-4">
                         <span>{card.deleted_at ? new Date(card.deleted_at).toLocaleString() : ""}</span>
                    </div>

                    <!-- Actions -->
                    {#if !selectMode}
                        <div class="flex items-center gap-1 sm:gap-2 flex-shrink-0">
                            <!-- Recover -->
                            <Button
                                size="sm"
                                variant="secondary"
                                class="h-8 shadow-none px-2 sm:px-3"
                                onclick={(e: Event) => { e.stopPropagation(); restoreCard(card.id); }}
                                title="恢复"
                            >
                                <RefreshCcw class="h-4 w-4 sm:mr-2" />
                                <span class="hidden sm:inline">恢复</span>
                            </Button>
                            
                            <!-- Permanent Delete (Icon Only) -->
                            <Button
                                size="sm"
                                variant="ghost"
                                class="h-8 w-8 p-0 text-muted-foreground/50 hover:text-destructive hover:bg-destructive/10"
                                onclick={(e: Event) => { e.stopPropagation(); cardToDelete = card.id; }}
                                title="永久删除"
                            >
                                <Trash2 class="h-4 w-4" />
                            </Button>
                        </div>
                    {/if}
                </div>
            {/each}
        </div>
    {/if}
</div>

