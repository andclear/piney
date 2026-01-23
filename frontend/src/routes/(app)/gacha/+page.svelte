<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { Button } from "$lib/components/ui/button";
    import { API_BASE, resolveUrl } from "$lib/api";
    import { toast } from "svelte-sonner";
    import { Dices, Check, RotateCcw, Sparkles } from "lucide-svelte";
    import { fly, fade } from "svelte/transition";
    import { cubicOut, cubicInOut } from "svelte/easing";

    let phase = $state<"ready" | "shuffling" | "spreading" | "picking" | "revealed">("ready");
    let cards = $state<any[]>([]);
    let selectedIndex = $state<number | null>(null);
    let remaining = $state(0);
    let loading = $state(true);
    let totalCardCount = $state(0); // 系统中的角色卡总数
    let displayCardCount = $derived(Math.min(3, totalCardCount)); // 实际显示的卡片数量

    onMount(async () => {
        await checkStatus();
    });

    async function checkStatus() {
        try {
            const token = localStorage.getItem("auth_token");
            
            // 并行获取抽卡状态和卡片总数
            const [dashboardRes, cardsRes] = await Promise.all([
                fetch(`${API_BASE}/api/dashboard`, {
                    headers: token ? { "Authorization": `Bearer ${token}` } : {}
                }),
                fetch(`${API_BASE}/api/cards?page_size=1`, {
                    headers: token ? { "Authorization": `Bearer ${token}` } : {}
                })
            ]);
            
            if (dashboardRes.ok) {
                const stats = await dashboardRes.json();
                remaining = stats.gacha_remaining;
                if (stats.gacha_confirmed) {
                    toast.error("今天已经完成抽卡了");
                    goto("/");
                }
            }
            
            if (cardsRes.ok) {
                const cardsData = await cardsRes.json();
                totalCardCount = cardsData.total || 0;
            }
            
            loading = false;
        } catch (e) {
            console.error(e);
            loading = false;
        }
    }

    async function startGacha() {
        if (remaining <= 0) {
            toast.error("次数已用完");
            return;
        }
        
        if (totalCardCount === 0) {
            toast.error("角色库中还没有角色卡哦，快去创建一个吧！");
            return;
        }
        
        if (totalCardCount < 3) {
            toast.info(`角色卡不足3张，将使用${totalCardCount}张进行抽卡`);
        }

        phase = "shuffling";
        
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/gacha/draw`, {
                 method: "POST",
                 headers: token ? { "Authorization": `Bearer ${token}` } : {}
            });
            
            if (!res.ok) {
                const err = await res.text();
                toast.error(err || "抽卡失败");
                phase = "ready";
                return;
            }

            const newCards = await res.json();
            
            // Wait for shuffle animation (2s + buffer)
            await new Promise(r => setTimeout(r, 2200));
            
            cards = newCards;
            // Transition to spreading phase
            phase = "spreading";
            
            // Wait for spread animation to complete (1.5s)
            await new Promise(r => setTimeout(r, 1500));
            
            // Now allow picking
            phase = "picking";
            selectedIndex = null;

        } catch (e) {
            console.error(e);
            toast.error("网络错误");
            phase = "ready";
        }
    }

    async function selectCard(idx: number) {
        if (phase !== "picking") return;
        
        // Optimistic UI? No, must verify count.
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/gacha/reveal`, {
                method: "POST",
                headers: token ? { "Authorization": `Bearer ${token}` } : {}
            });

            if (!res.ok) {
                const err = await res.text();
                toast.error(err || "无法翻牌（次数不足？）");
                return;
            }

            // Success -> Reveal
            remaining -= 1; 
            selectedIndex = idx;
            phase = "revealed";

        } catch (e) {
             toast.error("网络错误");
        }
    }

    function resetRound() {
        cards = [];
        selectedIndex = null;
        phase = "ready";
    }

    async function confirmSelection() {
        if (selectedIndex === null) return;
        const card = cards[selectedIndex];
        
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/gacha/confirm`, {
                method: "POST",
                headers: { 
                    "Content-Type": "application/json",
                    ...(token ? { "Authorization": `Bearer ${token}` } : {})
                },
                body: JSON.stringify({ card_id: card.id })
            });

            if (res.ok) {
                toast.success("天命已定！");
                goto(`/characters/${card.id}`);
            } else {
                toast.error("确认失败");
            }
        } catch(e) {
             toast.error("网络错误");
        }
    }

    // Card Back Design - "Ethereal Geometry" (Lighter Purple / Line Art)
    const CardBack = `
    <svg width="100%" height="100%" viewBox="0 0 240 360" xmlns="http://www.w3.org/2000/svg">
        <defs>
            <!-- Background Gradient: Soft Mystery -->
            <linearGradient id="softPurpleGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" style="stop-color:#a78bfa;stop-opacity:1" /> <!-- Light Violet -->
                <stop offset="50%" style="stop-color:#7c3aed;stop-opacity:1" /> <!-- Violet 600 -->
                <stop offset="100%" style="stop-color:#4c1d95;stop-opacity:1" /> <!-- Violet 900 -->
            </linearGradient>

            <!-- Pattern: Delicate Hexagons -->
            <pattern id="hexPattern" x="0" y="0" width="20" height="34.6" patternUnits="userSpaceOnUse">
                <path d="M 10 0 L 20 5.77 L 20 17.32 L 10 23.09 L 0 17.32 L 0 5.77 Z" fill="none" stroke="#ddd6fe" stroke-width="0.5" stroke-opacity="0.15"/>
            </pattern>
        </defs>
        
        <!-- Base Background -->
        <rect width="240" height="360" rx="16" ry="16" fill="url(#softPurpleGrad)" />
        <rect width="240" height="360" rx="16" ry="16" fill="url(#hexPattern)" />
        
        <!-- --- Frames --- -->
        <!-- Outer White Line -->
        <rect x="12" y="12" width="216" height="336" rx="10" fill="none" stroke="#fff" stroke-width="1.5" stroke-opacity="0.6" />
        <!-- Inner Gold/Purple Line -->
        <rect x="20" y="20" width="200" height="320" rx="6" fill="none" stroke="#e9d5ff" stroke-width="0.8" stroke-opacity="0.4" />

        <!-- --- Corner Geometry --- -->
        <g stroke="#fff" stroke-width="1" fill="none" class="corners">
            <!-- Diamond Shapes -->
            <path d="M 12 40 L 40 12 M 228 40 L 200 12 M 12 320 L 40 348 M 228 320 L 200 348" opacity="0.5"/>
            <!-- Circles -->
            <circle cx="28" cy="28" r="3" fill="#fff" opacity="0.8"/>
            <circle cx="212" cy="28" r="3" fill="#fff" opacity="0.8"/>
            <circle cx="28" cy="332" r="3" fill="#fff" opacity="0.8"/>
            <circle cx="212" cy="332" r="3" fill="#fff" opacity="0.8"/>
        </g>

        <!-- --- Central Geometric Mandala --- -->
        <g transform="translate(120, 180)">
             <!-- Rotating Squares -->
             <g stroke="#fff" stroke-width="0.8" fill="none" opacity="0.7">
                 <rect x="-60" y="-60" width="120" height="120" rx="2" transform="rotate(45)" />
                 <rect x="-50" y="-50" width="100" height="100" rx="2" transform="rotate(22.5)" stroke-opacity="0.5"/>
                 <rect x="-50" y="-50" width="100" height="100" rx="2" transform="rotate(67.5)" stroke-opacity="0.5"/>
             </g>

             <!-- Concentric Circles -->
             <circle r="30" fill="none" stroke="#e9d5ff" stroke-width="1.5" />
             <circle r="15" fill="none" stroke="#e9d5ff" stroke-width="1" stroke-dasharray="2 2"/>
             
             <!-- Center Star/Diamond -->
             <path d="M 0 -22 L 14 0 L 0 22 L -14 0 Z" fill="#e9d5ff" opacity="0.8" />
             
             <!-- Radiating Lines -->
             <g stroke="#fff" stroke-width="0.5" opacity="0.4">
                 <line x1="0" y1="-70" x2="0" y2="-90" />
                 <line x1="0" y1="70" x2="0" y2="90" />
                 <line x1="-70" y1="0" x2="-90" y2="0" />
                 <line x1="70" y1="0" x2="90" y2="0" />
                 
                 <line x1="50" y1="-50" x2="65" y2="-65" />
                 <line x1="-50" y1="-50" x2="-65" y2="-65" />
                 <line x1="50" y1="50" x2="65" y2="65" />
                 <line x1="-50" y1="50" x2="-65" y2="65" />
             </g>
        </g>
        
        <!-- Top/Bottom Axis Accents -->
        <path d="M 120 40 L 120 80" stroke="#fff" stroke-width="0.5" opacity="0.3" stroke-dasharray="4 4" />
        <path d="M 120 280 L 120 320" stroke="#fff" stroke-width="0.5" opacity="0.3" stroke-dasharray="4 4" />
        
        <circle cx="120" cy="50" r="4" fill="none" stroke="#fff" stroke-width="1" opacity="0.8"/>
        <circle cx="120" cy="310" r="4" fill="none" stroke="#fff" stroke-width="1" opacity="0.8"/>

    </svg>
    `;
</script>

<div class="container max-w-5xl mx-auto py-12 flex flex-col items-center justify-center min-h-[80vh] space-y-12">
    
    <!-- Cards Container -->
    <div class="relative w-full h-[450px] sm:h-[550px] flex items-center justify-center perspective-1000">
        
        {#if phase === 'ready'}
             <!-- Stacked Cards (动态数量) -->
             {#if displayCardCount === 0}
                 <div class="text-center text-muted-foreground py-20">
                     <p class="text-lg">还没有角色卡可供抽取</p>
                     <p class="text-sm mt-2">去角色库创建角色后再来抽卡吧！</p>
                 </div>
             {:else}
                 <div class="relative w-56 sm:w-64 h-80 sm:h-96"
                      style="cursor: url(&quot;data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32' viewBox='0 0 24 24' fill='%238b5cf6' stroke='white' stroke-width='2'><path d='M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z'/></svg>&quot;) 16 16, auto;">
                     {#each Array(displayCardCount) as _, i}
                         <div class="absolute inset-0 rounded-xl overflow-hidden shadow-2xl border border-white/10" 
                              style="transform: rotate({i * 9 - (displayCardCount - 1) * 4.5}deg) translate({i * 12 - (displayCardCount - 1) * 6}px, -{i * 4}px); z-index: {i}; box-shadow: 0 10px 30px rgba(0,0,0,0.5);">
                             {@html CardBack}
                         </div>
                     {/each}
                 </div>
             {/if}

        {:else if phase === 'shuffling'}
             <!-- Shuffling Animation (动态数量) -->
              {#each Array(displayCardCount) as _, i}
                 <div class="absolute w-56 sm:w-64 h-80 sm:h-96 rounded-xl overflow-hidden shadow-2xl border border-white/10 animate-shuffle-{i}"
                      style="z-index: {10 - i};">
                     {@html CardBack}
                 </div>
              {/each}

        {:else if phase === 'spreading'}
             <!-- Spreading Animation: Cards move from stacked to spread positions -->
             <div class="flex gap-4 sm:gap-8 items-center justify-center w-full">
                 {#each cards as card, i}
                     <div class="relative w-56 sm:w-64 h-80 sm:h-96 rounded-xl overflow-hidden shadow-2xl border border-white/10 animate-spread-{i}">
                         {@html CardBack}
                     </div>
                 {/each}
             </div>

        {:else}
              <!-- Picking / Revealed -->
              <div class="flex gap-4 sm:gap-8 items-center justify-center w-full">
                  {#each cards as card, i}
                      <!-- Wrapper -->
                      <!-- Transition logic: Spread from center -->
                      <!-- Left card (i=0): Flies from right (+150px) -->
                      <!-- Right card (i=2): Flies from left (-150px) -->
                      <!-- Center card (i=1): Fades in or scales -->
                      
                      <button 
                           class="relative w-56 sm:w-64 h-80 sm:h-96 perspective-1000 group outline-none focus:outline-none transition-transform duration-300 {phase === 'picking' ? 'hover:-translate-y-6 hover:shadow-purple-500/30' : ''}"
                           onclick={() => selectCard(i)}
                           style="cursor: url(&quot;data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32' viewBox='0 0 24 24' fill='%238b5cf6' stroke='white' stroke-width='2'><path d='M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z'/></svg>&quot;) 16 16, auto;"
                      >
                          
                          <!-- Card Inner (Flip Logic) -->
                          <div class="w-full h-full relative preserve-3d transition-all duration-700"
                               class:rotate-y-180={selectedIndex === i}>
                              
                              <!-- Front (Back of card actually) -->
                              <div class="absolute inset-0 backface-hidden rounded-xl overflow-hidden shadow-2xl border border-white/10">
                                  {@html CardBack}
                              </div>

                              <!-- Back (Content) -->
                              <div class="absolute inset-0 backface-hidden rotate-y-180 rounded-xl overflow-hidden shadow-xl bg-card border border-border flex flex-col items-center bg-zinc-900">
                                  <!-- Image -->
                                  <div class="w-full h-full relative group">
                                      <img src={resolveUrl(card.avatar)} alt={card.name} class="w-full h-full object-cover" />
                                      <div class="absolute inset-0 bg-gradient-to-t from-black/90 via-black/20 to-transparent"></div>
                                      
                                      <div class="absolute bottom-0 left-0 right-0 p-6 text-center transform translate-y-2 group-hover:translate-y-0 transition-transform">
                                          <h3 class="text-white text-2xl font-bold drop-shadow-lg mb-1">{card.name}</h3>
                                          {#if card.description}
                                              <p class="text-white/70 text-xs line-clamp-2 opacity-0 group-hover:opacity-100 transition-opacity delay-100">
                                                  {card.description}
                                              </p>
                                          {/if}
                                      </div>
                                  </div>
                              </div>
                          </div>

                          {#if phase === 'revealed' && selectedIndex === i}
                               <div class="absolute -inset-4 rounded-[20px] ring-2 ring-primary/50 ring-offset-2 ring-offset-background animate-pulse pointer-events-none z-20"></div>
                          {/if}
                      </button>
                  {/each}
              </div>
        {/if}
    </div>

    <!-- Controls -->
    <div class="w-full max-w-md space-y-4 z-20 min-h-[60px]">
        {#if phase === 'ready'}
            {#if displayCardCount === 0}
                <Button size="lg" class="w-full h-14 text-lg font-bold" href="/characters">
                    去创建角色
                </Button>
            {:else}
                <Button size="lg" class="w-full h-14 text-lg font-bold shadow-xl animate-bounce-slow bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-700 hover:to-indigo-700 border-0" onclick={startGacha}>
                    <Sparkles class="mr-2 h-5 w-5" /> 我命其实还由天，抽卡！
                </Button>
            {/if}
        {:else if phase === 'shuffling'}
             <Button disabled size="lg" class="w-full h-14 text-lg font-bold bg-muted/50 text-muted-foreground backdrop-blur-sm">
                 <RotateCcw class="mr-2 h-5 w-5 animate-spin" /> 命运洗牌中...
             </Button>
        {:else if phase === 'picking'}
             <div in:fade class="text-center p-4 bg-background/50 backdrop-blur-md rounded-full border border-white/10 shadow-lg">
                 <p class="text-lg font-medium text-foreground">请选一张牌</p>
             </div>
        {:else if phase === 'revealed'}
             <div class="flex gap-4 animate-in slide-in-from-bottom-4 duration-500">
                 <Button variant="outline" size="lg" class="flex-1 h-12" onclick={resetRound} disabled={remaining <= 0}>
                     再次抽卡 ({remaining}/3)
                 </Button>
                 
                 <Button variant="default" size="lg" class="flex-1 h-12 font-bold bg-gradient-to-r from-primary to-purple-600 text-white shadow-lg hover:scale-105 transition-transform" onclick={confirmSelection}>
                     <Check class="mr-2 h-5 w-5" /> 就翻你牌子了
                 </Button>
             </div>
        {/if}
    </div>

</div>

<style>
    /* CSS Utility Classes for 3D Transform */
    .perspective-1000 {
        perspective: 1000px;
    }
    .preserve-3d {
        transform-style: preserve-3d;
    }
    .backface-hidden {
        backface-visibility: hidden;
        -webkit-backface-visibility: hidden; 
    }
    .rotate-y-180 {
        transform: rotateY(180deg);
    }
    
    /* Shuffle Animations - 3 Seconds, Wide Amplitude */
    @keyframes shuffle-0 {
        0% { transform: translate(0,0) rotate(-2deg); }
        25% { transform: translate(-150px, 0) rotate(-15deg); z-index: 1; }
        50% { transform: translate(150px, 0) rotate(15deg); z-index: 3; }
        75% { transform: translate(0, 0) rotate(0deg); z-index: 2; }
        100% { transform: translate(0, 0) rotate(0deg); }
    }
    @keyframes shuffle-1 {
        0% { transform: translate(0,0); }
        25% { transform: translate(150px, 0) rotate(15deg); z-index: 3; }
        50% { transform: translate(-150px, 0) rotate(-15deg); z-index: 1; }
        75% { transform: translate(0, -20px) scale(1.05); z-index: 5; }
        100% { transform: translate(0, 0); }
    }
    @keyframes shuffle-2 {
        0% { transform: translate(0,0) rotate(2deg); }
        33% { transform: translate(0, -100px) rotate(10deg); z-index: 5; }
        66% { transform: translate(0, 100px) rotate(-10deg); z-index: 1; }
        100% { transform: translate(0, 0); }
    }

    .animate-shuffle-0 { animation: shuffle-0 2s ease-in-out infinite; }
    .animate-shuffle-1 { animation: shuffle-1 2s ease-in-out infinite; }
    .animate-shuffle-2 { animation: shuffle-2 2s ease-in-out infinite; }

    /* Spread Animations - Cards move from center stack to spread positions */
    @keyframes spread-0 {
        0% {
            transform: translateX(200px) rotate(0deg) scale(0.95);
            opacity: 0.8;
        }
        100% {
            transform: translateX(0) rotate(0deg) scale(1);
            opacity: 1;
        }
    }
    @keyframes spread-1 {
        0% {
            transform: translateY(-20px) scale(1.05);
            opacity: 0.8;
        }
        100% {
            transform: translateY(0) scale(1);
            opacity: 1;
        }
    }
    @keyframes spread-2 {
        0% {
            transform: translateX(-200px) rotate(0deg) scale(0.95);
            opacity: 0.8;
        }
        100% {
            transform: translateX(0) rotate(0deg) scale(1);
            opacity: 1;
        }
    }

    .animate-spread-0 { animation: spread-0 1.2s cubic-bezier(0.22, 1, 0.36, 1) forwards; }
    .animate-spread-1 { animation: spread-1 1.2s cubic-bezier(0.22, 1, 0.36, 1) 0.15s forwards; }
    .animate-spread-2 { animation: spread-2 1.2s cubic-bezier(0.22, 1, 0.36, 1) 0.3s forwards; }

    .animate-bounce-slow {
        animation: bounce 3s infinite;
    }
</style>
