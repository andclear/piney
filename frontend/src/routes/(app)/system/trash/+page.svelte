<script lang="ts">
    import { onMount } from "svelte";
    import { fade } from "svelte/transition";
    import Trash2 from "lucide-svelte/icons/trash-2";
    import RefreshCcw from "lucide-svelte/icons/refresh-ccw";
    import AlertTriangle from "lucide-svelte/icons/alert-triangle";
    import { toast } from "svelte-sonner";
    import { Button } from "$lib/components/ui/button/index.js";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import { cn } from "$lib/utils.js";

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

    async function fetchDeletedCards() {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch("/api/trash/cards", {
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
            const res = await fetch(`/api/trash/cards/${id}/restore`, {
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
            const res = await fetch(`/api/trash/cards/${cardToDelete}`, {
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

    onMount(() => {
        fetchDeletedCards();
    });
</script>

<div class="h-full flex flex-col p-6 space-y-6">
    <div class="flex items-center justify-between">
        <div>
            <h2 class="text-2xl font-bold tracking-tight">回收站</h2>
            <p class="text-muted-foreground">
                恢复误删的角色卡或永久清理数据。
            </p>
        </div>
        <Button variant="outline" size="sm" onclick={fetchDeletedCards}>
            <RefreshCcw class="mr-2 h-4 w-4" />
            刷新
        </Button>
    </div>

    <!-- Alert Dialog for Permanent Delete -->
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
                    class="flex items-center justify-between gap-4 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors group h-20"
                >
                    <!-- Left: Avatar + Info -->
                    <div class="flex items-center gap-4 min-w-0">
                        <!-- Avatar -->
                        <div
                            class="w-10 h-14 rounded overflow-hidden bg-muted flex-shrink-0 relative border"
                        >
                            <img
                                src={card.avatar || "/default.webp"}
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

                    <!-- Right: Date + Actions -->
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

                        <div
                            class="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity"
                        >
                            <Button
                                size="sm"
                                variant="secondary"
                                class="h-8 shadow-none"
                                onclick={() => restoreCard(card.id)}
                            >
                                <RefreshCcw class="mr-2 h-3.5 w-3.5" />
                                恢复
                            </Button>
                            <Button
                                size="sm"
                                variant="destructive"
                                class="h-8 shadow-none"
                                onclick={() => (cardToDelete = card.id)}
                            >
                                <Trash2 class="mr-2 h-3.5 w-3.5" />
                                永久删除
                            </Button>
                        </div>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</div>
