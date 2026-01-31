<script lang="ts">
    // 小剧场列表页面
    import { onMount } from "svelte";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Badge } from "$lib/components/ui/badge";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { 
        Plus, 
        Upload, 
        Download, 
        Sparkles, 
        Search, 
        ChevronLeft, 
        ChevronRight,
        Drama,
        X
    } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";
    import { API_BASE } from "$lib/api";
    import TheaterCard from "$lib/components/theater/TheaterCard.svelte";
    import CreateTheaterDialog from "$lib/components/theater/CreateTheaterDialog.svelte";
    import ImportTheaterDialog from "$lib/components/theater/ImportTheaterDialog.svelte";

    // 状态
    let theaters: any[] = $state([]);
    let categories: string[] = $state([]);
    let loading = $state(true);
    
    // 分页
    let currentPage = $state(1);
    let pageSize = $state(50);
    let totalItems = $state(0);
    let totalPages = $state(0);
    let jumpToPage = $state(1);
    
    // 筛选
    let searchQuery = $state("");
    let selectedCategory: string | null = $state(null);
    
    // 多选导出模式
    let isSelectionMode = $state(false);
    let selectedIds = $state(new Set<string>());
    
    // 弹窗
    let createDialogOpen = $state(false);
    let importDialogOpen = $state(false);
    let editingTheater: any = $state(null);
    let deleteDialogOpen = $state(false);
    let theaterToDelete: string | null = $state(null);

    onMount(async () => {
        breadcrumbs.set([{ label: "小剧场", href: "/theaters" }]);
        await Promise.all([loadTheaters(), loadCategories()]);
    });

    async function loadTheaters() {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const params = new URLSearchParams({
                page: currentPage.toString(),
                page_size: pageSize.toString(),
            });
            
            if (searchQuery) params.set("search", searchQuery);
            if (selectedCategory) params.set("category", selectedCategory);

            const res = await fetch(`${API_BASE}/api/theaters?${params}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            
            if (!res.ok) throw new Error("加载失败");
            
            const data = await res.json();
            theaters = data.items || [];
            totalItems = data.total || 0;
            totalPages = data.total_pages || 1;
            jumpToPage = currentPage;
        } catch (e) {
            console.error(e);
            toast.error("加载小剧场失败");
        } finally {
            loading = false;
        }
    }

    async function loadCategories() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/theaters/categories`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                categories = await res.json();
            }
        } catch (e) {
            console.error(e);
        }
    }

    function selectCategory(cat: string | null) {
        selectedCategory = cat;
        currentPage = 1;
        loadTheaters();
    }

    let searchTimeout: any;
    function onSearchInput() {
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(() => {
            currentPage = 1;
            loadTheaters();
        }, 300);
    }

    function toggleSelection(id: string) {
        if (selectedIds.has(id)) {
            selectedIds.delete(id);
        } else {
            selectedIds.add(id);
        }
        selectedIds = new Set(selectedIds);
    }

    function startExportMode() {
        isSelectionMode = true;
        selectedIds = new Set();
    }

    function cancelExportMode() {
        isSelectionMode = false;
        selectedIds = new Set();
    }

    async function handleExport(exportAll: boolean = false) {
        try {
            const token = localStorage.getItem("auth_token");
            let url = `${API_BASE}/api/theaters/export`;
            
            if (!exportAll && selectedIds.size > 0) {
                url += `?ids=${Array.from(selectedIds).join(',')}`;
            }

            const res = await fetch(url, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            
            if (!res.ok) throw new Error("导出失败");
            
            const blob = await res.blob();
            const downloadUrl = window.URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = downloadUrl;
            a.download = `theaters_${new Date().toISOString().slice(0, 10)}.txt`;
            document.body.appendChild(a);
            a.click();
            window.URL.revokeObjectURL(downloadUrl);
            document.body.removeChild(a);
            
            toast.success("导出成功");
            cancelExportMode();
        } catch (e) {
            console.error(e);
            toast.error("导出失败");
        }
    }

    function handleEdit(theater: any) {
        editingTheater = theater;
        createDialogOpen = true;
    }

    function handleDelete(id: string) {
        theaterToDelete = id;
        deleteDialogOpen = true;
    }

    async function confirmDelete() {
        if (!theaterToDelete) return;
        
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/theaters/${theaterToDelete}`, {
                method: "DELETE",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            
            if (!res.ok) throw new Error("删除失败");
            
            toast.success("删除成功");
            await loadTheaters();
        } catch (e) {
            console.error(e);
            toast.error("删除失败");
        } finally {
            theaterToDelete = null;
            deleteDialogOpen = false;
        }
    }

    function handleSuccess() {
        loadTheaters();
        loadCategories();
        editingTheater = null;
    }

    function handleCreateNew() {
        editingTheater = null;
        createDialogOpen = true;
    }
</script>

<div class="container mx-auto py-6 space-y-6 max-w-7xl">
    <!-- 页面头部 -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div class="space-y-1">
            <h1 class="text-2xl font-bold tracking-tight">小剧场</h1>
            <p class="text-muted-foreground">管理 {totalItems} 个小剧场提示词</p>
        </div>
        
        <div class="flex items-center gap-2 flex-wrap">
            {#if isSelectionMode}
                <span class="text-sm text-muted-foreground">已选 {selectedIds.size} 项</span>
                <Button size="sm" onclick={() => handleExport(false)} disabled={selectedIds.size === 0}>
                    导出选中
                </Button>
                <Button size="sm" variant="outline" onclick={() => handleExport(true)}>
                    导出全部
                </Button>
                <Button size="sm" variant="ghost" onclick={cancelExportMode}>
                    <X class="h-4 w-4" />
                </Button>
            {:else}
                <Button variant="outline" onclick={() => importDialogOpen = true} class="gap-2">
                    <Upload class="h-4 w-4" />
                    导入
                </Button>
                <Button variant="outline" onclick={startExportMode} class="gap-2">
                    <Download class="h-4 w-4" />
                    导出
                </Button>
                <!-- <Button variant="outline" disabled class="gap-2">
                    <Sparkles class="h-4 w-4" />
                    AI生成
                </Button> -->
                <Button onclick={handleCreateNew} class="gap-2">
                    <Plus class="h-4 w-4" />
                    新建小剧场
                </Button>
            {/if}
        </div>
    </div>

    <!-- 搜索栏 -->
    <div class="relative">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input
            placeholder="搜索标题、简介、内容..."
            class="pl-10"
            bind:value={searchQuery}
            oninput={onSearchInput}
        />
    </div>

    <!-- 分类栏 -->
    <div class="flex items-center gap-2 overflow-x-auto pb-2 scrollbar-hide">
        <button
            class={cn(
                "px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-colors",
                selectedCategory === null
                    ? "bg-primary text-primary-foreground"
                    : "hover:bg-muted"
            )}
            onclick={() => selectCategory(null)}
        >
            全部
        </button>
        {#each categories as cat}
            <button
                class={cn(
                    "px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-colors",
                    selectedCategory === cat
                        ? "bg-primary text-primary-foreground"
                        : "hover:bg-muted"
                )}
                onclick={() => selectCategory(cat)}
            >
                {cat}
            </button>
        {/each}
    </div>

    <!-- 内容区域 -->
    {#if loading}
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {#each Array(8) as _}
                <div class="h-32 rounded-xl bg-muted/50 animate-pulse"></div>
            {/each}
        </div>
    {:else if theaters.length > 0}
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {#each theaters as theater (theater.id)}
                <TheaterCard
                    {theater}
                    isSelected={selectedIds.has(theater.id)}
                    {isSelectionMode}
                    onEdit={handleEdit}
                    onDelete={handleDelete}
                    onSelect={toggleSelection}
                />
            {/each}
        </div>
    {:else}
        <div class="text-center py-20">
            <div class="mx-auto w-16 h-16 rounded-full bg-muted/50 flex items-center justify-center mb-4">
                <Drama class="h-8 w-8 text-muted-foreground" />
            </div>
            <h3 class="text-lg font-medium">暂无小剧场</h3>
            <p class="text-muted-foreground mt-1">
                点击上方"新建小剧场"或"导入"添加内容
            </p>
        </div>
    {/if}

    <!-- 分页控件 -->
    {#if totalPages > 1}
        <div class="mt-8 flex flex-col sm:flex-row items-center justify-between gap-4 border-t pt-6">
            <!-- Summary Text -->
            <div class="text-sm text-muted-foreground order-2 sm:order-1 text-center sm:text-left">
                显示 第 <span class="font-medium">{(currentPage - 1) * pageSize + 1}</span> 到 
                <span class="font-medium">{Math.min(currentPage * pageSize, totalItems)}</span> 条，
                共 <span class="font-medium">{totalItems}</span> 条
            </div>
            
            <!-- Controls -->
            <div class="flex flex-col sm:flex-row items-center gap-4 sm:gap-2 order-1 sm:order-2 w-full sm:w-auto">
                <div class="flex items-center justify-between w-full sm:w-auto gap-2">
                    <Button
                        variant="outline"
                        size="sm"
                        disabled={currentPage <= 1}
                        onclick={() => { currentPage--; loadTheaters(); }}
                         class="flex-1 sm:flex-none"
                    >
                        <ChevronLeft class="h-4 w-4 mr-1" /> <span class="sm:inline">上一页</span>
                    </Button>
                    
                     <!-- Mobile Page Indicator -->
                    <div class="sm:hidden text-sm font-medium">
                        {currentPage} / {totalPages}
                    </div>

                    <Button
                        variant="outline"
                        size="sm"
                        disabled={currentPage >= totalPages}
                        onclick={() => { currentPage++; loadTheaters(); }}
                         class="flex-1 sm:flex-none"
                    >
                        <span class="sm:inline">下一页</span> <ChevronRight class="h-4 w-4 ml-1" />
                    </Button>
                </div>

                 <!-- Desktop Jump Controls -->
                <div class="hidden sm:flex items-center gap-2 mx-2">
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
                                loadTheaters();
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
                            loadTheaters();
                        }}
                    >
                        跳转
                    </Button>
                </div>
            </div>
        </div>
    {/if}
</div>

<!-- 创建/编辑弹窗 -->
<CreateTheaterDialog 
    bind:open={createDialogOpen} 
    theater={editingTheater}
    {categories}
    onSuccess={handleSuccess}
/>

<!-- 导入弹窗 -->
<ImportTheaterDialog 
    bind:open={importDialogOpen} 
    onSuccess={handleSuccess}
/>

<!-- 删除确认弹窗 -->
<AlertDialog.Root bind:open={deleteDialogOpen}>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>确定要删除吗？</AlertDialog.Title>
            <AlertDialog.Description>
                此操作将永久删除该小剧场，且无法恢复。
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
            <AlertDialog.Cancel onclick={() => theaterToDelete = null}>取消</AlertDialog.Cancel>
            <AlertDialog.Action
                class="bg-destructive !text-destructive-foreground hover:bg-destructive/90"
                onclick={confirmDelete}
            >
                确认删除
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>
