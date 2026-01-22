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

    onMount(() => {
        fetchDeletedCards();
    });
</script>

<div class="h-full flex flex-col p-6 space-y-6">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-2xl font-bold tracking-tight">回收站</h1>
            <p class="text-muted-foreground">
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
                        退出多选
                    {:else}
                        <Square class="mr-2 h-4 w-4" />
                        多选模式
                    {/if}
                </Button>
                <Button 
                    variant="destructive" 
                    size="sm" 
                    onclick={() => showClearAllDialog = true}
                >
                    <Trash2 class="mr-2 h-4 w-4" />
                    清空回收站
                </Button>
            {/if}
            <Button variant="outline" size="sm" onclick={fetchDeletedCards}>
                <RefreshCcw class="mr-2 h-4 w-4" />
                刷新
            </Button>
        </div>
    </div>

    <!-- Multi-select Action Bar -->
    {#if selectMode && deletedCards.length > 0}
        <div class="flex items-center gap-4 p-3 rounded-lg bg-muted/50 border">
            <div class="flex items-center gap-2">
                <Button size="sm" variant="ghost" onclick={selectAll}>
                    全选
                </Button>
                <Button size="sm" variant="ghost" onclick={deselectAll}>
                    取消全选
                </Button>
            </div>
            <div class="flex-1 text-sm text-muted-foreground">
                已选择 {selectedIds.size} / {deletedCards.length} 项
            </div>
            <Button 
                size="sm" 
                variant="destructive" 
                disabled={selectedIds.size === 0}
                onclick={() => showBatchDeleteDialog = true}
            >
                <Trash2 class="mr-2 h-4 w-4" />
                批量删除 ({selectedIds.size})
            </Button>
        </div>
    {/if}

    <!-- Alert Dialog for Single Permanent Delete -->
    <AlertDialog.Root
        open={!!cardToDelete}
        onOpenChange={(open) => !open && (cardToDelete = null)}
    >
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>确定要永久删除吗？</AlertDialog.Title>
                <AlertDialog.Description>
                    此操作无法撤销。该角色卡及其所有相关数据将被永久移除。
                </AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={() => (cardToDelete = null)}
                    >取消</AlertDialog.Cancel
                >
                <AlertDialog.Action
                    class="bg-destructive !text-destructive-foreground hover:bg-destructive/90"
                    onclick={permanentDelete}
                >
                    确认永久删除
                </AlertDialog.Action>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>

    <!-- Alert Dialog for Batch Delete -->
    <AlertDialog.Root
        open={showBatchDeleteDialog}
        onOpenChange={(open) => !open && (showBatchDeleteDialog = false)}
    >
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>批量永久删除</AlertDialog.Title>
                <AlertDialog.Description>
                    确定要永久删除选中的 {selectedIds.size} 个角色卡吗？此操作无法撤销。
                </AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={() => (showBatchDeleteDialog = false)}
                    >取消</AlertDialog.Cancel
                >
                <AlertDialog.Action
                    class="bg-destructive !text-destructive-foreground hover:bg-destructive/90"
                    onclick={batchDelete}
                >
                    确认删除 {selectedIds.size} 项
                </AlertDialog.Action>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>

    <!-- Alert Dialog for Clear All -->
    <AlertDialog.Root
        open={showClearAllDialog}
        onOpenChange={(open) => !open && (showClearAllDialog = false)}
    >
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>
                    <AlertTriangle class="inline h-5 w-5 mr-2 text-destructive" />
                    清空回收站
                </AlertDialog.Title>
                <AlertDialog.Description>
                    确定要清空回收站吗？这将永久删除回收站中的全部 {deletedCards.length} 个角色卡。此操作无法撤销！
                </AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={() => (showClearAllDialog = false)}
                    >取消</AlertDialog.Cancel
                >
                <AlertDialog.Action
                    class="bg-destructive !text-destructive-foreground hover:bg-destructive/90"
                    onclick={clearAll}
                >
                    清空回收站
                </AlertDialog.Action>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>

    {#if loading}
        <div
            class="flex-1 flex items-center justify-center text-muted-foreground"
        >
            加载中...
        </div>
    {:else if deletedCards.length === 0}
        <div
            class="flex-1 flex flex-col items-center justify-center text-muted-foreground space-y-4"
        >
            <Trash2 class="h-16 w-16 opacity-20" />
            <p>回收站是空的</p>
        </div>
    {:else}
        <div class="space-y-2 max-w-7xl mx-auto w-full">
            {#each deletedCards as card (card.id)}
                <div
                    transition:fade={{ duration: 200 }}
                    class={cn(
                        "flex items-center justify-between gap-4 p-3 rounded-lg border bg-card transition-colors group h-20",
                        selectMode ? "cursor-pointer" : "",
                        selectMode && selectedIds.has(card.id) ? "bg-primary/10 border-primary/50" : "hover:bg-accent/50"
                    )}
                    onclick={() => selectMode && toggleSelect(card.id)}
                    role={selectMode ? "button" : undefined}
                >
                    <!-- Left: Checkbox (in select mode) + Avatar + Info -->
                    <div class="flex items-center gap-4 min-w-0">
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
                        <div
                            class="w-10 h-14 rounded overflow-hidden bg-muted flex-shrink-0 relative border"
                        >
                            <img
                                src={resolveUrl(card.avatar)}
                                alt={card.name}
                                class="w-full h-full object-cover grayscale opacity-70"
                            />
                        </div>

                        <!-- Title & Version -->
                        <div class="flex items-center gap-2">
                            <h3
                                class="font-medium truncate text-base"
                                title={card.name}
                            >
                                {card.name}
                            </h3>
                            {#if card.version}
                                <span
                                    class="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-mono"
                                    >v{card.version}</span
                                >
                            {/if}
                        </div>
                    </div>

                    <!-- Right: Date + Actions (hidden in select mode) -->
                    <div class="flex items-center gap-8 flex-shrink-0">
                        <div
                            class="text-sm text-muted-foreground whitespace-nowrap font-mono flex flex-col items-end"
                        >
                            <span class="text-xs text-muted-foreground/70"
                                >删除时间</span
                            >
                            <span
                                >{card.deleted_at
                                    ? new Date(card.deleted_at).toLocaleString()
                                    : "未知时间"}</span
                            >
                        </div>

                        {#if !selectMode}
                        <div
                            class="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity"
                        >
                            <Button
                                size="sm"
                                variant="secondary"
                                class="h-8 shadow-none"
                                onclick={(e: Event) => { e.stopPropagation(); restoreCard(card.id); }}
                            >
                                <RefreshCcw class="mr-2 h-3.5 w-3.5" />
                                恢复
                            </Button>
                            <Button
                                size="sm"
                                variant="destructive"
                                class="h-8 shadow-none"
                                onclick={(e: Event) => { e.stopPropagation(); cardToDelete = card.id; }}
                            >
                                <Trash2 class="mr-2 h-3.5 w-3.5" />
                                永久删除
                            </Button>
                        </div>
                        {/if}
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</div>

