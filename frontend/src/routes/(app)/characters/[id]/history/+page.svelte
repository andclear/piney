
<script lang="ts">
    import { onMount, onDestroy, tick, mount, unmount } from "svelte";
    import { page } from "$app/stores";
    import { 
        Loader2, ArrowLeft, Settings, Eye,
        ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight 
    } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Progress } from "$lib/components/ui/progress";
    import * as Popover from "$lib/components/ui/popover";
    import { Label } from "$lib/components/ui/label";
    import { Slider } from "$lib/components/ui/slider";
    import { toast } from "svelte-sonner";
    import { goto } from "$app/navigation";
    import { cn } from "$lib/utils";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import Iframe from "$lib/components/render/Iframe.svelte"; // New Iframe Component
    import { type RegexScript, processContentWithScripts } from "$lib/utils/regexProcessor";
    import { processTextWithPipelineSimple, processTextWithPipeline, createIframeContent, isHtmlCodeBlock } from "$lib/utils/renderUtils"; // New Pipeline
    import { detectTags, sortTags } from "$lib/utils/tagFilter";
    import { API_BASE } from "$lib/api";

    const historyId = $page.url.searchParams.get("history_id");
    const cardId = $page.params.id;

    // State
    let isLoading = $state(true);
    let title = $state("阅读模式");
    let characterName = $state("");
    let isTxtFormat = $state(false); // Detect format
    
    // Tag Settings
    let availableTags = $state(new Set<string>());
    let filterTags = $state<string[]>([]); // Tags to HIDE (Unchecked in UI)
    let newlineTags = $state<string[]>([]); // Tags to apply Newline conversion (Checked in UI)
    
    // Regex State
    let cardRegex: RegexScript[] = $state([]);
    let chatRegex: RegexScript[] = $state([]);
    
    // Pagination
    let currentPage = $state(1);
    let totalPages = $state(1);
    let jumpPage = $state(1);
    
    // Progress
    let globalProgress = $state(0);
    
    // Content
    interface ChatMessage {
        floor: number;
        name: string;
        content: string;
    }
    let floors = $state<ChatMessage[]>([]);
    
    // Settings
    let textBrightness = $state(100);
    let savedScrollRatio = 0; // Temp store for restoration
    
    // Auto-save timer
    let saveTimeout: any;

    // Cache for preloading next page
    let cacheData = $state<{
        page: number;
        floors: ChatMessage[];
        totalPages: number;
    } | null>(null);

    async function loadMetadata() {
        cacheData = null; // Reset cache on new load
        if (!cardId) return;
        const token = localStorage.getItem("auth_token");
        
        // 1. Get Character Name & Regex
        try {
            const cardRes = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (cardRes.ok) {
                const card = await cardRes.json();
                characterName = card.name;
                // Parse card regex
                try {
                    // Check structure. Usually card.data.extensions.regex_scripts
                    // Check structure. Support V1 and V2
                    const data = typeof card.data === 'string' ? JSON.parse(card.data) : card.data;
                    const v2Data = data?.data || {}; // V2 structure
                    
                    if (v2Data?.extensions?.regex_scripts) {
                        cardRegex = v2Data.extensions.regex_scripts;
                    } else if (data?.extensions?.regex_scripts) {
                        cardRegex = data.extensions.regex_scripts;
                    }
                } catch (e) { console.error("Failed to parse card regex", e); }
            }
        } catch {}

        // 2. Get History Metadata & Regex
        try {
            const res = await fetch(`${API_BASE}/api/cards/${cardId}/history`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                const list = await res.json();
                const item = list.find((h: any) => h.id === historyId);
                if (item) {
                    title = item.display_name;
                    currentPage = item.current_page || 1;
                    globalProgress = item.progress || 0;
                    // Detect TXT format
                    isTxtFormat = item.format === 'txt' || item.file_name?.toLowerCase().endsWith('.txt');
                    
                    if (item.reading_settings) {
                        try {
                            const settings = JSON.parse(item.reading_settings);
                            if (settings.brightness) textBrightness = settings.brightness;
                            if (settings.scroll_ratio) savedScrollRatio = settings.scroll_ratio;
                            if (settings.tag_filters) filterTags = settings.tag_filters;
                            if (settings.newline_tags) newlineTags = settings.newline_tags;
                        } catch {}
                    }
                    // Parse chat regex
                    if (item.regex_scripts) {
                         try {
                             chatRegex = JSON.parse(item.regex_scripts);
                         } catch (e) { console.error("Failed to parse chat regex", e); }
                    }
                }
            }
        } catch {}
        
        updateBreadcrumbs();
    }
    
    function updateBreadcrumbs() {
        breadcrumbs.set([
            { label: "角色库", href: "/characters" },
            { label: characterName || "角色", href: `/characters/${cardId}` },
            { label: "聊天记录", href: `/characters/${cardId}?tab=chat` },
            { label: title },
        ]);
    }

    async function loadPage(p: number, restoreScroll = false) {
        if (!historyId || !cardId) return;

        // Use cached data if available
        if (cacheData && cacheData.page === p) {
            totalPages = cacheData.totalPages;
            currentPage = cacheData.page;
            floors = cacheData.floors;
            jumpPage = currentPage;
            cacheData = null; // Clear cache
        } else {
            isLoading = true;
            try {
                const token = localStorage.getItem("auth_token");
                const res = await fetch(`${API_BASE}/api/cards/${cardId}/history/${historyId}/content?page=${p}`, {
                    headers: token ? { Authorization: `Bearer ${token}` } : {},
                });
                
                if (!res.ok) throw new Error("加载失败");
                
                const data = await res.json();
                
                totalPages = data.total_pages;
                currentPage = data.current_page;
                floors = data.floors;
                jumpPage = currentPage;

                // Backend Detected Tags - IGNORED as per new requirement: "Only show tags from current page after regex processing"
                // if (!isTxtFormat && (data as any).detected_tags) {
                //      (data as any).detected_tags.forEach((t: string) => availableTags.add(t));
                // }

                // Frontend Detected Tags (Using new pipeline dry-run if needed, but simple scan is ok)
                // We just want to find tags that might be created by regex
                if (!isTxtFormat && floors.length > 0) {
                     // Note: We don't fully run the render pipeline here, just regex to find tags
                     // This mimics the 'loadPage' logic from before but simplified
                     // Reset availableTags for this page view
                     const currentViewTags = new Set<string>();
                     floors.forEach(f => {
                         // Dry run regex for tag detection
                         const result = processTextWithPipeline(f.content, { 
                             chatRegex, cardRegex, 
                             hiddenTags: [], newlineTags: [] // No filtering
                         });
                         result.detectedTags.forEach(t => currentViewTags.add(t)); // Use detectedTags from pipeline result
                     });
                     availableTags = currentViewTags;
                }
            } catch (e) {
                console.error(e);
                toast.error("加载内容失败");
                isLoading = false;
                return;
            } finally {
                isLoading = false;
            }
        }

        // Common post-load logic (from cache or fresh fetch)
        try {
            await tick(); // Wait for DOM update
            
            // Restore scroll position with a slight delay to ensure layout
            if (restoreScroll && savedScrollRatio > 0) {
                setTimeout(() => {
                    const container = document.querySelector('.overflow-y-auto');
                    if (container) {
                        const { scrollHeight, clientHeight } = container;
                        container.scrollTop = (scrollHeight - clientHeight) * savedScrollRatio;
                    }
                }, 100);
            } else if (!restoreScroll) {
                 const container = document.querySelector('.overflow-y-auto');
                 if (container) container.scrollTop = 0;
            }

            // Trigger preload of NEXT page
            if (currentPage < totalPages) {
                setTimeout(() => preloadPage(currentPage + 1), 500);
            }

        } catch (e) {
            console.error(e);
        }
    }

    async function preloadPage(p: number) {
        if (!historyId || !cardId || p > totalPages) return;
        if (cacheData && cacheData.page === p) return;

        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}/history/${historyId}/content?page=${p}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            
            if (res.ok) {
                const data = await res.json();
                cacheData = {
                    page: p,
                    floors: data.floors,
                    totalPages: data.total_pages
                };
                
                // Also scan tags for the preloaded page to keep the menu updated (Global)
                // Tag scanning removed from preloadPage to ensure 'availableTags' 
                // only reflects the currently viewed page as per requirements.
            }
        } catch (e) {
            // Silently fail preload
        }
    }
    
    function isTagMaskedByRegex(tag: string): boolean {
        const patternString = `<${tag}`;
        const check = (scripts: RegexScript[]) => {
            return scripts.some(s => {
                if (!s.findRegex) return false;
                // Check if the regex pattern contains the tag (case insensitive)
                return s.findRegex.toLowerCase().includes(patternString.toLowerCase());
            });
        };
        return check(chatRegex) || check(cardRegex);
    }

    function handlePageChange(newPage: number) {
        if (newPage < 1 || newPage > totalPages) return;
        savedScrollRatio = 0; 
        currentPage = newPage;
        saveProgress();
        loadPage(newPage, false);
    }

    function handleScroll(e: Event) {
        const target = e.target as HTMLElement;
        const { scrollTop, scrollHeight, clientHeight } = target;
        
        // Update saved ratio immediately
        let ratio = 0;
        if (scrollHeight > clientHeight) {
            ratio = scrollTop / (scrollHeight - clientHeight);
        }
        savedScrollRatio = ratio;
        
        // Calculate Global Progress
        const totalRaw = ((currentPage - 1) + ratio) / totalPages;
        globalProgress = Math.min(100, Math.round(totalRaw * 100));
        
        // Debounce Save
        clearTimeout(saveTimeout);
        saveTimeout = setTimeout(() => {
            saveProgress();
        }, 1000);
    }

    async function saveProgress() {
        if (!historyId || !cardId) return;
        try {
            const token = localStorage.getItem("auth_token");
            const settings = JSON.stringify({ 
                brightness: textBrightness,
                scroll_ratio: savedScrollRatio,
                tag_filters: filterTags,
                newline_tags: newlineTags
            });
            
            await fetch(`${API_BASE}/api/cards/${cardId}/history/${historyId}`, {
                method: 'PATCH',
                headers: { 
                    'Content-Type': 'application/json',
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ 
                    progress: globalProgress,
                    current_page: currentPage,
                    reading_settings: settings
                })
            });
        } catch {}
    }
    
    // Debounced save for brightness
    function handleBrightnessChange(val: number[]) {
        textBrightness = val[0];
        triggerSave();
    }
    
    function triggerSave() {
        clearTimeout(saveTimeout);
        saveTimeout = setTimeout(() => { saveProgress(); }, 1000);
    }
    
    function toggleTagFilter(tag: string, checked: boolean) {
        // UI: Checked = Visible. 
        // Logic: filterTags stores HIDDEN.
        // So Checked(true) -> Remove from filterTags.
        // Unchecked(false) -> Add to filterTags.
        if (checked) {
            filterTags = filterTags.filter(t => t !== tag);
        } else {
            if(!filterTags.includes(tag)) filterTags = [...filterTags, tag];
        }
        triggerSave();
    }
    
    function toggleNewlineTag(tag: string, checked: boolean) {
        // UI: Checked = Enabled.
        if (checked) {
            if(!newlineTags.includes(tag)) newlineTags = [...newlineTags, tag];
        } else {
             newlineTags = newlineTags.filter(t => t !== tag);
        }
        triggerSave();
    }

    // --- DEBUG RAW VIEW ---
    let rawViewFloors = $state(new Set<number>());

    function toggleRawView(floor: number) {
        const newSet = new Set(rawViewFloors);
        if (newSet.has(floor)) {
            newSet.delete(floor);
        } else {
            newSet.add(floor);
        }
        rawViewFloors = newSet;
    }

    function getDebugRawText(content: string) {
        // 1. Normalize
        let res = content.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
        // 2. Regex (Display Context)
        res = processContentWithScripts(res, chatRegex, { isMarkdown: true });
        res = processContentWithScripts(res, cardRegex, { isMarkdown: true });
        return res;
    }

    // --- NEW PIPELINE RENDERING ---
    
    // Action to mount Iframe components in place of code blocks
    function mountCodeBlockIframes(node: HTMLElement, content: string) {
        // Track mounted components to clean up
        const mountedComponents: any[] = [];

        function update() {
            // 查找所有标记为需要 iframe 渲染的代码块
            const iframeBlocks = node.querySelectorAll('pre.piney-iframe-code');
            
            iframeBlocks.forEach((pre) => {
                const codeBlock = pre.querySelector('code');
                if (!codeBlock) return;

                // 获取原始 HTML 内容（需要解码 HTML 实体）
                let rawHtml = codeBlock.textContent || "";
                
                // 创建容器
                const container = document.createElement('div');
                container.className = "th-iframe-wrapper w-full my-4";
                
                // 替换 <pre> 为容器
                pre.replaceWith(container);
                
                // 挂载 Iframe 组件
                const component = mount(Iframe, {
                    target: container,
                    props: { content: rawHtml }
                });
                
                mountedComponents.push(component);
            });
        }

        update();

        return {
            update(newContent: string) {
            },
            destroy() {
                mountedComponents.forEach(c => unmount(c));
            }
        };
    }

    // Helper to get processed HTML
    function getProcessedHtml(content: string): string {
        if (isTxtFormat) {
            // Escape HTML for safety in TXT mode
             return content
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;');
        }
        return processTextWithPipelineSimple(content, {
            chatRegex,
            cardRegex,
            hiddenTags: filterTags,
            newlineTags: newlineTags
        });
    }

    onMount(async () => {
        await loadMetadata();
        await loadPage(currentPage, true);

        // Keyboard Navigation
        window.addEventListener('keydown', handleKeydown);
    });

    onDestroy(() => {
        if (typeof window !== 'undefined') {
            window.removeEventListener('keydown', handleKeydown);
        }
    });

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'ArrowLeft') {
            if (currentPage > 1) handlePageChange(currentPage - 1);
        } else if (e.key === 'ArrowRight' || e.key.toLowerCase() === 'd') {
            if (currentPage < totalPages) handlePageChange(currentPage + 1);
        }
    }

    let touchStartX = 0;
    let touchStartY = 0;
    let touchStartTime = 0;

    function handleTouchStart(e: TouchEvent) {
        touchStartX = e.changedTouches[0].clientX;
        touchStartY = e.changedTouches[0].clientY;
        touchStartTime = Date.now();
    }

    function handleTouchEnd(e: TouchEvent) {
        const touchEndX = e.changedTouches[0].clientX;
        const touchEndY = e.changedTouches[0].clientY;
        const diffX = touchEndX - touchStartX;
        const diffY = touchEndY - touchStartY;
        const duration = Date.now() - touchStartTime;

        if (duration < 300 && Math.abs(diffX) > 50 && Math.abs(diffX) > Math.abs(diffY) * 1.5) {
            if (diffX > 0) {
                if (currentPage > 1) handlePageChange(currentPage - 1);
            } else {
                if (currentPage < totalPages) handlePageChange(currentPage + 1);
            }
        }
    }
</script>

<style>
    /* Styles ported from HTMLRender and renderUtils for MAIN DOM rendering */
    :global(.chat-content) {
        line-height: 1.8;
    }
    :global(.chat-content p) {
        margin-bottom: 1.5em;
    }
    :global(.chat-content p:last-child) {
        margin-bottom: 0;
    }
    :global(.chat-content code) {
        background: rgba(128,128,128,0.2); 
        padding: 2px 6px; 
        border-radius: 4px; 
        font-family: ui-monospace, monospace;
        font-size: 0.9em;
    }
    :global(.chat-content pre) {
        background: #f5f5f5;
        border: 1px solid #e5e5e5;
        border-radius: 8px;
        padding: 1rem;
        margin: 1rem 0;
        overflow-x: auto;
    }
    :global(.dark .chat-content pre) {
        background: #1e1e1e;
        border-color: #333;
    }
    :global(.chat-content q) { color: #2e7d32; }
    :global(.dark .chat-content q) { color: #99cc99; }
    :global(.chat-content q::before) { content: '"'; }
    :global(.chat-content q::after) { content: '"'; }
    :global(.chat-content em) { color: #b8860b; font-style: italic; }
    :global(.dark .chat-content em) { color: #ffcc00; }
    :global(.chat-content strong) { color: #c62828; font-weight: bold; }
    :global(.dark .chat-content strong) { color: #ff9966; }
    :global(.chat-content del) { color: #888888; text-decoration: line-through; }
</style>

<div class="flex flex-col h-screen bg-background transition-all" style="filter: brightness({textBrightness}%);">
    <!-- Header -->
    <header class="flex items-center gap-4 border-b px-6 py-3 bg-card/80 backdrop-blur-sm sticky top-0 z-10 justify-between">
        <div class="flex items-center gap-2 md:gap-4 flex-1 min-w-0 mr-2">
            <Button variant="ghost" size="icon" onclick={() => goto(`/characters/${cardId}?tab=chat`)} class="shrink-0">
                <ArrowLeft class="h-5 w-5" />
            </Button>
            <div class="flex flex-col min-w-0 flex-1 md:block">
                <div class="flex items-center gap-2">
                    <h1 class="text-base md:text-lg font-semibold truncate">{title}</h1>
                    <span class="md:hidden text-xs text-muted-foreground font-mono shrink-0 whitespace-nowrap">{globalProgress}%</span>
                </div>
                <div class="hidden md:flex items-center gap-2 text-xs text-muted-foreground">
                    <span class="font-mono">进度 {globalProgress}%</span>
                    <span>•</span>
                    <span>第 {currentPage} 页 / 共 {totalPages} 页</span>
                </div>
            </div>
        </div>
        <div class="flex items-center gap-2">
             <div class="w-32 mr-4 hidden md:block"><Progress value={globalProgress} class="h-2" /></div>
             <Popover.Root>
                <Popover.Trigger><Button variant="ghost" size="icon"><Settings class="h-5 w-5" /></Button></Popover.Trigger>
                <Popover.Content class="w-80">
                    <div class="grid gap-4">
                        <div class="space-y-2"><h4 class="font-medium leading-none">阅读设置</h4></div>
                        
                        {#if availableTags.size > 0}
                            <div class="grid gap-2 border-b pb-4">
                                <Label>标签显示</Label>
                                <p class="text-xs text-muted-foreground">选择的标签内容将会显示</p>
                                <div class="grid grid-cols-2 gap-2 max-h-40 overflow-y-auto">
                                    {#each sortTags(availableTags) as tag}
                                        <div class="flex items-center space-x-2">
                                            <Checkbox 
                                                id={`filter-${tag}`} 
                                                checked={!filterTags.includes(tag)} 
                                                onCheckedChange={(v) => toggleTagFilter(tag, !!v)} 
                                            />
                                            <Label for={`filter-${tag}`} class="text-xs font-normal cursor-pointer leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">{tag}</Label>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                            
                            <div class="grid gap-2 border-b pb-4">
                                <Label>标签分行</Label>
                                <p class="text-xs text-muted-foreground">选择的标签内容将会很好的分行显示</p>
                                <div class="grid grid-cols-2 gap-2 max-h-40 overflow-y-auto">
                                    {#each sortTags(availableTags) as tag}
                                        <div class="flex items-center space-x-2">
                                            <Checkbox 
                                                id={`newline-${tag}`} 
                                                checked={newlineTags.includes(tag)} 
                                                onCheckedChange={(v) => toggleNewlineTag(tag, !!v)} 
                                            />
                                            <Label for={`newline-${tag}`} class="text-xs font-normal cursor-pointer leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">{tag}</Label>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {/if}

                        <div class="grid gap-2"><Label>文字亮度</Label><div class="flex items-center gap-4"><Slider value={[textBrightness]} max={100} step={5} onValueChange={handleBrightnessChange} class="flex-1" type="multiple" /><span class="w-8 text-sm text-right">{textBrightness}%</span></div></div>
                    </div>
                </Popover.Content>
             </Popover.Root>
        </div>
    </header>

    <!-- Content -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div 
        class="flex-1 overflow-y-auto p-2 md:p-8 bg-muted/10 outline-none touch-pan-y" 
        onscroll={handleScroll}
        ontouchstart={handleTouchStart}
        ontouchend={handleTouchEnd}
        role="region"
        tabindex="0"
        aria-label="Chat History Content"
    >
        <div class="mx-auto max-w-3xl space-y-4">
            {#if isLoading}
                <div class="flex h-64 items-center justify-center"><Loader2 class="h-8 w-8 animate-spin text-primary" /></div>
            {:else}
                {#each floors as floor, i (floor.floor)}
                    <div class={cn("rounded-lg border p-3 md:p-6 shadow-sm", i % 2 === 0 ? "bg-card" : "bg-card/50")}>
                        <div class="flex items-center justify-between mb-4 pb-2 border-b border-border/50">
                            <span class="font-mono text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded">#{floor.floor}</span>
                            <div class="flex items-center gap-2">
                                <span class="font-semibold opacity-90">{floor.name}</span>
                                <Button variant="ghost" size="icon" class="h-6 w-6 ml-2" onclick={() => toggleRawView(floor.floor)} title="查看渲染前内容 (Regex Only)">
                                    <Eye class="h-4 w-4 text-muted-foreground" />
                                </Button>
                            </div>
                        </div>

                        {#if rawViewFloors.has(floor.floor)}
                            <div class="mb-4">
                                <div class="text-[10px] uppercase font-bold text-muted-foreground/50 mb-1">正则处理后 (After Regex)</div>
                                <textarea 
                                    class="w-full h-48 p-3 text-xs bg-muted/50 border rounded-md font-mono resize-y focus:outline-none focus:ring-1 focus:ring-ring" 
                                    readonly
                                >{getDebugRawText(floor.content)}</textarea>
                            </div>
                        {/if}

                        <div class="leading-relaxed text-foreground/90 w-full min-h-[50px] chat-content">
                            <!-- New Rendering Pipeline -->
                            {#if isTxtFormat}
                                <div class="p-2 md:p-4 whitespace-pre-wrap font-sans text-base leading-relaxed break-words text-foreground">
                                    {@html getProcessedHtml(floor.content)}
                                </div>
                            {:else}
                                <!-- Use action to mount iframes after rendering HTML -->
                                <div 
                                    use:mountCodeBlockIframes={floor.content}
                                    class="break-words"
                                >
                                    {@html getProcessedHtml(floor.content)}
                                </div>
                            {/if}
                        </div>
                    </div>
                {/each}

                <!-- Pagination Controls -->
                <div class="flex flex-col md:flex-row items-center justify-center gap-4 py-8 mt-8">
                     <div class="flex items-center gap-2">
                         <Button variant="outline" size="icon" disabled={currentPage <= 1} onclick={() => handlePageChange(1)}>
                             <ChevronsLeft class="h-4 w-4" />
                         </Button>
                         <Button variant="outline" size="icon" disabled={currentPage <= 1} onclick={() => handlePageChange(currentPage - 1)}>
                             <ChevronLeft class="h-4 w-4" />
                         </Button>
                         
                         <div class="flex items-center gap-2 mx-2">
                             <Input 
                                type="number" 
                                class="w-16 text-center" 
                                min={1} 
                                max={totalPages}
                                bind:value={jumpPage} 
                                onkeydown={(e) => {
                                    if(e.key === 'Enter') handlePageChange(jumpPage);
                                }}
                             />
                             <span class="text-sm text-muted-foreground">/ {totalPages}</span>
                         </div>

                         <Button variant="outline" size="icon" disabled={currentPage >= totalPages} onclick={() => handlePageChange(currentPage + 1)}>
                             <ChevronRight class="h-4 w-4" />
                         </Button>
                         <Button variant="outline" size="icon" disabled={currentPage >= totalPages} onclick={() => handlePageChange(totalPages)}>
                             <ChevronsRight class="h-4 w-4" />
                         </Button>
                     </div>
                </div>
            {/if}
        </div>
    </div>
</div>
