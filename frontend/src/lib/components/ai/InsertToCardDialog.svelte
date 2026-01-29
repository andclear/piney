<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import { Search, Loader2, User, Check } from "lucide-svelte";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { API_BASE } from "$lib/api";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";

    let { 
        open = $bindable(false),
        onConfirm = (card: any) => {} 
    } = $props<{
        open: boolean,
        onConfirm: (card: any) => void
    }>();

    let searchTerm = $state("");
    let loading = $state(false);
    let cards: any[] = $state([]);
    let selectedCard: any = $state(null);

    async function fetchCards() {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const params = new URLSearchParams({
                sort: "updated_at",
                order: "desc",
                page_size: "50", // Fetch enough recent cards
                search: searchTerm
            });
            
            const res = await fetch(`${API_BASE}/api/cards?${params.toString()}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {}
            });
            
            if (!res.ok) throw new Error("获取角色列表失败");
            
            const data = await res.json();
            cards = data.items || [];
        } catch (e) {
            console.error(e);
            toast.error("加载角色列表失败");
        } finally {
            loading = false;
        }
    }

    // Debounce search
    let timer: any;
    $effect(() => {
        if (open) {
            fetchCards(); // Initial load
            selectedCard = null;
        }
    });

    function handleSearch() {
        clearTimeout(timer);
        timer = setTimeout(() => {
            fetchCards();
        }, 300);
    }

    function handleConfirm() {
        if (!selectedCard) return;
        onConfirm(selectedCard);
        open = false;
    }
</script>

<Dialog.Root bind:open={open}>
    <Dialog.Content class="sm:max-w-[500px] flex flex-col max-h-[85vh] overflow-hidden">
        <Dialog.Header>
            <Dialog.Title>选择目标角色卡</Dialog.Title>
            <Dialog.Description>
                将生成的正则和世界书条目插入到选中的角色卡中。
            </Dialog.Description>
        </Dialog.Header>

        <div class="flex items-center space-x-2 py-4 border-b">
            <Search class="h-4 w-4 text-muted-foreground" />
            <Input 
                placeholder="搜索角色名称..." 
                class="flex-1 border-none bg-transparent focus-visible:ring-0 px-0"
                bind:value={searchTerm}
                oninput={handleSearch}
            />
        </div>

        <div class="flex-1 min-h-0 overflow-y-auto -mx-6 px-6">
            {#if loading}
                <div class="flex justify-center py-8">
                    <Loader2 class="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
            {:else if cards.length === 0}
                <div class="text-center py-8 text-muted-foreground text-sm">
                    未找到匹配的角色卡
                </div>
            {:else}
                <div class="space-y-2 py-2">
                    {#each cards as card (card.id)}
                        <button
                            class={cn(
                                "w-full flex items-center p-3 rounded-lg border text-left transition-all hover:bg-accent",
                                selectedCard?.id === card.id ? "border-primary bg-primary/5 ring-1 ring-primary" : "border-transparent"
                            )}
                            onclick={() => selectedCard = card}
                        >
                            {#if card.avatar}
                                <img src={resolveUrl(card.avatar)} alt="" class="h-10 w-10 rounded-md object-cover bg-muted mr-3" />
                            {:else}
                                <div class="h-10 w-10 rounded-md bg-muted flex items-center justify-center mr-3">
                                    <User class="h-5 w-5 text-muted-foreground" />
                                </div>
                            {/if}
                            <div class="flex-1 min-w-0">
                                <div class="font-medium truncate">{card.name}</div>
                                <div class="text-xs text-muted-foreground truncate">
                                    {new Date(card.updated_at || card.created_at).toLocaleString()}
                                </div>
                            </div>
                            {#if selectedCard?.id === card.id}
                                <Check class="h-4 w-4 text-primary ml-2" />
                            {/if}
                        </button>
                    {/each}
                </div>
            {/if}
        </div>

        <Dialog.Footer class="pt-4 border-t mt-auto">
            <Button variant="outline" onclick={() => open = false}>取消</Button>
            <Button onclick={handleConfirm} disabled={!selectedCard}>
                一键插入到角色卡
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>

<script module>
    import { resolveUrl } from "$lib/api";
</script>
