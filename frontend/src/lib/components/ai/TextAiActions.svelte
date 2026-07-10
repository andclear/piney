<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import { Sparkles, Languages, Loader2 } from "lucide-svelte";
    import { AiService } from "$lib/ai/service";
    import { AiFeature } from "$lib/ai/types";
    import AiDiffDialog from "./AiDiffDialog.svelte";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";

    let {
        value = $bindable(""),
        feature,
        class: className
    } = $props<{
        value: string;
        feature: AiFeature;
        class?: string;
    }>();

    let isLoading = $state(false);
    let activeAction = $state<"optimize" | "translate" | null>(null);
    let showDiff = $state(false);
    let resultText = $state("");
    let dialogTitle = $state("");

    async function handleAction(action: "optimize" | "translate") {
        if (!value.trim()) {
            toast.error("即无内容，何须优化/翻译？");
            return;
        }

        isLoading = true;
        activeAction = action;
        
        try {
            const targetFeature = action === "optimize" ? feature : AiFeature.TRANSLATE;
            const res = await AiService.processText(targetFeature, value);
            
            resultText = res.content;
            dialogTitle = action === "optimize" ? "AI 润色优化建议" : "AI 翻译建议";
            showDiff = true;
            if (res.truncated) toast.warning("AI 输出达到上限，已保留生成内容，请检查结尾");
        } catch (e: any) {
            console.error(e);
            toast.error("AI 请求失败: " + e.message);
        } finally {
            isLoading = false;
            activeAction = null;
        }
    }

    function handleApply() {
        value = resultText;
        toast.success("内容已应用 (记得保存哦)");
    }
</script>

<div class={cn("flex items-center gap-2 bg-background/50 backdrop-blur-sm rounded-lg px-2 py-0.5 shadow-sm border border-border/10", className)}>
    <Button
        variant="ghost"
        size="icon"
        class="h-5 w-5 text-primary hover:bg-primary/10 transition-colors rounded-sm"
        title="AI 润色优化"
        disabled={isLoading}
        onclick={() => handleAction("optimize")}
    >
        {#if isLoading && activeAction === "optimize"}
            <Loader2 class="h-3 w-3 animate-spin" />
        {:else}
            <Sparkles class="h-3 w-3" />
        {/if}
    </Button>

    <Button
        variant="ghost"
        size="icon"
        class="h-5 w-5 text-primary hover:bg-primary/10 transition-colors rounded-sm"
        title="AI 翻译 (中文)"
        disabled={isLoading}
        onclick={() => handleAction("translate")}
    >
        {#if isLoading && activeAction === "translate"}
             <Loader2 class="h-3 w-3 animate-spin" />
        {:else}
             <Languages class="h-3 w-3" />
        {/if}
    </Button>
</div>

<AiDiffDialog
    bind:open={showDiff}
    title={dialogTitle}
    original={value}
    optimized={resultText}
    onApply={handleApply}
    onCancel={() => {}}
/>
