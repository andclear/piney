<script lang="ts">
    import { onMount } from "svelte";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
    import { Input } from "$lib/components/ui/input";
    import { Button } from "$lib/components/ui/button";
    import * as Card from "$lib/components/ui/card";
    import { Plus, Upload, Book, ChevronLeft, ChevronRight } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import { goto } from "$app/navigation";
    import { cn } from "$lib/utils";
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { longpress } from "$lib/actions/longpress";

    import { API_BASE } from "$lib/api";

    let items: any[] = $state([]);
    let loading = $state(true);
    let currentPage = $state(1);
    let pageSize = $state(20);
    let totalItems = $state(0);
    let totalPages = $state(0);
    let jumpToPage = $state(1);

    onMount(async () => {
        breadcrumbs.set([{ label: "世界书", href: "/worldinfo" }]);
        await loadItems();
    });

    async function loadItems() {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const params = new URLSearchParams({
                page: currentPage.toString(),
                page_size: pageSize.toString(),
            });

            const res = await fetch(`${API_BASE}/api/world_info?${params}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (!res.ok) throw new Error("加载世界书失败");
            const data = await res.json();

             if (data.items) {
                 items = data.items;
                 totalItems = data.total;
                 currentPage = data.page;
                 pageSize = data.page_size;
                 totalPages = data.total_pages;
                 jumpToPage = currentPage;
            } else {
                 items = data; 
                 totalItems = items.length;
                 totalPages = 1;
            }

            // Parse data JSON for stats if needed
            items = items.map((item) => {
                try {
                    const data = JSON.parse(item.data);
                    return { ...item, parsedData: data };
                } catch (e) {
                    return { ...item, parsedData: {} };
                }
            });
        } catch (e) {
            console.error(e);
            toast.error("加载失败", { description: String(e) });
        } finally {
            loading = false;
        }
    }

    let isSelectionMode = $state(false);
    let selectedItemIds = $state(new Set<string>());
    let deleteDialogOpen = $state(false);
    let itemToDelete: string | null = $state(null);

    function toggleSelection(id: string) {
        if (selectedItemIds.has(id)) {
            selectedItemIds.delete(id);
        } else {
            selectedItemIds.add(id);
        }
        selectedItemIds = new Set(selectedItemIds); // Trigger reactivity
    }

    async function confirmDelete() {
        deleteDialogOpen = false;
        try {
            const token = localStorage.getItem("auth_token");
            const headers = token ? { Authorization: `Bearer ${token}` } : {};

            const idsToDelete = itemToDelete
                ? [itemToDelete]
                : Array.from(selectedItemIds);

            // Parallel delete requests (since no batch API yet)
            await Promise.all(
                idsToDelete.map((id) =>
                    fetch(`${API_BASE}/api/world_info/${id}`, {
                        method: "DELETE",
                        headers,
                    }).then((res) => {
                        if (!res.ok) throw new Error(`Delete ${id} failed`);
                    }),
                ),
            );

            toast.success("删除成功");
            await loadItems();
            selectedItemIds = new Set();
            isSelectionMode = false;
        } catch (e) {
            console.error(e);
            toast.error("部分删除失败");
            // Reload to reflect partial success
            await loadItems();
        } finally {
            itemToDelete = null;
        }
    }
</script>

<div class="container mx-auto py-6 space-y-8 animate-in fade-in duration-500">
    <!-- Header -->
    <div class="flex items-center justify-between">
        <div class="space-y-1">
            <h1 class="text-2xl font-bold tracking-tight">全局世界书</h1>
            <p class="text-muted-foreground">管理和编辑您的全局世界书设定</p>
        </div>
        <div class="flex items-center gap-2">
            <Button onclick={() => goto("/import?tab=worldbook")} class="gap-2">
                <Upload class="h-4 w-4" /> 导入世界书
            </Button>
        </div>
    </div>



    <!-- Grid -->
    {#if loading}
        <div
            class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6"
        >
            {#each Array(4) as _}
                <div class="h-32 rounded-xl bg-muted/50 animate-pulse"></div>
            {/each}
        </div>
    {:else if items.length > 0}
        <div
            class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6"
        >
            {#each items as item (item.id)}
                <ContextMenu.Root>
                    <ContextMenu.Trigger>
                        <div
                            class={cn(
                                "group relative rounded-xl border bg-card text-card-foreground shadow-sm transition-all hover:!bg-accent/40 hover:shadow-md cursor-pointer",
                                isSelectionMode &&
                                    selectedItemIds.has(item.id) &&
                                    "bg-primary/5 ring-2 ring-primary",
                            )}
                            onclick={() =>
                                isSelectionMode
                                    ? toggleSelection(item.id)
                                    : goto(`/worldinfo/${item.id}`)}
                            use:longpress
                            onlongpress={(e) => {
                                const original = e.detail.originalEvent;
                                const touch = original.touches?.[0] || original;
                                e.target.dispatchEvent(
                                    new MouseEvent("contextmenu", {
                                        bubbles: true,
                                        cancelable: true,
                                        view: window,
                                        button: 2,
                                        clientX: touch.clientX,
                                        clientY: touch.clientY,
                                    }),
                                );
                            }}
                        >
                            <div class="p-6 space-y-4">
                                <div class="flex items-start justify-between">
                                    <div
                                        class="p-2 w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center text-primary"
                                    >
                                        <Book class="h-5 w-5" />
                                    </div>
                                    <!-- Selection Checkbox or Delete Action -->
                                    {#if isSelectionMode}
                                        <input
                                            type="checkbox"
                                            class="h-5 w-5 rounded border-gray-300 text-primary accent-primary focus:ring-primary shadow-sm"
                                            checked={selectedItemIds.has(
                                                item.id,
                                            )}
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                toggleSelection(item.id);
                                            }}
                                        />
                                    {/if}
                                </div>

                                <div class="space-y-1">
                                    <h3 class="font-semibold text-lg truncate">
                                        {item.name}
                                    </h3>
                                    <p class="text-xs text-muted-foreground">
                                        条目数: {item.parsedData?.entries
                                            ? Object.keys(
                                                  item.parsedData.entries,
                                              ).length
                                            : 0}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </ContextMenu.Trigger>
                    <ContextMenu.Content>
                        <ContextMenu.Item
                            onclick={() => {
                                if (isSelectionMode) {
                                    isSelectionMode = false;
                                    selectedItemIds = new Set();
                                } else {
                                    isSelectionMode = true;
                                    selectedItemIds = new Set([item.id]);
                                }
                            }}
                        >
                            {isSelectionMode ? "取消多选" : "多选"}
                        </ContextMenu.Item>
                        <ContextMenu.Separator />
                        <ContextMenu.Item
                            class="text-destructive focus:text-destructive"
                            onclick={() => {
                                itemToDelete = item.id;
                                deleteDialogOpen = true;
                            }}
                        >
                            删除
                        </ContextMenu.Item>
                    </ContextMenu.Content>
                </ContextMenu.Root>
            {/each}
        </div>
    {:else}
        <div class="text-center py-20">
            <div
                class="mx-auto w-12 h-12 rounded-full bg-muted/50 flex items-center justify-center mb-4"
            >
                <Book class="h-6 w-6 text-muted-foreground" />
            </div>
            <h3 class="text-lg font-medium">暂无世界书</h3>
            <p class="text-muted-foreground mt-1">
                点击上方"导入世界书"添加您的第一个设定集
            </p>
        </div>
    {/if}

    <!-- Pagination Controls -->
    {#if totalPages > 1}
        <div class="mt-8 flex flex-col sm:flex-row items-center justify-between gap-4 border-t pt-6">
            <div class="text-sm text-muted-foreground">
                显示 第 <span class="font-medium">{(currentPage - 1) * pageSize + 1}</span> 到 <span class="font-medium">{Math.min(currentPage * pageSize, totalItems)}</span> 条，共 <span class="font-medium">{totalItems}</span> 条
            </div>
            
            <div class="flex items-center gap-2">
                 <Button
                    variant="outline"
                    size="sm"
                    disabled={currentPage <= 1}
                    onclick={() => {
                        currentPage--;
                        loadItems();
                    }}
                >
                    <ChevronLeft class="h-4 w-4 mr-1" /> 上一页
                </Button>
                
                <div class="flex items-center gap-2 mx-2">
                    <span class="text-sm">第</span>
                    <Input
                        type="number"
                        min="1"
                        max={totalPages}
                        bind:value={jumpToPage}
                        class="h-8 w-16 text-center"
                        onkeydown={(e) => {
                            if (e.key === 'Enter') {
                                let p = parseInt(String(jumpToPage));
                                if (isNaN(p) || p < 1) p = 1;
                                if (p > totalPages) p = totalPages;
                                currentPage = p;
                                jumpToPage = p;
                                loadItems();
                            }
                        }}
                    />
                    <span class="text-sm">页 / 共 {totalPages} 页</span>
                     <Button
                        variant="ghost"
                        size="sm"
                        class="h-8 px-2 text-xs"
                        onclick={() => {
                             let p = parseInt(String(jumpToPage));
                                if (isNaN(p) || p < 1) p = 1;
                                if (p > totalPages) p = totalPages;
                                currentPage = p;
                                jumpToPage = p;
                                loadItems();
                        }}
                    >
                        跳转
                    </Button>
                </div>

                <Button
                    variant="outline"
                    size="sm"
                    disabled={currentPage >= totalPages}
                    onclick={() => {
                        currentPage++;
                        loadItems();
                    }}
                >
                    下一页 <ChevronRight class="h-4 w-4 ml-1" />
                </Button>
            </div>
        </div>
    {/if}

    <!-- Batch Action Bar -->
    {#if isSelectionMode && selectedItemIds.size > 0}
        <div
            class="fixed bottom-6 left-1/2 -translate-x-1/2 bg-popover border shadow-lg rounded-full px-6 py-3 flex items-center gap-4 animate-in slide-in-from-bottom z-50"
        >
            <div class="text-sm font-medium">
                已选择 {selectedItemIds.size} 项
            </div>
            <div class="h-4 w-px bg-border"></div>
            <Button
                size="sm"
                variant="destructive"
                onclick={() => {
                    itemToDelete = null;
                    deleteDialogOpen = true;
                }}
            >
                删除
            </Button>
            <Button
                size="sm"
                variant="ghost"
                onclick={() => {
                    selectedItemIds = new Set();
                    isSelectionMode = false;
                }}>取消选择</Button
            >
        </div>
    {/if}

    <!-- Delete Confirmation Dialog -->
    <AlertDialog.Root bind:open={deleteDialogOpen}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>确定要删除吗？</AlertDialog.Title>
                <AlertDialog.Description>
                    此操作将永久删除 {itemToDelete
                        ? "该"
                        : `选中的 ${selectedItemIds.size} 个`}世界书及其包含的所有条目，且无法恢复。
                </AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={() => (itemToDelete = null)}
                    >取消</AlertDialog.Cancel
                >
                <AlertDialog.Action
                    class="bg-destructive !text-destructive-foreground hover:bg-destructive/90"
                    onclick={confirmDelete}
                >
                    确认删除
                </AlertDialog.Action>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>
</div>
