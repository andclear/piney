<script lang="ts">
    // 新建/编辑小剧场对话框
    import * as Dialog from "$lib/components/ui/dialog";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Textarea } from "$lib/components/ui/textarea";
    import { Label } from "$lib/components/ui/label";
    import { Loader2 } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import { API_BASE } from "$lib/api";
    import * as Command from "$lib/components/ui/command";
    import * as Popover from "$lib/components/ui/popover";

    let { 
        open = $bindable(false), 
        theater = null,  // 编辑时传入
        categories = [],
        onSuccess 
    } = $props();

    let title = $state("");
    let desc = $state("");
    let content = $state("");
    let category = $state("");
    let isProcessing = $state(false);
    let categoryOpen = $state(false);

    $effect(() => {
        if (open) {
            if (theater) {
                // 编辑模式
                title = theater.title || "";
                desc = theater.desc || "";
                content = theater.content || "";
                category = theater.category || "";
            } else {
                // 新建模式
                title = "";
                desc = "";
                content = "";
                category = "";
            }
        }
    });

    async function handleSubmit() {
        if (!title.trim()) {
            toast.error("标题不能为空");
            return;
        }
        if (!desc.trim()) {
            toast.error("简介不能为空");
            return;
        }
        if (!content.trim()) {
            toast.error("内容不能为空");
            return;
        }

        isProcessing = true;
        try {
            const token = localStorage.getItem("auth_token");
            const payload = {
                title: title.trim(),
                desc: desc.trim(),
                content: content.trim(),
                category: category.trim() || "未分类",
            };

            const url = theater 
                ? `${API_BASE}/api/theaters/${theater.id}`
                : `${API_BASE}/api/theaters`;
            
            const res = await fetch(url, {
                method: theater ? "PATCH" : "POST",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify(payload),
            });

            if (!res.ok) {
                const errText = await res.text();
                throw new Error(errText || "操作失败");
            }

            toast.success(theater ? "更新成功" : "创建成功");
            onSuccess?.();
            open = false;
        } catch (e: any) {
            console.error(e);
            toast.error(e.message || "操作失败");
        } finally {
            isProcessing = false;
        }
    }
</script>

<Dialog.Root bind:open={open}>
    <Dialog.Content class="sm:max-w-[600px] max-h-[90vh] flex flex-col">
        <Dialog.Header>
            <Dialog.Title>{theater ? "编辑小剧场" : "新建小剧场"}</Dialog.Title>
        </Dialog.Header>

        <div class="flex-1 overflow-y-auto py-4 space-y-4 px-1">
            <!-- 标题 -->
            <div class="space-y-2">
                <Label for="theater-title">标题 <span class="text-destructive">*</span></Label>
                <Input 
                    id="theater-title"
                    bind:value={title} 
                    placeholder="请输入小剧场标题" 
                />
            </div>

            <!-- 分类 -->
            <div class="space-y-2">
                <Label>分类</Label>
                <Popover.Root bind:open={categoryOpen}>
                    <Popover.Trigger asChild>
                        {#snippet child({ props })}
                            <Button variant="outline" class="w-full justify-start" {...props}>
                                {category || "选择分类..."}
                            </Button>
                        {/snippet}
                    </Popover.Trigger>
                    <Popover.Content class="w-[300px] p-2" align="start">
                        <div class="pb-2 mb-2 border-b">
                            <Input 
                                placeholder="输入新分类名..." 
                                bind:value={category}
                                class="h-8 text-sm"
                            />
                        </div>
                        <div class="space-y-1 max-h-[200px] overflow-y-auto">
                            {#each categories as cat}
                                <button
                                    class="w-full px-3 py-2 text-left text-sm rounded-md hover:bg-accent flex items-center justify-between"
                                    onclick={() => { category = cat; categoryOpen = false; }}
                                >
                                    <span>{cat}</span>
                                    {#if category === cat}
                                        <span class="text-primary">✓</span>
                                    {/if}
                                </button>
                            {/each}
                            {#if categories.length === 0}
                                <div class="px-3 py-2 text-sm text-muted-foreground">暂无分类</div>
                            {/if}
                        </div>
                    </Popover.Content>
                </Popover.Root>
                <p class="text-xs text-muted-foreground">留空则使用"未分类"</p>
            </div>

            <!-- 简介 -->
            <div class="space-y-2">
                <Label for="theater-desc">简介 <span class="text-destructive">*</span></Label>
                <Textarea 
                    id="theater-desc"
                    bind:value={desc} 
                    placeholder="请输入简介" 
                    rows={2}
                />
            </div>

            <!-- 内容 -->
            <div class="space-y-2">
                <Label for="theater-content">内容 <span class="text-destructive">*</span></Label>
                <Textarea 
                    id="theater-content"
                    bind:value={content} 
                    placeholder="请输入小剧场内容（提示词）" 
                    rows={8}
                    class="font-mono text-sm"
                />
            </div>
        </div>

        <Dialog.Footer>
            <Button variant="outline" onclick={() => open = false} disabled={isProcessing}>
                取消
            </Button>
            <Button onclick={handleSubmit} disabled={isProcessing}>
                {#if isProcessing}
                    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                {/if}
                {theater ? "保存修改" : "创建"}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
