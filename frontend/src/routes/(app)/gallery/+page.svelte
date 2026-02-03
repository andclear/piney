<script lang="ts">
    import { onMount } from "svelte";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";
    import { Skeleton } from "$lib/components/ui/skeleton";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Badge } from "$lib/components/ui/badge";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";

    import { API_BASE, resolveUrl } from "$lib/api";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
    import {
        Upload,
        Search,
        Plus,
        Trash2,
        Edit2,
        Heart,
        Download,
        Filter,
        Check,
        X,
        ChevronDown,
        GripVertical,
        Sparkles,
        Copy,
        CheckSquare,
    } from "lucide-svelte";
    import { Label } from "$lib/components/ui/label";
    import { dndzone, TRIGGERS } from "svelte-dnd-action";
    import { flip } from "svelte/animate";
    import { longpress } from "$lib/actions/longpress";

    // ============ 类型定义 ============
    interface Category {
        id: string;
        name: string;
        sort_order: number;
    }

    interface ImageItem {
        id: string;
        title: string;
        thumbnail_path: string;
        width: number;
        height: number;
        is_favorite: boolean;
        is_ai: boolean;
        is_authorized: boolean;
        color_category: string | null;
        created_at: string;
    }

    interface ImageDetail {
        id: string;
        title: string;
        category_id: string | null;
        tags: string[];
        file_path: string;
        thumbnail_path: string;
        width: number;
        height: number;
        file_size: number;
        color_category: string | null;
        is_ai: boolean;
        ai_platform: string | null;
        ai_prompt: string | null;
        ai_negative_prompt: string | null;
        is_authorized: boolean;
        is_favorite: boolean;
        user_notes: string | null;
        created_at: string;
    }

    // ============ 颜色常量 ============
    const COLOR_OPTIONS = [
        { value: "red", label: "红色", color: "#FF0000" },
        { value: "orange", label: "橙色", color: "#FFA500" },
        { value: "yellow", label: "黄色", color: "#FFFF00" },
        { value: "green", label: "绿色", color: "#008000" },
        { value: "cyan", label: "青色", color: "#00FFFF" },
        { value: "blue", label: "蓝色", color: "#0000FF" },
        { value: "purple", label: "紫色", color: "#800080" },
        { value: "black", label: "黑色", color: "#000000" },
        { value: "white", label: "白色", color: "#FFFFFF" },
        { value: "gray", label: "灰色", color: "#808080" },
    ];

    // ============ 状态 ============
    let searchQuery = $state("");
    let selectedCategoryId: string | null = $state(null);
    let selectedColor: string | null = $state(null);
    let filterIsAuthorized: boolean | null = $state(null);
    let filterIsFavorite: boolean = $state(false);


    let categories: Category[] = $state([]);
    let images: ImageItem[] = $state([]);

    let loading = $state(true);
    let categoryDialogOpen = $state(false);
    let newCategoryName = $state("");
    let editingCategory: Category | null = $state(null);

    // 分页 & 无限滚动
    let currentPage = $state(1);
    let pageSize = $state(50);
    let totalItems = $state(0);
    let totalPages = $state(0);
    let hasMore = $state(true);
    let isLoadingMore = $state(false);
    
    // 缓存: key = filter组合, value = 已加载的图片和分页信息
    let imageCache = new Map<string, { items: ImageItem[], total: number, loadedPages: number }>();
    
    function getCacheKey(): string {
        return `${selectedCategoryId || 'all'}_${searchQuery}_${selectedColor || 'all'}_${filterIsFavorite}`;
    }

    // 编辑对话框
    let editDialogOpen = $state(false);
    let editingImage: ImageDetail | null = $state(null);

    // 拖拽状态 (svelte-dnd-action)
    const FLIP_DURATION_MS = 200;
    const TOUCH_DELAY_MS = 300;
    let dndCategories: Category[] = $state([]);
    let isCategoryDragging = $state(false);
    let categoryOrderBeforeDrag: string[] = [];

    // Prevent click after longpress
    let isLongPressTriggered = false;

    // 批量操作状态
    let isSelectionMode = $state(false);
    let selectedImageIds = $state(new Set<string>());
    let deleteDialogOpen = $state(false);
    let imageToDelete = $state<string | null>(null);
    let isBatchDeleteArgs = $state(false);
    let moveDialogOpen = $state(false);
    let targetCategoryId: string | null = $state(null);
    let isExporting = $state(false);


    // ============ API 调用 ============
    async function fetchCategories() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/image-categories`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                categories = await res.json();
            }
        } catch (e) {
            console.error(e);
            toast.error("获取分类失败");
        }
    }

    // ============ 瀑布流布局逻辑 ============
    let colCount = $state(4);
    let windowWidth = $state(0);

    // 监听窗口大小变化
    $effect(() => {
        if (windowWidth < 768) colCount = 2;
        else if (windowWidth < 1024) colCount = 3;
        else if (windowWidth < 1280) colCount = 4;
        else colCount = 5;
    });

    // 计算分列数据 (水平优先)
    let columns = $derived.by(() => {
        const cols: ImageItem[][] = Array.from({ length: colCount }, () => []);
        images.forEach((img, i) => {
            cols[i % colCount].push(img);
        });
        return cols;
    });


    async function fetchImages(append: boolean = false) {
        if (isLoadingMore) return;
        
        const cacheKey = getCacheKey();
        
        // 如果不是追加模式，检查缓存
        if (!append && imageCache.has(cacheKey)) {
            const cached = imageCache.get(cacheKey)!;
            images = cached.items;
            totalItems = cached.total;
            currentPage = cached.loadedPages;
            hasMore = cached.items.length < cached.total;
            return;
        }
        
        try {
            isLoadingMore = true;
            const token = localStorage.getItem("auth_token");
            let url = `${API_BASE}/api/images`;
            const params = new URLSearchParams();
            
            if (selectedCategoryId) params.set("category_id", selectedCategoryId);
            if (searchQuery) params.set("search", searchQuery);
            if (selectedColor) params.set("color_category", selectedColor);

            if (filterIsAuthorized !== null) params.set("is_authorized", String(filterIsAuthorized));
            if (filterIsFavorite) params.set("is_favorite", "true");
            
            params.set("page", currentPage.toString());
            params.set("page_size", pageSize.toString());

            if (params.toString()) url += `?${params.toString()}`;

            const res = await fetch(url, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                const data = await res.json();
                const newItems = data.items || [];
                
                if (append) {
                    images = [...images, ...newItems];
                } else {
                    images = newItems;
                }
                
                totalItems = data.total || 0;
                totalPages = data.total_pages || 1;
                hasMore = currentPage < totalPages;
                
                // 更新缓存
                imageCache.set(cacheKey, {
                    items: images,
                    total: totalItems,
                    loadedPages: currentPage
                });
            }
        } catch (e) {
            console.error("获取图片失败", e);
        } finally {
            isLoadingMore = false;
        }
    }
    
    // 加载更多
    async function loadMore() {
        if (!hasMore || isLoadingMore) return;
        currentPage++;
        await fetchImages(true);
    }
    
    // 重置并重新加载 (筛选条件变化时)
    function resetAndFetch() {
        currentPage = 1;
        images = [];
        hasMore = true;
        fetchImages(false);
    }

    async function createCategory() {
        if (!newCategoryName.trim()) {
            toast.error("分类名称不能为空");
            return;
        }
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/image-categories`, {
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
            }
        } catch (e) {
            toast.error("创建分类失败");
        }
    }

    async function updateCategory(id: string, name: string) {
        try {
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/image-categories/${id}`, {
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
        if (!confirm("确认删除该分类？图片将移至「未分类」")) return;
        try {
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/image-categories/${id}`, {
                method: "DELETE",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            await fetchCategories();
            imageCache.clear();
            await fetchImages();
            toast.success("分类已删除");
        } catch (e) {
            toast.error("删除分类失败");
        }
    }

    async function toggleFavorite(img: ImageItem) {
        try {
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/images/${img.id}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ is_favorite: !img.is_favorite }),
            });
            
            img.is_favorite = !img.is_favorite;

            // 重新排序：收藏 > 时间
            images = images.slice().sort((a, b) => {
                if (a.is_favorite !== b.is_favorite) {
                    return a.is_favorite ? -1 : 1;
                }
                return b.created_at.localeCompare(a.created_at);
            });
        } catch (e) {
            toast.error("操作失败");
        }
    }

    async function openEditDialog(id: string) {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/images/${id}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                editingImage = await res.json();
                editDialogOpen = true;
            }
        } catch (e) {
            toast.error("获取图片详情失败");
        }
    }

    async function saveImageChanges() {
        if (!editingImage) return;
        try {
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/images/${editingImage.id}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({
                    title: editingImage.title,
                    category_id: editingImage.category_id,
                    tags: editingImage.tags,
                    is_ai: editingImage.is_ai,
                    ai_platform: editingImage.ai_platform,
                    ai_prompt: editingImage.ai_prompt,
                    ai_negative_prompt: editingImage.ai_negative_prompt,
                    is_authorized: editingImage.is_authorized,
                    user_notes: editingImage.user_notes,
                }),
            });
            
            // 立即更新本地状态，让 UI 即时反映变化
            const editedId = editingImage.id;
            const updatedFields = {
                title: editingImage.title,
                category_id: editingImage.category_id,
                tags: editingImage.tags,
                is_ai: editingImage.is_ai,
                ai_platform: editingImage.ai_platform,
                ai_prompt: editingImage.ai_prompt,
                ai_negative_prompt: editingImage.ai_negative_prompt,
                is_authorized: editingImage.is_authorized,
                user_notes: editingImage.user_notes,
            };
            images = images.map(img => 
                img.id === editedId ? { ...img, ...updatedFields } : img
            );
            
            // 清除缓存，下次切换筛选条件时重新加载
            imageCache.clear();
            
            toast.success("保存成功");
            editDialogOpen = false;
        } catch (e) {
            toast.error("保存失败");
        }
    }

    function deleteImage(id: string) {
        imageToDelete = id;
        isBatchDeleteArgs = false;
        deleteDialogOpen = true;
    }

    async function confirmDelete() {
        if (!imageToDelete) return;
        try {
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/images/${imageToDelete}`, {
                method: "DELETE",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            toast.success("删除成功");
            imageCache.clear();
            await fetchImages();
        } catch (e) {
            toast.error("删除失败");
        }
        deleteDialogOpen = false;
        imageToDelete = null;
    }

    function startMultiselect(id: string) {
        if (!isSelectionMode) {
            isSelectionMode = true;
            selectedImageIds = new Set([id]);
        } else {
            toggleImageSelection(id);
        }
    }

    async function handleBatchDelete() {
        if (selectedImageIds.size === 0) return;
        // 弹窗确认逻辑移交给 UI 中的 AlertDialog
        deleteDialogOpen = true;
        isBatchDeleteArgs = true;
    }

    async function confirmBatchDelete() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/images/batch/delete`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ ids: Array.from(selectedImageIds) }),
            });
            if (res.ok) {
                toast.success("已批量删除");
                selectedImageIds = new Set();
                isSelectionMode = false;
                imageCache.clear();
                fetchImages();
            } else {
                toast.error("批量删除失败");
            }
        } catch (e) {
            console.error(e);
            toast.error("网络错误");
        }
        deleteDialogOpen = false;
        isBatchDeleteArgs = false;
    }

    // 批量移动
    async function handleBatchMove() {
        if (selectedImageIds.size === 0) return;
        moveDialogOpen = true;
        targetCategoryId = null; // Reset
    }

    async function confirmBatchMove() {
        if (targetCategoryId === null) return;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/images/batch/category`, {
                method: "PUT",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({
                    ids: Array.from(selectedImageIds),
                    category_id: targetCategoryId === "null" ? null : targetCategoryId,
                }),
            });
            if (res.ok) {
                toast.success("已移动分类");
                moveDialogOpen = false;
                selectedImageIds = new Set();
                isSelectionMode = false;
                imageCache.clear();
                fetchImages();
            } else {
                toast.error("移动分类失败");
            }
        } catch (e) {
            console.error(e);
            toast.error("网络错误");
        }
    }

    // 批量导出
    async function handleBatchExport() {
        if (selectedImageIds.size === 0) return;
        
        // 单个导出直接下载原图
        if (selectedImageIds.size === 1) {
            const id = Array.from(selectedImageIds)[0];
            handleExport(id);
            return;
        }

        try {
            isExporting = true;
            toast.info("正在打包下载...");
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/images/batch/export`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                     ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ ids: Array.from(selectedImageIds) }),
            });
            if (res.ok) {
                const blob = await res.blob();
                const url = window.URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = `images_export_${new Date().getTime()}.zip`;
                document.body.appendChild(a);
                a.click();
                window.URL.revokeObjectURL(url);
                document.body.removeChild(a);
                toast.success("导出成功", {
                    description: "文件已保存到浏览器默认下载位置",
                    duration: 4000,
                });
            } else {
                const err = await res.json();
                toast.error(`导出失败: ${err.error}`);
            }
        } catch (e) {
            console.error(e);
            toast.error("网络错误");
        } finally {
            isExporting = false;
        }
    }

    // 批量授权
    async function handleBatchAuthorized() {
        if (selectedImageIds.size === 0) return;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/images/batch/update`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({
                     ids: Array.from(selectedImageIds),
                     is_authorized: true 
                }),
            });
            if (res.ok) {
                toast.success("已批量设置已获许可");
                imageCache.clear();
                fetchImages(); 
                // 保持选择状态以便用户确认
            } else {
                toast.error("设置失败");
            }
        } catch (e) {
             console.error(e); 
             toast.error("网络错误");
        }
    }

    async function handleExport(id: string) {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/images/${id}/export`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (!res.ok) throw new Error("导出失败");
            
            const blob = await res.blob();
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            
            // 优先使用本地缓存的标题 (避免 WebView/Windows Header 解析问题)
            const img = images.find(i => i.id === id);
            let filename = img ? `${img.title}.png` : "image.png";

            const contentDisposition = res.headers.get("content-disposition");
            if (contentDisposition) {
                const utf8Match = contentDisposition.match(/filename\*=UTF-8''(.+)/);
                if (utf8Match) {
                    filename = decodeURIComponent(utf8Match[1]);
                } else {
                    const filenameMatch = contentDisposition.match(/filename="(.+)"/);
                    if (filenameMatch) {
                        filename = filenameMatch[1];
                    }
                }
            }
            
            // 如果 Header 没解析出带扩展名的文件名，且本地有标题，确保扩展名正确
            if (img && !filename.includes('.')) {
                // Determine extension from content-type or default to png
                const contentType = res.headers.get("content-type") || "";
                let ext = "png";
                if (contentType.includes("jpeg")) ext = "jpg";
                else if (contentType.includes("webp")) ext = "webp";
                else if (contentType.includes("gif")) ext = "gif";
                
                filename = `${img.title}.${ext}`;
            }
            a.download = filename;
            
            document.body.appendChild(a);
            a.click();
            window.URL.revokeObjectURL(url);
            document.body.removeChild(a);
            toast.success("导出成功", {
                description: "文件已保存到浏览器下载文件夹",
                duration: 3000,
            });
        } catch (e) {
            toast.error("导出失败");
        }
    }

    // ============ 导入 ============
    let fileInput: HTMLInputElement;
    async function handleImport(event: Event) {
        const input = event.target as HTMLInputElement;
        const files = input.files;
        if (!files || files.length === 0) return;

        const formData = new FormData();
        for (const file of files) {
            formData.append("files", file);
        }

        try {
            toast.info(`正在导入 ${files.length} 张图片...`);
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/images`, {
                method: "POST",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
                body: formData,
            });
            if (res.ok) {
                const data = await res.json();
                toast.success(`成功导入 ${data.imported} 张图片`);
                imageCache.clear();
                currentPage = 1;
                await fetchImages();
            } else {
                const error = await res.json();
                toast.error(error.error || "导入失败");
            }
        } catch (e) {
            toast.error("导入失败");
        }
        input.value = "";
    }

    // ============ 拖拽排序 (svelte-dnd-action) ============
    function handleCategoryDndConsider(e: CustomEvent<{ items: Category[], info: { trigger: string } }>) {
        if (e.detail.info.trigger === TRIGGERS.DRAG_STARTED) {
            categoryOrderBeforeDrag = categories.map(c => c.id);
            isCategoryDragging = true;
        }
        dndCategories = e.detail.items;
    }

    async function handleCategoryDndFinalize(e: CustomEvent<{ items: Category[], info: { trigger: string } }>) {
        isCategoryDragging = false;
        const newOrder = e.detail.items.map(c => c.id);
        const orderChanged = categoryOrderBeforeDrag.length > 0 && 
            (categoryOrderBeforeDrag.length !== newOrder.length || 
             categoryOrderBeforeDrag.some((id, i) => id !== newOrder[i]));
        
        if (orderChanged) {
            categories = e.detail.items.map(item => {
                const { isDndShadowItem, ...cleanItem } = item as any;
                return cleanItem;
            });
            
            // 保存排序
            const token = localStorage.getItem("auth_token");
            await fetch(`${API_BASE}/api/image-categories/reorder`, {
                method: "PUT",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ ids: newOrder }),
            });
        }
        
        dndCategories = [];
        categoryOrderBeforeDrag = [];
    }

    // 用于显示的分类列表
    let displayCategories = $derived(isCategoryDragging ? dndCategories : categories);

    // ============ 工具函数 ============
    function selectCategory(id: string | null) {
        selectedCategoryId = id;
        filterIsFavorite = false; // 切换分类时清除收藏筛选
        resetAndFetch();
    }

    function selectFavorites() {
        filterIsFavorite = true;
        selectedCategoryId = null; // 切换到收藏时清除分类选择
        resetAndFetch();
    }

    function toggleImageSelection(id: string) {
        if (selectedImageIds.has(id)) {
            selectedImageIds.delete(id);
        } else {
            selectedImageIds.add(id);
        }
        selectedImageIds = new Set(selectedImageIds);
    }

    function formatDate(dateStr: string) {
        if (!dateStr) return '';
        const d = new Date(dateStr);
        if (isNaN(d.getTime())) return dateStr;
        return d.toLocaleString('zh-CN', {
            year: 'numeric', 
            month: '2-digit', 
            day: '2-digit', 
            hour: '2-digit', 
            minute: '2-digit',
            hour12: false 
        });
    }

    function formatFileSize(bytes: number) {
        if (bytes < 1024) return bytes + ' B';
        if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
        return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
    }

    let searchTimeout: any;
    function onSearchInput() {
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(() => {
            resetAndFetch();
        }, 300);
    }

    // ============ 无限滚动 ============
    let sentinelEl = $state<HTMLDivElement>();
    let observer: IntersectionObserver | null = null;
    
    $effect(() => {
        // 当 sentinelEl 存在时设置观察者
        if (sentinelEl && !observer) {
            observer = new IntersectionObserver(
                (entries) => {
                    if (entries[0].isIntersecting && hasMore && !isLoadingMore) {
                        loadMore();
                    }
                },
                { rootMargin: '200px' }
            );
            observer.observe(sentinelEl);
        }
        
        return () => {
            if (observer) {
                observer.disconnect();
                observer = null;
            }
        };
    });

    // ============ 生命周期 ============
    onMount(async () => {
        breadcrumbs.set([{ label: "图库" }]);
        await Promise.all([fetchCategories(), fetchImages()]);
        loading = false;
    });
</script>

<svelte:window bind:innerWidth={windowWidth} />

<div class="container py-6 space-y-6 max-w-7xl mx-auto">
    <!-- 页面标题 -->
    <div class="flex items-center justify-between">
        <div class="space-y-1">
            <h1 class="text-2xl font-bold tracking-tight">图库</h1>
            <p class="text-muted-foreground">
                管理 {totalItems} 张图片
            </p>
        </div>
        <div class="flex gap-2">
            <input
                type="file"
                accept="image/png,image/jpeg,image/webp,image/gif"
                multiple
                class="hidden"
                bind:this={fileInput}
                onchange={handleImport}
            />
            <Button class="gap-2" onclick={() => fileInput.click()}>
                <Upload class="h-4 w-4" />
                导入图片
            </Button>
        </div>
    </div>

    <!-- 搜索栏和筛选 -->
    <div class="flex items-center gap-3 flex-wrap">
        <div class="relative flex-1 min-w-[200px]">
            <Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
                placeholder="搜索图片标题、标签、分类、正向提示词..."
                class="pl-10"
                bind:value={searchQuery}
                oninput={onSearchInput}
            />
        </div>

        <!-- 颜色筛选 -->
        <DropdownMenu.Root>
            <DropdownMenu.Trigger>
                <Button variant="outline" class="gap-2">
                    {#if selectedColor}
                        <span 
                            class="w-3 h-3 rounded-full border"
                            style="background-color: {COLOR_OPTIONS.find(c => c.value === selectedColor)?.color}"
                        ></span>
                        {COLOR_OPTIONS.find(c => c.value === selectedColor)?.label}
                    {:else}
                        <Filter class="h-4 w-4" />
                        颜色
                    {/if}
                    <ChevronDown class="h-4 w-4" />
                </Button>
            </DropdownMenu.Trigger>
            <DropdownMenu.Content>
                <DropdownMenu.Item onclick={() => { selectedColor = null; fetchImages(); }}>
                    全部颜色
                </DropdownMenu.Item>
                <DropdownMenu.Separator />
                {#each COLOR_OPTIONS as color}
                    <DropdownMenu.Item onclick={() => { selectedColor = color.value; fetchImages(); }}>
                        <span 
                            class="w-3 h-3 rounded-full border mr-2"
                            style="background-color: {color.color}"
                        ></span>
                        {color.label}
                    </DropdownMenu.Item>
                {/each}
            </DropdownMenu.Content>
        </DropdownMenu.Root>
    </div>

    <!-- 分类栏 -->
    <div class="flex items-center gap-2 overflow-x-auto pb-2">
        <!-- 收藏按钮 (固定在最前) -->
        <button
            class={cn(
                "px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-colors flex items-center gap-1.5",
                filterIsFavorite
                    ? "bg-red-500 text-white"
                    : "hover:bg-muted",
            )}
            onclick={selectFavorites}
        >
            <Heart class="h-4 w-4" fill={filterIsFavorite ? "currentColor" : "none"} />
            收藏
        </button>

        <div class="w-[1px] h-6 bg-border"></div>

        <button
            class={cn(
                "px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-colors",
                selectedCategoryId === null && !filterIsFavorite
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
                            onkeydown={(e) => e.key === "Enter" && createCategory()}
                        />
                        <Button onclick={createCategory}>添加</Button>
                    </div>

                    <!-- 分类列表 -->
                    <div 
                        class="space-y-2 max-h-60 overflow-y-auto"
                        use:dndzone={{
                            items: displayCategories,
                            flipDurationMs: FLIP_DURATION_MS,
                            delayTouchStart: TOUCH_DELAY_MS,
                            dropTargetStyle: {},
                            type: 'gallery-categories'
                        }}
                        onconsider={handleCategoryDndConsider}
                        onfinalize={handleCategoryDndFinalize}
                    >
                        {#each displayCategories as category (category.id)}
                            <div
                                animate:flip={{ duration: FLIP_DURATION_MS }}
                                class="flex items-center gap-2 p-2 rounded-lg border bg-background group transition-colors"
                            >
                                <GripVertical class="h-4 w-4 text-muted-foreground cursor-grab" />
                                {#if editingCategory?.id === category.id}
                                    <Input
                                        class="flex-1 h-8"
                                        value={editingCategory.name}
                                        oninput={(e) => {
                                            if (editingCategory) {
                                                editingCategory.name = e.currentTarget.value;
                                            }
                                        }}
                                        onkeydown={(e) => {
                                            if (e.key === "Enter" && editingCategory) {
                                                updateCategory(editingCategory.id, editingCategory.name);
                                            }
                                        }}
                                    />
                                    <Button size="sm" variant="ghost" onclick={() => updateCategory(editingCategory!.id, editingCategory!.name)}>
                                        <Check class="h-4 w-4" />
                                    </Button>
                                {:else}
                                    <span class="flex-1">{category.name}</span>
                                    <Button size="sm" variant="ghost" class="opacity-0 group-hover:opacity-100" onclick={() => editingCategory = { ...category }}>
                                        <Edit2 class="h-4 w-4" />
                                    </Button>
                                    <Button size="sm" variant="ghost" class="opacity-0 group-hover:opacity-100 text-destructive" onclick={() => deleteCategory(category.id)}>
                                        <Trash2 class="h-4 w-4" />
                                    </Button>
                                {/if}
                            </div>
                        {/each}
                    </div>
                </div>
            </Dialog.Content>
        </Dialog.Root>
    </div>

    <!-- 瀑布流图片 -->
    {#if loading}
        <div class="columns-2 md:columns-3 lg:columns-4 xl:columns-5 gap-4">
            {#each Array(12) as _}
                <Skeleton class="w-full h-48 mb-4 rounded-lg" />
            {/each}
        </div>
    {:else if images.length === 0}
        <div class="text-center py-20 text-muted-foreground">
            <p>暂无图片</p>
        </div>
    {:else}
        <!-- 瀑布流布局 -->
        <div class="flex gap-4 items-start">

            {#each columns as column}
                <div class="flex-1 flex flex-col gap-4 min-w-0">
                    {#each column as image (image.id)}
                        <ContextMenu.Root>
                            <ContextMenu.Trigger>
                                <div
                                    class={cn(
                                        "relative rounded-lg overflow-hidden cursor-pointer group break-inside-avoid select-none touch-callout-none",
                                        "border border-transparent hover:border-primary/50 transition-all",
                                        isSelectionMode && selectedImageIds.has(image.id) && "ring-2 ring-primary"
                                    )}
                                    style="-webkit-touch-callout: none;"
                                    role="button"
                                    tabindex="0"

                                    use:longpress
                                    onlongpress={(e) => {
                                        isLongPressTriggered = true;
                                        setTimeout(() => isLongPressTriggered = false, 1000); // Safety reset
                                        
                                        const original = e.detail.originalEvent;
                                        const touch = original.touches?.[0] || original;
                                        e.target?.dispatchEvent(
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
                                    onkeydown={(e) => {
                                        if (e.key === 'Enter' || e.key === ' ') {
                                            e.preventDefault();
                                            if (isSelectionMode) {
                                                toggleImageSelection(image.id);
                                            } else {
                                                openEditDialog(image.id);
                                            }
                                        }
                                    }}
                                    onclick={(e) => {
                                        if (isLongPressTriggered) {
                                            isLongPressTriggered = false;
                                            e.stopPropagation();
                                            return;
                                        }

                                        if (isSelectionMode) {
                                            toggleImageSelection(image.id);
                                        } else {
                                            openEditDialog(image.id);
                                        }
                                    }}
                                >
                                    <img
                                        src={resolveUrl(image.thumbnail_path)}
                                        alt={image.title}
                                        class="w-full object-cover pointer-events-none select-none"
                                        loading="lazy"
                                        decoding="async"
                                    />
                                    
                                    <!-- 悬浮操作 -->
                                    <div class="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-end p-2">
                                        <div class="flex-1 truncate text-white text-sm font-medium">
                                            {image.title}
                                        </div>
                                    </div>

                                    <!-- 收藏按钮 -->
                                    <button
                                        class={cn(
                                            "absolute top-2 right-2 p-1.5 rounded-full transition-all",
                                            image.is_favorite 
                                                ? "text-red-500 drop-shadow-md hover:scale-110" 
                                                : "bg-black/30 text-white opacity-0 group-hover:opacity-100"
                                        )}
                                        onclick={(e) => { e.stopPropagation(); toggleFavorite(image); }}
                                    >
                                        <Heart class="h-4 w-4" fill={image.is_favorite ? "currentColor" : "none"} />
                                    </button>

                                    <!-- 标记 (AI / 已授权) -->
                                    {#if !isSelectionMode && (image.is_ai || image.is_authorized)}
                                        <div class="absolute top-2 left-2 flex gap-1">
                                            {#if image.is_ai}
                                                <div class="px-1.5 py-0.5 rounded bg-primary/90 text-primary-foreground text-xs font-bold shadow-sm">
                                                    AI
                                                </div>
                                            {/if}
                                            {#if image.is_authorized}
                                                <div class="px-1.5 py-0.5 rounded bg-primary/90 text-primary-foreground text-xs font-bold shadow-sm" title="已获许可">
                                                    ©
                                                </div>
                                            {/if}
                                        </div>
                                    {/if}

                                    <!-- 选择框 -->
                                    {#if isSelectionMode}
                                        <div class={cn(
                                            "absolute top-2 left-2 w-5 h-5 rounded border-2 flex items-center justify-center",
                                            selectedImageIds.has(image.id) 
                                                ? "bg-primary border-primary text-primary-foreground" 
                                                : "bg-white/80 border-gray-300"
                                        )}>
                                            {#if selectedImageIds.has(image.id)}
                                                <Check class="h-3 w-3" />
                                            {/if}
                                        </div>
                                    {/if}
                                </div>
                            </ContextMenu.Trigger>
                            <ContextMenu.Content>
                                <ContextMenu.Item onclick={() => startMultiselect(image.id)}>
                                    <CheckSquare class="h-4 w-4 mr-2" />
                                    {isSelectionMode ? "多选" : "多选"}
                                </ContextMenu.Item>
                                <ContextMenu.Separator />
                                <ContextMenu.Item class="text-destructive" onclick={() => deleteImage(image.id)}>
                                    <Trash2 class="h-4 w-4 mr-2" />
                                    删除
                                </ContextMenu.Item>
                            </ContextMenu.Content>
                        </ContextMenu.Root>
                    {/each}
                </div>
            {/each}
        </div>

        <!-- 无限滚动哨兵 & 加载指示器 -->
        <div bind:this={sentinelEl} class="w-full py-8 flex justify-center">
            {#if isLoadingMore}
                <div class="flex items-center gap-2 text-muted-foreground">
                    <div class="animate-spin h-5 w-5 border-2 border-primary border-t-transparent rounded-full"></div>
                    <span>加载中...</span>
                </div>
            {:else if !hasMore && images.length > 0}
                <span class="text-muted-foreground text-sm">已加载全部 {totalItems} 张图片</span>
            {/if}
        </div>

        <!-- 移动分类对话框 -->
        <Dialog.Root bind:open={moveDialogOpen}>
            <Dialog.Content class="max-w-sm">
                <Dialog.Header>
                    <Dialog.Title>移动到分类</Dialog.Title>
                </Dialog.Header>
                <div class="py-4 space-y-2">
                    <p class="text-sm text-muted-foreground mb-4">
                        将选中的 {selectedImageIds.size} 张图片移动到：
                    </p>
                    <div class="grid grid-cols-2 gap-2 max-h-[60vh] overflow-y-auto">
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

        <!-- 删除确认对话框 - 复用 deleteDialogOpen 但增加逻辑判断 -->
        <AlertDialog.Root bind:open={deleteDialogOpen}>
            <AlertDialog.Content>
                <AlertDialog.Header>
                    <AlertDialog.Title>确定要删除吗？</AlertDialog.Title>
                    <AlertDialog.Description>
                        此操作将永久删除 {isBatchDeleteArgs ? `选中的 ${selectedImageIds.size} 张` : "该"}图片，无法恢复。
                    </AlertDialog.Description>
                </AlertDialog.Header>
                <AlertDialog.Footer>
                    <AlertDialog.Cancel onclick={() => { isBatchDeleteArgs = false; imageToDelete = null; }}>取消</AlertDialog.Cancel>
                    <AlertDialog.Action
                        class="bg-destructive !text-destructive-foreground hover:bg-destructive/90"
                        onclick={() => {
                            if (isBatchDeleteArgs) {
                                confirmBatchDelete();
                            } else if (imageToDelete) {
                                confirmDelete();
                            }
                        }}
                    >
                        确认删除
                    </AlertDialog.Action>
                </AlertDialog.Footer>
            </AlertDialog.Content>
        </AlertDialog.Root>

        <!-- 底部批量操作栏 -->
        {#if isSelectionMode && selectedImageIds.size > 0}
            <div class="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 w-[92%] sm:w-auto">
                <div class="bg-popover/95 backdrop-blur border shadow-xl rounded-2xl sm:rounded-full p-3 sm:px-6 sm:py-3 flex flex-col sm:flex-row items-center gap-3 sm:gap-4 animate-in slide-in-from-bottom duration-300">
                    
                    <!-- Top Row (Mobile): Count + Cancel -->
                    <div class="flex items-center justify-between w-full sm:w-auto sm:gap-4">
                        <span class="text-sm font-medium whitespace-nowrap pl-1">已选择 {selectedImageIds.size} 项</span>
                         <Button
                            variant="ghost"
                            size="sm"
                            class="h-8 text-muted-foreground sm:hidden"
                            onclick={() => {
                                selectedImageIds = new Set();
                                isSelectionMode = false;
                            }}
                        >
                            取消
                        </Button>
                        <div class="hidden sm:block h-4 w-px bg-border"></div>
                    </div>

                    <!-- Actions Row -->
                    <div class="flex items-center justify-between w-full sm:w-auto gap-2 overflow-x-auto pb-1 sm:pb-0 scrollbar-hide">
                         <Button 
                            size="sm" 
                            class="flex-1 sm:flex-none whitespace-nowrap"
                            onclick={handleBatchMove}
                        >
                            移动
                        </Button>

                        <Button 
                            size="sm"
                            class="flex-1 sm:flex-none whitespace-nowrap bg-green-600 hover:bg-green-700 text-white dark:bg-green-600 dark:hover:bg-green-700"
                            onclick={handleBatchAuthorized}
                        >
                            已获许可
                        </Button>

                        <Button 
                            size="sm" 
                            class="flex-1 sm:flex-none whitespace-nowrap bg-blue-600 hover:bg-blue-700 text-white dark:bg-blue-600 dark:hover:bg-blue-700"
                            disabled={isExporting}
                            onclick={handleBatchExport}
                        >
                            {#if isExporting}
                                打包...
                            {:else}
                                导出
                            {/if}
                        </Button>

                        <Button
                            size="sm"
                            variant="destructive"
                            class="flex-1 sm:flex-none whitespace-nowrap"
                            onclick={handleBatchDelete}
                        >
                            删除
                        </Button>
                    </div>
                
                     <!-- Desktop Cancel -->
                    <Button
                        size="sm"
                        variant="ghost"
                        class="hidden sm:inline-flex"
                        onclick={() => {
                            selectedImageIds = new Set();
                            isSelectionMode = false;
                        }}>取消选择</Button
                    >
                </div>
            </div>
        {/if}

    {/if}

    <!-- 加载更多 -->
    {#if totalPages > 1 && currentPage < totalPages}
        <div class="flex justify-center mt-6">
            <Button variant="outline" onclick={() => { currentPage++; fetchImages(); }}>
                加载更多
            </Button>
        </div>
    {/if}
</div>

<!-- 编辑对话框 -->
<Dialog.Root bind:open={editDialogOpen}>
    <Dialog.Content class="!w-[95%] !md:w-[90%] !lg:w-[90%] !max-w-none max-h-[90vh] overflow-hidden p-0">
        {#if editingImage}
            <div class="flex flex-col md:flex-row gap-6 max-h-[80vh] overflow-y-auto w-full mx-auto p-4 md:p-6">
                <!-- 图片预览 -->
                <div 
                    class="md:w-auto flex-shrink-0 flex flex-col items-center md:max-w-[50%]"
                    style="min-width: min(50%, calc(70vh * {editingImage.width} / {editingImage.height}))"
                >
                    <img
                        src={resolveUrl(editingImage.thumbnail_path)}
                        alt={editingImage.title}
                        width={editingImage.width}
                        height={editingImage.height}
                        class="w-auto h-auto max-w-full max-h-[70vh] rounded-3xl object-contain border shadow-sm"
                        style="aspect-ratio: {editingImage.width} / {editingImage.height}"
                    />
                    <div class="mt-3 text-sm text-muted-foreground w-full text-center">
                        {editingImage.width} × {editingImage.height} · {formatFileSize(editingImage.file_size)}
                    </div>
                </div>

                <!-- 信息表单 -->
                <div class="flex-1 space-y-6 pr-2 min-w-0">
                    <div class="space-y-2">
                        <Label class="text-base font-semibold text-foreground">标题</Label>
                        <Input bind:value={editingImage.title} class="h-10" />
                    </div>

                    <div class="space-y-2">
                        <Label class="text-base font-semibold text-foreground">分类</Label>
                        <select 
                            class="w-full h-10 px-3 rounded-md border bg-background text-sm"
                            bind:value={editingImage.category_id}
                        >
                            <option value={null}>未分类</option>
                            {#each categories as cat}
                                <option value={cat.id}>{cat.name}</option>
                            {/each}
                        </select>
                    </div>

                    <!-- 标签 -->
                    <div class="space-y-2">
                        <Label class="text-base font-semibold text-foreground">标签</Label>
                        <div class="flex flex-wrap gap-2 mb-2">
                            {#each editingImage.tags as tag, i}
                                {@const colorIndex = Math.abs(tag.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0)) % 10}
                                {@const colors = [
                                    "bg-red-100 text-red-700 border-red-200 hover:bg-red-200",
                                    "bg-orange-100 text-orange-700 border-orange-200 hover:bg-orange-200",
                                    "bg-amber-100 text-amber-700 border-amber-200 hover:bg-amber-200",
                                    "bg-green-100 text-green-700 border-green-200 hover:bg-green-200",
                                    "bg-emerald-100 text-emerald-700 border-emerald-200 hover:bg-emerald-200",
                                    "bg-teal-100 text-teal-700 border-teal-200 hover:bg-teal-200",
                                    "bg-cyan-100 text-cyan-700 border-cyan-200 hover:bg-cyan-200",
                                    "bg-blue-100 text-blue-700 border-blue-200 hover:bg-blue-200",
                                    "bg-indigo-100 text-indigo-700 border-indigo-200 hover:bg-indigo-200",
                                    "bg-purple-100 text-purple-700 border-purple-200 hover:bg-purple-200",
                                ]}
                                <div class={cn("flex items-center gap-1 px-2 py-1 rounded-md border text-sm transition-colors", colors[colorIndex])}>
                                    <span>{tag}</span>
                                    <button 
                                        class="ml-1 opacity-60 hover:opacity-100"
                                        onclick={() => {
                                            if (editingImage) {
                                                editingImage.tags = editingImage.tags.filter((_, idx) => idx !== i);
                                            }
                                        }}
                                    >
                                        <X class="h-3 w-3" />
                                    </button>
                                </div>
                            {/each}
                        </div>
                        <div class="flex gap-2">
                            <Input 
                                placeholder="输入标签后按回车添加" 
                                class="h-9"
                                onkeydown={(e) => {
                                    if (e.key === 'Enter' && e.currentTarget.value.trim() && editingImage) {
                                        e.preventDefault();
                                        const newTag = e.currentTarget.value.trim();
                                        if (!editingImage.tags.includes(newTag)) {
                                            editingImage.tags = [...editingImage.tags, newTag];
                                        }
                                        e.currentTarget.value = '';
                                    }
                                }}
                            />
                        </div>
                    </div>

                    <div class="flex items-center gap-6 py-2">
                        <label class="flex items-center gap-2 cursor-pointer group">
                            <div class={cn(
                                "w-5 h-5 rounded border border-input flex items-center justify-center transition-colors",
                                editingImage.is_ai ? "bg-primary border-primary text-primary-foreground" : "group-hover:border-primary"
                            )}>
                                {#if editingImage.is_ai}
                                    <Check class="h-3.5 w-3.5" />
                                {/if}
                            </div>
                            <input type="checkbox" bind:checked={editingImage.is_ai} class="hidden" />
                            <span class={cn("text-sm font-medium transition-colors", editingImage.is_ai ? "text-primary" : "text-foreground")}>AI生图</span>
                        </label>

                        <label class="flex items-center gap-2 cursor-pointer group">
                            <div class={cn(
                                "w-5 h-5 rounded border border-input flex items-center justify-center transition-colors",
                                editingImage.is_authorized ? "bg-primary border-primary text-primary-foreground" : "group-hover:border-primary"
                            )}>
                                {#if editingImage.is_authorized}
                                    <Check class="h-3.5 w-3.5" />
                                {/if}
                            </div>
                            <input type="checkbox" bind:checked={editingImage.is_authorized} class="hidden" />
                            <span class={cn("text-sm font-medium transition-colors", editingImage.is_authorized ? "text-primary" : "text-foreground")}>已获许可</span>
                        </label>
                    </div>

                    <!-- AI 信息 -->
                    {#if editingImage.is_ai}
                        <div class="space-y-4 p-4 rounded-lg bg-muted/30 border">
                            <div class="space-y-2">
                                <Label class="text-sm font-semibold text-foreground">生图平台</Label>
                                <select 
                                    class="w-full h-9 px-3 rounded-md border bg-background text-sm"
                                    bind:value={editingImage.ai_platform}
                                >
                                    <option value={null}>未知</option>
                                    <option value="NovelAI">NovelAI</option>
                                    <option value="Midjourney">Midjourney</option>
                                    <option value="ComfyUI">ComfyUI</option>
                                    <option value="StableDiffusion">Stable Diffusion</option>
                                    <option value="Other">其他</option>
                                </select>
                            </div>
                            
                            <div class="space-y-2">
                                <div class="flex items-center justify-between">
                                    <Label class="text-sm font-semibold text-foreground">正向提示词</Label>
                                    <Button variant="ghost" size="icon" class="h-6 w-6" onclick={() => {
                                        if (editingImage?.ai_prompt) {
                                            navigator.clipboard.writeText(editingImage.ai_prompt);
                                            toast.success("已复制");
                                        }
                                    }}>
                                        <Copy class="h-3.5 w-3.5" />
                                    </Button>
                                </div>
                                <textarea 
                                    class="w-full min-h-[80px] px-3 py-2 rounded-md border bg-background resize-y text-sm font-mono leading-relaxed"
                                    bind:value={editingImage.ai_prompt}
                                    placeholder="Positive prompt..."
                                ></textarea>
                            </div>
                            
                            <div class="space-y-2">
                                <div class="flex items-center justify-between">
                                    <Label class="text-sm font-semibold text-foreground">负向提示词</Label>
                                    <Button variant="ghost" size="icon" class="h-6 w-6" onclick={() => {
                                        if (editingImage?.ai_negative_prompt) {
                                            navigator.clipboard.writeText(editingImage.ai_negative_prompt);
                                            toast.success("已复制");
                                        }
                                    }}>
                                        <Copy class="h-3.5 w-3.5" />
                                    </Button>
                                </div>
                                <textarea 
                                    class="w-full min-h-[60px] px-3 py-2 rounded-md border bg-background resize-y text-sm font-mono leading-relaxed"
                                    bind:value={editingImage.ai_negative_prompt}
                                    placeholder="Negative prompt..."
                                ></textarea>
                            </div>
                        </div>
                    {/if}

                    <!--
                    <div class="space-y-2">
                        <Label class="text-base font-semibold text-foreground">用户备注</Label>
                        <textarea 
                            class="w-full min-h-[80px] px-3 py-2 rounded-md border bg-background resize-y text-sm"
                            bind:value={editingImage.user_notes}
                            placeholder="写点什么..."
                        ></textarea>
                    </div>
                    -->


                    <div class="text-sm text-muted-foreground">
                        导入时间: {formatDate(editingImage.created_at)}
                    </div>

                    <div class="flex gap-2 pt-4">
                        <Button onclick={saveImageChanges}>保存</Button>
                        <Button variant="outline" onclick={() => editDialogOpen = false}>取消</Button>
                        <Button variant="outline" onclick={() => handleExport(editingImage!.id)}>
                            <Download class="h-4 w-4 mr-1" />
                            导出原图
                        </Button>
                    </div>
                </div>
            </div>
        {/if}
    </Dialog.Content>
</Dialog.Root>
