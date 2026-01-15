<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { flip } from "svelte/animate";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Badge } from "$lib/components/ui/badge";
    import * as Sheet from "$lib/components/ui/sheet";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as Tabs from "$lib/components/ui/tabs";
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import * as Pagination from "$lib/components/ui/pagination";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";

    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { longpress } from "$lib/actions/longpress";
    import { API_BASE, resolveUrl } from "$lib/api";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
    import {
        Search,
        Grid3X3,
        List,
        Filter,
        Plus,
        Eye,
        EyeClosed,
        X,
        GripVertical,
        Trash2,
        Edit2,
        CheckSquare,
        Hash,
        Upload,
        ChevronLeft,
        ChevronRight,
        ArrowUpDown,
    } from "lucide-svelte";

    // ============ 类型定义 ============
    interface Category {
        id: string;
        name: string;
        sort_order: number;
    }

    interface CardItem {
        id: string;
        name: string;
        description: string | null;
        author: string | null;
        avatar: string | null;
        category_id: string | null;
        tags: string[];
        rating: number;
        cover_blur: boolean;
        version: string | null;
        created_at: string;
        updated_at: string;
    }

    // ============ 状态 ============
    let viewMode: "gallery" | "list" = $state("gallery");
    let searchQuery = $state("");
    let selectedCategoryId: string | null = $state(null);
    let selectedTags: string[] = $state([]);
    let filterOpen = $state(false);
    let categoryDialogOpen = $state(false);

    // 批量选择
    let isSelectionMode = $state(false);
    let selectedCardIds = $state(new Set<string>());

    let deleteDialogOpen = $state(false);
    let cardToDelete: string | null = $state(null); // ID of card to soft delete (single)
    let isBatchDeleteArgs = $state(false); // Whether the dialog is for batch delete
    let moveDialogOpen = $state(false);
    let targetCategoryId: string | null = $state(null);

    let categories: Category[] = $state([]);
    let cards: CardItem[] = $state([]);
    let allTags: string[] = $state([]);
    let tagCounts: Record<string, number> = $state({});

    let loading = $state(true);
    let newCategoryName = $state("");
    let editingCategory: Category | null = $state(null);

    // 拖拽状态
    let draggedCategoryId: string | null = $state(null);

    // 排序状态
    let currentSort = $state("updated_at");
    let currentOrder = $state("desc");

    // 分页状态
    let currentPage = $state(1);
    let pageSize = $state(20);
    let totalItems = $state(0);
    let totalPages = $state(0);
    let jumpToPage = $state(1); // For the input field

    // ============ API 调用 ============
    async function fetchCategories() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/categories`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                categories = await res.json();
            }
        } catch (e) {
            console.error("获取分类失败", e);
        }
    }

    async function fetchCards() {
        try {
            const token = localStorage.getItem("auth_token");
            let url = `${API_BASE}/api/cards`;
            const params = new URLSearchParams();
            if (selectedCategoryId)
                params.set("category_id", selectedCategoryId);
            if (searchQuery) params.set("search", searchQuery);
            
            // Pagination params
            params.set("page", currentPage.toString());
            params.set("page_size", pageSize.toString());

            // Sorting params
            params.set("sort", currentSort);
            params.set("order", currentOrder);

            if (params.toString()) url += `?${params.toString()}`;

            const res = await fetch(url, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                const data = await res.json();
                
                // Handle paginated response
                if (data.items) {
                    cards = data.items;
                    totalItems = data.total;
                    currentPage = data.page;
                    pageSize = data.page_size;
                    totalPages = data.total_pages;
                    jumpToPage = currentPage;
                } else {
                    // Fallback for old API if something goes wrong (should not happen with updated backend)
                    cards = data;
                    totalItems = cards.length;
                    totalPages = 1;
                }

                // Collect tags from current page (or should it be all? Usually all is better for sidebar, but API only returns page. 
                // Note: Tag stats might only reflect current page now, which is a trade-off unless we have a separate tags API)
                // For now, we calculate from visible cards.
                const counts: Record<string, number> = {};
                const tagSet = new Set<string>();
                cards.forEach((c) => {
                    c.tags.forEach((t) => {
                        tagSet.add(t);
                        counts[t] = (counts[t] || 0) + 1;
                    });
                });
                
                // If we want comprehensive tags, we might need a separate API call. 
                // But for now, let's keep it based on visible cards or accept limitation.
                // Or maybe we should append tags? No, let's stick to visible.
                allTags = Array.from(tagSet).sort();
                tagCounts = counts;
            }
        } catch (e) {
            console.error("获取角色卡失败", e);
        }
    }

    async function createCategory() {
        if (!newCategoryName.trim()) {
            toast.error("分类名称不能为空");
            return;
        }
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/categories`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ name: newCategoryName.trim() }),
            });
            if (res.ok) {
                newCategoryName = "";
                await fetchCategories();
                toast.success("分类创建成功");
            } else {
                const errorText = await res.text();
                toast.error(`创建失败: ${res.status}`);
            }
        } catch (e) {
            console.error("Create category error:", e);
            toast.error("创建分类失败");
        }
    }

    async function updateCategory(id: string, name: string) {
        try {
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/categories/${id}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ name }),
            });
            await fetchCategories();
            editingCategory = null;
            toast.success("分类已更新");
        } catch (e) {
            toast.error("更新分类失败");
        }
    }

    async function deleteCategory(id: string) {
        const cat = categories.find((c) => c.id === id);
        const cardCount = cards.filter((c) => c.category_id === id).length;

        if (cardCount > 0) {
            const confirmed = confirm(
                `分类"${cat?.name}"中包含 ${cardCount} 个角色，删除后这些角色将被移到"全部"分类。确认删除？`,
            );
            if (!confirmed) return;
        }

        try {
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/categories/${id}`, {
                method: "DELETE",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            await fetchCategories();
            await fetchCards();
            toast.success("分类已删除");
        } catch (e) {
            toast.error("删除分类失败");
        }
    }

    async function toggleCoverBlur(card: CardItem) {
        try {
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/cards/${card.id}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ cover_blur: !card.cover_blur }),
            });
            card.cover_blur = !card.cover_blur;
            cards = [...cards]; // 触发响应式更新
        } catch (e) {
            toast.error("更新失败");
        }
    }

    // ============ 拖拽排序 ============
    function handleDragStart(categoryId: string) {
        draggedCategoryId = categoryId;
    }

    function handleDragOver(e: DragEvent) {
        e.preventDefault();
    }

    function handleDrop(targetCategoryId: string) {
        if (!draggedCategoryId || draggedCategoryId === targetCategoryId) {
            draggedCategoryId = null;
            return;
        }

        // 重新排序
        const draggedIndex = categories.findIndex(
            (c) => c.id === draggedCategoryId,
        );
        const targetIndex = categories.findIndex(
            (c) => c.id === targetCategoryId,
        );

        if (draggedIndex === -1 || targetIndex === -1) {
            draggedCategoryId = null;
            return;
        }

        // 移动元素
        const newCategories = [...categories];
        const [removed] = newCategories.splice(draggedIndex, 1);
        newCategories.splice(targetIndex, 0, removed);
        categories = newCategories;

        // 调用 API 保存排序
        saveOrder(newCategories.map((c) => c.id));
        draggedCategoryId = null;
    }

    async function saveOrder(ids: string[]) {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/categories/reorder`, {
                method: "PUT",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ ids }),
            });
            if (!res.ok) {
                throw new Error(`HTTP ${res.status}`);
            }
        } catch (e) {
            console.error("保存排序失败", e);
            toast.error("保存排序失败");
            await fetchCategories(); // 恢复原始排序
        }
    }

    // ============ 批量操作 ============
    function toggleSelectionMode() {
        isSelectionMode = !isSelectionMode;
        selectedCardIds = new Set();
    }

    function toggleCardSelection(id: string) {
        if (selectedCardIds.has(id)) {
            selectedCardIds.delete(id);
        } else {
            selectedCardIds.add(id);
        }
        selectedCardIds = new Set(selectedCardIds); // 触发响应式更新
    }

    function handleBatchMove() {
        if (selectedCardIds.size === 0) return;
        targetCategoryId = null; // 默认选中"无分类/全部"？或者让用户选
        moveDialogOpen = true;
    }

    async function confirmBatchMove() {
        try {
            const token = localStorage.getItem("auth_token");
            console.log(
                "Batch move:",
                Array.from(selectedCardIds),
                targetCategoryId,
            );
            const res = await fetch(`${API_BASE}/api/cards/batch/category`, {
                method: "PUT",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({
                    ids: Array.from(selectedCardIds),
                    category_id:
                        targetCategoryId === "null" ? null : targetCategoryId,
                }),
            });

            if (res.ok) {
                toast.success("批量移动成功");
                moveDialogOpen = false;
                isSelectionMode = false;
                selectedCardIds = new Set();
                await fetchCards();
            } else {
                toast.error("批量移动失败");
            }
        } catch (e) {
            console.error("Batch move error:", e);
            toast.error("批量移动失败");
        }
    }

    function softDeleteCard(id: string) {
        if (
            isSelectionMode &&
            selectedCardIds.has(id) &&
            selectedCardIds.size > 1
        ) {
            isBatchDeleteArgs = true;
            deleteDialogOpen = true;
            return;
        }

        cardToDelete = id;
        isBatchDeleteArgs = false;
        deleteDialogOpen = true;
    }

    async function confirmDelete() {
        deleteDialogOpen = false;

        if (isBatchDeleteArgs) {
            try {
                const token = localStorage.getItem("auth_token");
                const res = await fetch(`${API_BASE}/api/cards/batch/delete`, {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json",
                        ...(token ? { Authorization: `Bearer ${token}` } : {}),
                    },
                    body: JSON.stringify({ ids: Array.from(selectedCardIds) }),
                });
                if (res.ok) {
                    toast.success("批量删除成功");
                    selectedCardIds = new Set();
                    await fetchCards();
                } else {
                    toast.error("批量删除失败");
                }
            } catch (e) {
                toast.error("批量删除出错");
                console.error(e);
            }
        } else if (cardToDelete) {
            try {
                const token = localStorage.getItem("auth_token");
                const res = await fetch(`${API_BASE}/api/cards/${cardToDelete}`, {
                    method: "DELETE",
                    headers: token ? { Authorization: `Bearer ${token}` } : {},
                });
                if (res.ok) {
                    toast.success("已移至回收站");
                    await fetchCards();
                } else {
                    toast.error("删除失败");
                }
            } catch (e) {
                toast.error("删除失败");
                console.error(e);
            }
        }
        cardToDelete = null;
        isBatchDeleteArgs = false;
    }

    // ============ 生命周期 ============
    onMount(async () => {
        breadcrumbs.set([{ label: "角色库" }]);
        await Promise.all([fetchCategories(), fetchCards()]);
        loading = false;
    });

    // ============ 响应式 ============
    let filteredCards = $derived(
        cards.filter((card) => {
            if (
                selectedTags.length > 0 &&
                !selectedTags.some((t) => card.tags.includes(t))
            ) {
                return false;
            }
            return true;
        }),
    );

    function selectCategory(id: string | null) {
        selectedCategoryId = id;
        fetchCards();
    }

    function toggleTag(tag: string) {
        if (selectedTags.includes(tag)) {
            selectedTags = selectedTags.filter((t) => t !== tag);
        } else {
            selectedTags = [...selectedTags, tag];
        }
    }

    function clearTagFilter() {
        selectedTags = [];
    }

    function handleSearch() {
        fetchCards();
    }
</script>

<div class="container py-6 space-y-6 max-w-7xl mx-auto">
    <!-- 页面标题 -->
    <div class="flex items-center justify-between">
        <div class="space-y-1">
            <h1 class="text-2xl font-bold tracking-tight">我的角色</h1>
            <p class="text-muted-foreground">
                管理您的 {totalItems} 个角色卡片
            </p>
        </div>
        <div class="flex gap-2">
            <Button class="gap-2">
                <Upload class="h-4 w-4" />
                <a href="/import">导入角色</a>
            </Button>
            <Button class="gap-2">
                <Plus class="h-4 w-4" />
                新建角色
            </Button>
        </div>
    </div>

    <!-- 搜索栏 -->
    <div class="flex items-center gap-3">
        <div class="relative flex-1">
            <Search
                class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground"
            />
            <Input
                placeholder="搜索角色名称、设定..."
                class="pl-10"
                bind:value={searchQuery}
                onkeydown={(e) => e.key === "Enter" && handleSearch()}
            />
        </div>

        <!-- 视图切换 -->
        <div class="flex items-center border rounded-lg p-1 gap-1">
            <button
                class={cn(
                    "p-2 rounded transition-colors",
                    viewMode === "gallery"
                        ? "bg-primary text-primary-foreground"
                        : "hover:bg-muted",
                )}
                onclick={() => (viewMode = "gallery")}
            >
                <Grid3X3 class="h-4 w-4" />
            </button>
            <button
                class={cn(
                    "p-2 rounded transition-colors",
                    viewMode === "list"
                        ? "bg-primary text-primary-foreground"
                        : "hover:bg-muted",
                )}
                onclick={() => (viewMode = "list")}
            >
                <List class="h-4 w-4" />
            </button>
        </div>

        <DropdownMenu.Root>
            <DropdownMenu.Trigger>
                <Button variant="outline" size="icon">
                    <ArrowUpDown class="h-4 w-4" />
                </Button>
            </DropdownMenu.Trigger>
            <DropdownMenu.Content align="end">
                <DropdownMenu.Label>排序方式</DropdownMenu.Label>
                <DropdownMenu.Separator />
                <DropdownMenu.RadioGroup value={`${currentSort}-${currentOrder}`}>
                    <DropdownMenu.RadioItem 
                        value="updated_at-desc"
                        onclick={() => {
                            currentSort = "updated_at";
                            currentOrder = "desc";
                            currentPage = 1;
                            fetchCards();
                        }}
                    >
                        最后更新 (默认)
                    </DropdownMenu.RadioItem>
                    <DropdownMenu.RadioItem 
                        value="created_at-desc"
                        onclick={() => {
                            currentSort = "created_at";
                            currentOrder = "desc";
                            currentPage = 1;
                            fetchCards();
                        }}
                    >
                        创建时间 (最新)
                    </DropdownMenu.RadioItem>
                    <DropdownMenu.RadioItem 
                        value="created_at-asc"
                        onclick={() => {
                            currentSort = "created_at";
                            currentOrder = "asc";
                            currentPage = 1;
                            fetchCards();
                        }}
                    >
                        创建时间 (最早)
                    </DropdownMenu.RadioItem>
                    <DropdownMenu.RadioItem 
                        value="name-asc"
                        onclick={() => {
                            currentSort = "name";
                            currentOrder = "asc";
                            currentPage = 1;
                            fetchCards();
                        }}
                    >
                        名称 (A-Z)
                    </DropdownMenu.RadioItem>
                </DropdownMenu.RadioGroup>
            </DropdownMenu.Content>
        </DropdownMenu.Root>

        <!-- 筛选按钮 -->
        <Sheet.Root bind:open={filterOpen}>
            <Sheet.Trigger
                class="inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground h-9 px-4 py-2 rounded-md"
            >
                <Filter class="h-4 w-4" />
                筛选
                {#if selectedTags.length > 0}
                    <Badge variant="secondary" class="ml-1"
                        >{selectedTags.length}</Badge
                    >
                {/if}
            </Sheet.Trigger>
            <Sheet.Content
                side="right"
                class="w-[400px] flex flex-col p-0 sm:max-w-[400px]"
            >
                <div
                    class="px-6 py-4 flex items-center justify-between border-b"
                >
                    <div class="flex items-center gap-2 font-bold text-lg">
                        <Filter class="h-5 w-5 text-primary" />
                        标签筛选
                    </div>
                </div>
                <div class="flex-1 overflow-y-auto px-6 py-4">
                    <div class="flex items-center justify-between mb-4">
                        <span
                            class="flex items-center gap-2 text-sm font-medium text-muted-foreground"
                        >
                            <Hash class="h-4 w-4" /> 标签筛选
                        </span>
                        <span class="text-xs text-muted-foreground">可多选</span
                        >
                    </div>
                    <div class="flex flex-wrap gap-3">
                        {#each allTags as tag}
                            <button
                                class={cn(
                                    "inline-flex items-center justify-between rounded-lg border px-3 py-2 text-sm font-medium transition-all hover:bg-accent hover:text-accent-foreground",
                                    selectedTags.includes(tag)
                                        ? "border-primary bg-primary/5 ring-1 ring-primary"
                                        : "bg-background",
                                )}
                                onclick={() => toggleTag(tag)}
                            >
                                <span>{tag}</span>
                                <Badge
                                    variant="secondary"
                                    class="ml-2 h-5 min-w-5 px-1 justify-center bg-muted-foreground/10 hover:bg-muted-foreground/20 text-muted-foreground"
                                >
                                    {tagCounts[tag] || 0}
                                </Badge>
                            </button>
                        {/each}
                        {#if allTags.length === 0}
                            <div
                                class="col-span-full w-full text-center text-muted-foreground py-8"
                            >
                                暂无标签
                            </div>
                        {/if}
                    </div>
                </div>
                <div class="p-4 border-t flex gap-4 mt-auto">
                    <Button
                        variant="outline"
                        class="flex-1 h-11"
                        onclick={clearTagFilter}
                    >
                        重置
                    </Button>
                    <Button
                        class="flex-1 h-11 bg-primary text-primary-foreground hover:bg-primary/90"
                        onclick={() => (filterOpen = false)}
                    >
                        确认 ({filteredCards.length})
                    </Button>
                </div>
            </Sheet.Content>
        </Sheet.Root>
    </div>

    <!-- 分类栏 -->
    <div class="flex items-center gap-2 overflow-x-auto pb-2">
        <button
            class={cn(
                "px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-colors",
                selectedCategoryId === null
                    ? "bg-primary text-primary-foreground"
                    : "hover:bg-muted",
            )}
            onclick={() => selectCategory(null)}
        >
            全部
        </button>
        {#each categories as category}
            <button
                class={cn(
                    "px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-colors",
                    selectedCategoryId === category.id
                        ? "bg-primary text-primary-foreground"
                        : "hover:bg-muted",
                )}
                onclick={() => selectCategory(category.id)}
            >
                {category.name}
            </button>
        {/each}

        <!-- 管理分类按钮 -->
        <Dialog.Root bind:open={categoryDialogOpen}>
            <Dialog.Trigger
                class="px-3 py-2 rounded-lg text-sm font-medium whitespace-nowrap hover:bg-muted transition-colors"
            >
                <Plus class="h-4 w-4" />
            </Dialog.Trigger>
            <Dialog.Content class="max-w-md">
                <Dialog.Header>
                    <Dialog.Title>管理分类</Dialog.Title>
                </Dialog.Header>
                <div class="space-y-4 py-4">
                    <!-- 新建分类 -->
                    <div class="flex gap-2">
                        <Input
                            placeholder="新分类名称"
                            bind:value={newCategoryName}
                            onkeydown={(e) =>
                                e.key === "Enter" && createCategory()}
                        />
                        <Button onclick={createCategory}>添加</Button>
                    </div>

                    <!-- 分类列表 -->
                    <div class="space-y-2 max-h-60 overflow-y-auto">
                        {#each categories as category (category.id)}
                            <div
                                class="flex items-center gap-2 p-2 rounded-lg border transition-colors bg-background"
                                class:bg-accent={draggedCategoryId ===
                                    category.id}
                                draggable="true"
                                ondragstart={() => handleDragStart(category.id)}
                                ondragover={handleDragOver}
                                ondrop={() => handleDrop(category.id)}
                                animate:flip={{ duration: 200 }}
                            >
                                <GripVertical
                                    class="h-4 w-4 text-muted-foreground cursor-grab"
                                />
                                {#if editingCategory?.id === category.id}
                                    <Input
                                        class="flex-1 h-8"
                                        value={editingCategory.name}
                                        oninput={(e) => {
                                            if (editingCategory) {
                                                editingCategory.name =
                                                    e.currentTarget.value;
                                            }
                                        }}
                                        onkeydown={(e) => {
                                            if (
                                                e.key === "Enter" &&
                                                editingCategory
                                            ) {
                                                updateCategory(
                                                    editingCategory.id,
                                                    editingCategory.name,
                                                );
                                            }
                                        }}
                                    />
                                    <Button
                                        size="sm"
                                        variant="ghost"
                                        onclick={() =>
                                            editingCategory &&
                                            updateCategory(
                                                editingCategory.id,
                                                editingCategory.name,
                                            )}
                                    >
                                        保存
                                    </Button>
                                {:else}
                                    <span class="flex-1">{category.name}</span>
                                    <Button
                                        size="icon"
                                        variant="ghost"
                                        class="h-8 w-8"
                                        onclick={() =>
                                            (editingCategory = { ...category })}
                                    >
                                        <Edit2 class="h-4 w-4" />
                                    </Button>
                                {/if}
                                <Button
                                    size="icon"
                                    variant="ghost"
                                    class="h-8 w-8 text-destructive"
                                    onclick={() => deleteCategory(category.id)}
                                >
                                    <Trash2 class="h-4 w-4" />
                                </Button>
                            </div>
                        {/each}
                        {#if categories.length === 0}
                            <p
                                class="text-center text-muted-foreground py-4 text-sm"
                            >
                                暂无分类
                            </p>
                        {/if}
                    </div>
                </div>
            </Dialog.Content>
        </Dialog.Root>
    </div>

    <!-- 角色卡列表 -->
    {#if loading}
        <div class="flex items-center justify-center py-20">
            <div
                class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"
            ></div>
        </div>
    {:else if filteredCards.length === 0}
        <div class="text-center py-20 text-muted-foreground">
            <p>暂无角色卡</p>
        </div>
    {:else if viewMode === "gallery"}
        <!-- 画廊视图 -->
        <div
            class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-5"
        >
            {#each filteredCards as card (card.id)}
                <ContextMenu.Root>
                    <ContextMenu.Trigger>
                        <div
                            role="button"
                            tabindex="0"
                            class="group relative rounded-xl overflow-hidden bg-card shadow-md hover:shadow-xl transition-all duration-200 hover:-translate-y-1 cursor-pointer"
                            class:ring-2={isSelectionMode &&
                                selectedCardIds.has(card.id)}
                            class:ring-primary={isSelectionMode &&
                                selectedCardIds.has(card.id)}
                            onclick={() =>
                                isSelectionMode
                                    ? toggleCardSelection(card.id)
                                    : goto(`/characters/${card.id}`)}
                            onkeydown={(e) => {
                                if (e.key === "Enter" || e.key === " ") {
                                    isSelectionMode
                                        ? toggleCardSelection(card.id)
                                        : goto(`/characters/${card.id}`);
                                }
                            }}
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
                            <!-- 封面图容器 -->
                            <div class="aspect-[2/3] relative overflow-hidden">
                                <img
                                    src={resolveUrl(card.avatar ? `${card.avatar}?v=${new Date(card.updated_at).getTime()}` : "/default.webp")}
                                    alt={card.name}
                                    class={cn(
                                        "w-full h-full object-cover",
                                        card.cover_blur && "blur-xl scale-110",
                                    )}
                                />

                                <!-- 左上角：版本号（只有非1.0时显示） -->
                                {#if card.version && card.version !== "1.0"}
                                    <div
                                        class="absolute top-3 left-3 bg-black/60 text-white px-2.5 py-1 rounded-full text-xs font-medium backdrop-blur-sm"
                                    >
                                        {card.version}
                                    </div>
                                {/if}

                                <!-- 右上角：眼睛图标 -->
                                <!-- 右上角：眼睛图标 OR Checkbox -->
                                {#if isSelectionMode}
                                    <div class="absolute top-3 right-3 z-10">
                                        <input
                                            type="checkbox"
                                            class="h-5 w-5 rounded border-gray-300 text-primary accent-primary focus:ring-primary shadow-sm"
                                            checked={selectedCardIds.has(
                                                card.id,
                                            )}
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                toggleCardSelection(card.id);
                                            }}
                                        />
                                    </div>
                                {:else}
                                    <button
                                        class="absolute top-3 right-3 p-2 rounded-full bg-black/60 text-white opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity backdrop-blur-sm hover:bg-black/80"
                                        onclick={(e) => {
                                            e.stopPropagation();
                                            toggleCoverBlur(card);
                                        }}
                                    >
                                        {#if card.cover_blur}
                                            <EyeClosed class="h-4 w-4" />
                                        {:else}
                                            <Eye class="h-4 w-4" />
                                        {/if}
                                    </button>
                                {/if}

                                <!-- 底部渐变遮罩和内容 -->
                                <div
                                    class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent pt-16 pb-4 px-4"
                                >
                                    <!-- 标题 -->
                                    <h3
                                        class="font-bold text-white text-base truncate mb-2"
                                        title={card.name}
                                    >
                                        {card.name}
                                    </h3>

                                    <!-- 标签 -->
                                    {#if card.tags && card.tags.length > 0}
                                        <div
                                            class="flex gap-1.5 overflow-hidden"
                                        >
                                            {#each card.tags.slice(0, 3) as tag}
                                                <span
                                                    class="text-[10px] px-1.5 py-0.5 rounded-md border border-white/30 text-white/90 bg-white/10 backdrop-blur-sm whitespace-nowrap"
                                                    >{tag}</span
                                                >
                                            {/each}
                                            {#if card.tags.length > 3}
                                                <span
                                                    class="text-[10px] text-white/60 whitespace-nowrap"
                                                    >+{card.tags.length -
                                                        3}</span
                                                >
                                            {/if}
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    </ContextMenu.Trigger>
                    <ContextMenu.Content>
                        <ContextMenu.Item
                            onclick={() => {
                                if (isSelectionMode) {
                                    isSelectionMode = false;
                                    selectedCardIds = new Set();
                                } else {
                                    isSelectionMode = true;
                                }
                            }}
                        >
                            {isSelectionMode ? "取消选择" : "多选"}
                        </ContextMenu.Item>
                        <ContextMenu.Separator />
                        <ContextMenu.Item
                            class="text-destructive focus:text-destructive"
                            onclick={() => softDeleteCard(card.id)}
                        >
                            删除
                        </ContextMenu.Item>
                    </ContextMenu.Content>
                </ContextMenu.Root>
            {/each}
        </div>

        <!-- 底部批量操作栏 -->
        {#if isSelectionMode && selectedCardIds.size > 0}
            <div
                class="fixed bottom-6 left-1/2 -translate-x-1/2 bg-popover border shadow-lg rounded-full px-6 py-3 flex items-center gap-4 animate-in slide-in-from-bottom"
            >
                <div class="text-sm font-medium">
                    已选择 {selectedCardIds.size} 项
                </div>
                <div class="h-4 w-px bg-border"></div>
                <Button size="sm" onclick={handleBatchMove}>移动到分类</Button>
                <Button
                    size="sm"
                    variant="destructive"
                    onclick={() => {
                        isBatchDeleteArgs = true;
                        deleteDialogOpen = true;
                    }}
                >
                    删除
                </Button>
                <Button
                    size="sm"
                    variant="ghost"
                    onclick={() => {
                        selectedCardIds = new Set();
                        isSelectionMode = false;
                    }}>取消选择</Button
                >
            </div>
        {/if}

        <!-- 移动分类对话框 -->
        <Dialog.Root bind:open={moveDialogOpen}>
            <Dialog.Content class="max-w-sm">
                <Dialog.Header>
                    <Dialog.Title>移动到分类</Dialog.Title>
                </Dialog.Header>
                <div class="py-4 space-y-2">
                    <p class="text-sm text-muted-foreground mb-4">
                        将选中的 {selectedCardIds.size} 个角色移动到：
                    </p>
                    <div class="grid grid-cols-2 gap-2">
                        <button
                            class={cn(
                                "px-4 py-2 rounded-lg text-sm font-medium border hover:bg-accent transition-colors text-left",
                                targetCategoryId === "null" &&
                                    "border-primary bg-primary/5",
                            )}
                            onclick={() => (targetCategoryId = "null")}
                        >
                            无分类 (全部)
                        </button>
                        {#each categories as category}
                            <button
                                class={cn(
                                    "px-4 py-2 rounded-lg text-sm font-medium border hover:bg-accent transition-colors text-left truncate",
                                    targetCategoryId === category.id &&
                                        "border-primary bg-primary/5",
                                )}
                                onclick={() => (targetCategoryId = category.id)}
                            >
                                {category.name}
                            </button>
                        {/each}
                    </div>
                </div>
                <Dialog.Footer>
                    <Button
                        variant="ghost"
                        onclick={() => (moveDialogOpen = false)}
                    >
                        取消
                    </Button>
                    <Button
                        disabled={targetCategoryId === null}
                        onclick={confirmBatchMove}
                    >
                        确认移动
                    </Button>
                </Dialog.Footer>
            </Dialog.Content>
        </Dialog.Root>
    {:else}
        <!-- 列表视图 -->
        <div class="space-y-2">
            {#each filteredCards as card (card.id)}
                <ContextMenu.Root>
                    <ContextMenu.Trigger>
                        <div
                            role="button"
                            tabindex="0"
                            class={cn(
                                "flex items-center gap-4 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors",
                                isSelectionMode &&
                                    selectedCardIds.has(card.id) &&
                                    "bg-primary/10 ring-2 ring-primary",
                            )}
                            onclick={() =>
                                isSelectionMode
                                    ? toggleCardSelection(card.id)
                                    : null}
                            onkeydown={(e) => {
                                if (e.key === "Enter" || e.key === " ") {
                                    isSelectionMode
                                        ? toggleCardSelection(card.id)
                                        : null;
                                }
                            }}
                        >
                            <!-- 缩略图 -->
                            <div
                                class="w-10 h-14 rounded overflow-hidden bg-muted flex-shrink-0"
                            >
                                <img
                                    src={resolveUrl(card.avatar)}
                                    alt={card.name}
                                    class={cn(
                                        "w-full h-full object-cover",
                                        card.cover_blur && "blur-xl",
                                    )}
                                />
                            </div>

                            <!-- 信息 -->
                            <div class="flex-1 min-w-0">
                                <h3 class="font-medium truncate">
                                    {card.name}
                                </h3>
                                <div class="flex items-center gap-2 mt-1">
                                    {#if card.version && card.version !== "1.0"}
                                        <span
                                            class="text-xs px-1.5 py-0.5 rounded bg-primary/10 text-primary font-mono"
                                            >{card.version}</span
                                        >
                                    {/if}
                                    {#if card.tags && card.tags.length > 0}
                                        {#each card.tags.slice(0, 3) as tag}
                                            <Badge
                                                variant="secondary"
                                                class="text-xs">{tag}</Badge
                                            >
                                        {/each}
                                    {/if}
                                </div>
                            </div>

                            <!-- 操作 -->
                            {#if isSelectionMode}
                                <input
                                    type="checkbox"
                                    class="h-5 w-5 rounded border-gray-300 text-primary focus:ring-primary shadow-sm mr-2"
                                    checked={selectedCardIds.has(card.id)}
                                    onclick={(e) => {
                                        e.stopPropagation();
                                        toggleCardSelection(card.id);
                                    }}
                                />
                            {:else}
                                <button
                                    class="p-2 rounded hover:bg-muted transition-colors"
                                    onclick={(e) => {
                                        e.stopPropagation();
                                        toggleCoverBlur(card);
                                    }}
                                >
                                    {#if card.cover_blur}
                                        <EyeClosed class="h-4 w-4" />
                                    {:else}
                                        <Eye class="h-4 w-4" />
                                    {/if}
                                </button>
                            {/if}
                        </div>
                    </ContextMenu.Trigger>
                    <ContextMenu.Content>
                        <ContextMenu.Item
                            onclick={() => toggleCardSelection(card.id)}
                        >
                            {selectedCardIds.has(card.id) ? "取消选择" : "选择"}
                        </ContextMenu.Item>
                        <ContextMenu.Item
                            onclick={() => {
                                if (isSelectionMode) {
                                    isSelectionMode = false;
                                    selectedCardIds = new Set();
                                } else {
                                    isSelectionMode = true;
                                }
                            }}
                        >
                            {isSelectionMode ? "取消多选" : "多选"}
                        </ContextMenu.Item>
                        <ContextMenu.Separator />
                        <ContextMenu.Item
                            class="text-destructive focus:text-destructive"
                            onclick={() => softDeleteCard(card.id)}
                        >
                            <Trash2 class="mr-2 h-4 w-4" />
                            删除
                        </ContextMenu.Item>
                    </ContextMenu.Content>
                </ContextMenu.Root>
            {/each}
        </div>
    {/if}

    <!-- Pagination Controls -->
    {#if totalPages > 1}
        <div class="mt-8 flex flex-col sm:flex-row items-center justify-between gap-4 border-t pt-6">
            <div class="text-sm text-muted-foreground">
                显示 第 <span class="font-medium">{(currentPage - 1) * pageSize + 1}</span> 到 <span class="font-medium">{Math.min(currentPage * pageSize, totalItems)}</span> 条，共 <span class="font-medium">{totalItems}</span> 条角色
            </div>
            
            <div class="flex items-center gap-2">
                 <Button
                    variant="outline"
                    size="sm"
                    disabled={currentPage <= 1}
                    onclick={() => {
                        currentPage--;
                        fetchCards();
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
                                fetchCards();
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
                                fetchCards();
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
                        fetchCards();
                    }}
                >
                    下一页 <ChevronRight class="h-4 w-4 ml-1" />
                </Button>
            </div>
        </div>
    {/if}

    <!-- Delete Confirmation Dialog -->
    <AlertDialog.Root bind:open={deleteDialogOpen}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>确定要删除吗？</AlertDialog.Title>
                <AlertDialog.Description>
                    此操作将把 {isBatchDeleteArgs
                        ? `选中的 ${selectedCardIds.size} 个`
                        : "该"}角色卡移至回收站，你可以随时在回收站中恢复。
                </AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel>取消</AlertDialog.Cancel>
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
