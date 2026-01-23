<script lang="ts">
    import { onMount } from "svelte";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
    import * as Card from "$lib/components/ui/card";
    import { Button } from "$lib/components/ui/button";
    import { API_BASE, resolveUrl } from "$lib/api";
    import { User, Book, Zap, Database, ArrowRight, Sparkles, Dices } from "lucide-svelte";
    import { goto } from "$app/navigation";
    import { toast } from "svelte-sonner";
    import { auth } from "$lib/stores/auth.svelte";

    interface SimpleCard {
        id: string;
        name: string;
        avatar: string | null;
        updated_at: string;
    }
    interface LuckyCardInfo {
        id: string;
        name: string;
        avatar: string | null;
        description: string | null;
    }
    interface DashboardStats {
        total_characters: number;
        total_world_info: number;
        total_tokens_k: number;
        db_size_mb: number;
        recent_cards: SimpleCard[];
        lucky_card: LuckyCardInfo | null;
        username: string;
        gacha_remaining: number;
        gacha_confirmed: boolean;
    }

    let stats: DashboardStats | null = $state(null);
    let loading = $state(true);
    let today = new Date().toLocaleDateString("zh-CN", { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' });

    onMount(async () => {
        breadcrumbs.set([]);
        
        // SWR 策略：先显示缓存（如果有效），后台刷新数据
        const CACHE_TTL = 5 * 60 * 1000; // 5分钟缓存
        const cached = localStorage.getItem("dashboard_cache");
        const cachedTime = localStorage.getItem("dashboard_cache_time");
        
        // 检查缓存是否有效（5分钟内）
        const isCacheValid = cached && cachedTime && 
            (Date.now() - parseInt(cachedTime)) < CACHE_TTL;
        
        if (isCacheValid) {
            try {
                stats = JSON.parse(cached);
                loading = false; // 立即显示缓存内容
            } catch (e) {
                console.error("Cache parse error", e);
            }
        }

        // 无论是否有缓存，都后台刷新数据
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/dashboard`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                const data = await res.json();
                stats = data;
                // 更新缓存
                localStorage.setItem("dashboard_cache", JSON.stringify(data));
                localStorage.setItem("dashboard_cache_time", Date.now().toString());
            } else {
                console.error("Failed to load stats");
                if (!stats) toast.error("加载看板数据失败");
            }
        } catch (e) {
            console.error(e);
            if (!stats) toast.error("加载看板数据失败");
        } finally {
            loading = false;
        }
    });

    function formatTime(iso: string) {
        // 后端返回的是 UTC 时间但没有 Z 后缀，需要手动添加
        const d = new Date(iso.endsWith('Z') ? iso : iso + 'Z');
        const now = new Date();
        const diff = (now.getTime() - d.getTime()) / 1000;
        
        if (diff < 60) return "刚刚";
        if (diff < 3600) return `${Math.floor(diff / 60)}分钟前`;
        if (diff < 86400) return `${Math.floor(diff / 3600)}小时前`;
        if (diff < 86400 * 2) return "昨天";
        return d.toLocaleDateString();
    }

    function formatTokenCount(k: number) {
        if (!k) return { value: 0, unit: "K" };
        if (k >= 10000000) return { value: (k / 1000000).toFixed(2), unit: "B" };
        if (k >= 10000) return { value: (k / 1000).toFixed(2), unit: "M" };
        return { value: k, unit: "K" };
    }
</script>

<div class="container animate-in fade-in duration-700 py-8 space-y-10 max-w-7xl mx-auto">
    <!-- Header -->
    <div class="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
        <h1 class="text-3xl font-bold tracking-tight">欢迎回来，{auth.username || '用户'}</h1>
        <div class="text-muted-foreground text-sm">{today}</div>
    </div>
    
    <!-- Stats Row -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-6">
        <!-- 角色总数 -->
        <div class="group relative overflow-hidden rounded-2xl border bg-card p-4 sm:p-6 shadow-sm transition-all hover:shadow-md hover:-translate-y-0.5 min-h-[120px] sm:min-h-[160px] flex justify-between">
            <div class="flex flex-col justify-between z-10 w-full overflow-hidden">
                 <p class="text-xs sm:text-sm font-medium text-muted-foreground/70">角色总数</p>
                 <h3 class="text-2xl sm:text-4xl font-bold tracking-tight text-foreground flex items-baseline gap-1 whitespace-nowrap">
                    {#if loading} <span class="animate-pulse bg-muted rounded h-8 w-12 block"></span> {:else} {stats?.total_characters || 0} {/if}
                 </h3>
                 <div class="text-[10px] sm:text-xs text-muted-foreground truncate font-medium">
                     记录在这个世界的灵魂
                 </div>
            </div>
            <div class="flex h-9 w-9 sm:h-12 sm:w-12 items-center justify-center rounded-xl bg-orange-500/10 text-orange-600 dark:text-orange-400 group-hover:scale-110 transition-transform flex-shrink-0 ml-1 sm:ml-2">
                <User class="h-5 w-5 sm:h-6 sm:w-6" />
            </div>
        </div>

        <!-- Token 消耗 -->
        <div class="group relative overflow-hidden rounded-2xl border bg-card p-4 sm:p-6 shadow-sm transition-all hover:shadow-md hover:-translate-y-0.5 min-h-[120px] sm:min-h-[160px] flex justify-between">
            <div class="flex flex-col justify-between z-10 w-full overflow-hidden">
                 <p class="text-xs sm:text-sm font-medium text-muted-foreground/70">Token 总消耗</p>
                 <h3 class="text-2xl sm:text-4xl font-bold tracking-tight text-foreground flex items-baseline gap-1 whitespace-nowrap">
                    {#if loading} 
                        <span class="animate-pulse bg-muted rounded h-8 w-12 block"></span> 
                    {:else} 
                        {@const t = formatTokenCount(stats?.total_tokens_k || 0)}
                        {t.value} <span class="text-lg sm:text-2xl font-normal text-muted-foreground">{t.unit}</span>
                    {/if}
                 </h3>
                 <div class="text-[10px] sm:text-xs text-muted-foreground truncate font-medium">
                     编织故事的代价
                 </div>
            </div>
            <div class="flex h-9 w-9 sm:h-12 sm:w-12 items-center justify-center rounded-xl bg-teal-500/10 text-teal-600 dark:text-teal-400 group-hover:scale-110 transition-transform flex-shrink-0 ml-1 sm:ml-2">
                <Zap class="h-5 w-5 sm:h-6 sm:w-6" />
            </div>
        </div>

        <!-- 世界书 -->
        <div class="group relative overflow-hidden rounded-2xl border bg-card p-4 sm:p-6 shadow-sm transition-all hover:shadow-md hover:-translate-y-0.5 min-h-[120px] sm:min-h-[160px] flex justify-between">
            <div class="flex flex-col justify-between z-10 w-full overflow-hidden">
                 <p class="text-xs sm:text-sm font-medium text-muted-foreground/70">世界书条目</p>
                 <h3 class="text-2xl sm:text-4xl font-bold tracking-tight text-foreground flex items-baseline gap-1 whitespace-nowrap">
                    {#if loading} <span class="animate-pulse bg-muted rounded h-8 w-12 block"></span> {:else} {stats?.total_world_info || 0} {/if}
                 </h3>
                 <div class="text-[10px] sm:text-xs text-muted-foreground truncate font-medium">
                     构建世界的基石
                 </div>
            </div>
            <div class="flex h-9 w-9 sm:h-12 sm:w-12 items-center justify-center rounded-xl bg-blue-500/10 text-blue-600 dark:text-blue-400 group-hover:scale-110 transition-transform flex-shrink-0 ml-1 sm:ml-2">
                <Book class="h-5 w-5 sm:h-6 sm:w-6" />
            </div>
        </div>

        <!-- 数据库容量 -->
        <div class="group relative overflow-hidden rounded-2xl border bg-card p-4 sm:p-6 shadow-sm transition-all hover:shadow-md hover:-translate-y-0.5 min-h-[120px] sm:min-h-[160px] flex justify-between">
            <div class="flex flex-col justify-between z-10 w-full overflow-hidden">
                 <p class="text-xs sm:text-sm font-medium text-muted-foreground/70">知识库容量</p>
                 <h3 class="text-2xl sm:text-4xl font-bold tracking-tight text-foreground flex items-baseline gap-1 whitespace-nowrap">
                    {#if loading} <span class="animate-pulse bg-muted rounded h-8 w-12 block"></span> {:else} {stats?.db_size_mb || 0} <span class="text-lg sm:text-2xl font-normal text-muted-foreground">MB</span>{/if}
                 </h3>
                 <div class="text-[10px] sm:text-xs text-muted-foreground truncate font-medium">
                     承载记忆的重量
                 </div>
            </div>
            <div class="flex h-9 w-9 sm:h-12 sm:w-12 items-center justify-center rounded-xl bg-purple-500/10 text-purple-600 dark:text-purple-400 group-hover:scale-110 transition-transform flex-shrink-0 ml-1 sm:ml-2">
                <Database class="h-5 w-5 sm:h-6 sm:w-6" />
            </div>
        </div>
    </div>

    <!-- Main Content -->
    <div class="grid grid-cols-1 lg:grid-cols-5 gap-6 items-stretch">
        <!-- Recent Edits (Left) -->
        <div class="lg:col-span-3 flex flex-col h-full">
            <h2 class="text-xl font-bold tracking-tight mb-4">最近编辑</h2>
            
            <div class="rounded-2xl border bg-card text-card-foreground shadow-sm h-full flex flex-col overflow-hidden">
                {#if loading}
                    <div class="p-6 space-y-4">
                        {#each Array(5) as _}
                            <div class="h-16 w-full bg-muted/40 rounded animate-pulse"></div>
                        {/each}
                    </div>
                {:else}
                    <div class="flex flex-col divide-y divide-border h-full">
                        {#each stats?.recent_cards || [] as card}
                           <div 
                                class="group flex items-center justify-between p-4 pl-5 hover:bg-muted/30 transition-all cursor-pointer flex-1" 
                                onclick={() => goto(`/characters/${card.id}`)}
                            >
                               <div class="flex items-center gap-4 min-w-0 flex-1">
                                   <!-- 2:3 Aspect Ratio Thumbnail -->
                                   <div class="relative w-8 aspect-[2/3] rounded-sm overflow-hidden bg-muted flex-shrink-0 shadow-sm border border-border/50">
                                       <img src={resolveUrl(card.avatar)} alt={card.name} class="w-full h-full object-cover transition-transform group-hover:scale-105" />
                                   </div>
                                   
                                   <div class="min-w-0 flex-1">
                                       <div class="font-bold text-base truncate group-hover:text-primary transition-colors pr-4">
                                           {card.name}
                                       </div>
                                       <div class="text-xs text-muted-foreground mt-1">
                                           {formatTime(card.updated_at)}
                                       </div>
                                   </div>
                               </div>
                               <ArrowRight class="h-4 w-4 mr-4 text-muted-foreground/50 group-hover:text-primary group-hover:translate-x-1 transition-all flex-shrink-0" />
                           </div>
                        {/each}
                        {#if !stats?.recent_cards?.length}
                             <div class="flex-1 flex items-center justify-center text-muted-foreground py-12">暂无编辑记录</div>
                        {/if}
                    </div>
                {/if}
            </div>
        </div>

        <!-- Daily Lucky (Right) -->
        <div class="lg:col-span-2 flex flex-col h-full">
             <h2 class="text-xl font-bold tracking-tight mb-4">今日幸运星</h2>
             
             <div class="rounded-2xl border bg-card shadow-sm p-6 h-full flex flex-col justify-between relative overflow-hidden">
                 <!-- Background decoration -->
                 <div class="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent pointer-events-none"></div>

                 {#if loading}
                    <div class="w-full h-64 bg-muted animate-pulse rounded-xl"></div>
                 {:else if stats?.lucky_card}
                    <div class="w-full flex-1 flex flex-col items-center justify-center py-4 z-10">
                        <div 
                            class="relative w-48 sm:w-56 aspect-[2/3] rounded-xl overflow-hidden shadow-xl cursor-pointer group hover:-translate-y-1 transition-transform duration-300 ring-1 ring-border/50" 
                            onclick={() => goto(`/characters/${stats?.lucky_card?.id}`)}
                            style="cursor: url(&quot;data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32' viewBox='0 0 24 24' fill='%238b5cf6' stroke='white' stroke-width='2'><path d='M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z'/></svg>&quot;) 16 16, auto;"
                        >
                             <img src={resolveUrl(stats.lucky_card.avatar)} alt={stats.lucky_card.name} class="w-full h-full object-cover" />
                             
                             <!-- Name Overlay Bottom Center -->
                             <div class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent pt-12 pb-4 px-4 text-center group-hover:from-black/90 transition-all">
                                 <span class="text-white font-bold text-sm sm:text-base truncate block w-full drop-shadow-md">
                                     {stats.lucky_card.name}
                                 </span>
                             </div>
                        </div>
                    </div>
                    
                        <div class="w-full pt-6 border-t border-dashed border-border/60 space-y-4">
                            <p class="text-center text-sm text-muted-foreground italic font-medium font-serif">
                                “此刻的相遇，是命运最好的安排。”
                            </p>
                            
                            {#if stats?.gacha_confirmed}
                                <Button disabled class="w-full bg-muted text-muted-foreground shadow-none h-11 text-base">
                                    今天已逆天行事过，明天再来吧！
                                </Button>
                            {:else if (stats?.gacha_remaining ?? 0) <= 0}
                                <Button disabled class="w-full bg-muted text-muted-foreground shadow-none h-11 text-base">
                                    今天逆天已力竭，请明天再来
                                </Button>
                            {:else}
                                <Button 
                                    class="w-full bg-primary hover:bg-primary/90 text-primary-foreground shadow-lg transition-all h-11 text-base" 
                                    variant="default"
                                    onclick={() => goto('/gacha')}
                                >
                                    <Dices class="mr-2 h-5 w-5" /> 我命由我不由天，抽卡！
                                </Button>
                            {/if}
                        </div>
                 {:else}
                     <div class="flex flex-col items-center justify-center h-full text-muted-foreground">
                         <span class="mb-2 text-2xl">⏳</span>
                         命运的齿轮暂未转动...
                     </div>
                 {/if}
             </div>
        </div>
    </div>
</div>
