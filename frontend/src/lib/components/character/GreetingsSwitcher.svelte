<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import { Badge } from "$lib/components/ui/badge";
    import { Label } from "$lib/components/ui/label";
    import {
        ChevronLeft,
        ChevronRight,
        Plus,
        Trash2,
        Maximize2,
        MessageSquare,
    } from "lucide-svelte";
    import RichTextarea from "./RichTextarea.svelte";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";

    let {
        firstMes = $bindable(),
        alternateGreetings = $bindable([]),
        isDirty = false,
        class: className = undefined,
    } = $props();

    let currentIndex = $state(0);
    let totalCount = $derived(1 + (alternateGreetings?.length || 0));

    function next() {
        if (currentIndex < totalCount - 1) {
            currentIndex++;
        }
    }

    function prev() {
        if (currentIndex > 0) {
            currentIndex--;
        }
    }

    function addGreeting() {
        // Ensure alternateGreetings is initialized
        if (!alternateGreetings) alternateGreetings = [];
        alternateGreetings = [...alternateGreetings, ""];
        currentIndex = totalCount - 1; // Jump to new one
        toast.success("已添加新开场白");
    }

    function removeCurrent() {
        if (currentIndex === 0) {
            // Deleting main greeting
            if (alternateGreetings && alternateGreetings.length > 0) {
                // Determine which alternate becomes the new main
                // Strategy: The first alternate becomes the new First Message
                const newFirst = alternateGreetings[0];
                alternateGreetings.splice(0, 1);
                alternateGreetings = [...alternateGreetings];
                firstMes = newFirst;
                toast.success("已删除主开场白，备选开场白已晋升");
            } else {
                // No alternates, just clear it
                firstMes = "";
                toast.success("已清空主开场白");
            }
        } else {
            // Deleting alternate
            const altIndex = currentIndex - 1;
            alternateGreetings.splice(altIndex, 1);
            alternateGreetings = [...alternateGreetings]; // Trigger update
            currentIndex = Math.max(0, currentIndex - 1);
            toast.success("已删除开场白");
        }
    }

    let isZenMode = $state(false);
</script>

<div
    class={cn(
        "space-y-4 border rounded-xl p-3 md:p-4 bg-muted/20 relative transition-colors duration-300",
        className,
        isDirty && "border border-amber-500/50 bg-amber-500/5",
    )}
>
    {#if isDirty}
        <span
            class="absolute -right-1 -top-1 w-2 h-2 rounded-full bg-amber-500 z-10 shadow-sm"
            title="未保存的更改"
        ></span>
    {/if}
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
            <MessageSquare class="h-4 w-4 text-muted-foreground" />
            <Label>
                {#if currentIndex === 0}
                    开场白 (主)
                {:else}
                    开场白 (备选 {currentIndex})
                {/if}
            </Label>
            <Badge variant="outline" class="text-[10px] h-5 px-1.5">
                {currentIndex + 1} / {totalCount}
            </Badge>
        </div>

        <div class="flex items-center gap-1">
            <Button
                variant="ghost"
                size="icon"
                class="h-7 w-7"
                disabled={currentIndex === 0}
                onclick={prev}
            >
                <ChevronLeft class="h-4 w-4" />
            </Button>

            <Button
                variant="ghost"
                size="icon"
                class="h-7 w-7"
                disabled={currentIndex === totalCount - 1}
                onclick={next}
            >
                <ChevronRight class="h-4 w-4" />
            </Button>

            <div class="w-px h-4 bg-border mx-1"></div>

            <Button
                variant="ghost"
                size="icon"
                class="h-7 w-7 text-muted-foreground hover:text-foreground"
                onclick={addGreeting}
                title="添加开场白"
            >
                <Plus class="h-4 w-4" />
            </Button>

            <Button
                variant="ghost"
                size="icon"
                class="h-7 w-7 text-muted-foreground hover:text-destructive"
                onclick={removeCurrent}
                title="删除当前"
            >
                <Trash2 class="h-4 w-4" />
            </Button>

            <div class="w-px h-4 bg-border mx-1"></div>

            <Button
                variant="ghost"
                size="sm"
                class="h-7 text-[10px] px-2 text-muted-foreground hover:text-primary"
                onclick={() => (isZenMode = true)}
            >
                <Maximize2 class="mr-1 h-3 w-3" /> 全屏编辑
            </Button>
        </div>
    </div>

    {#if currentIndex === 0}
        <RichTextarea
            bind:value={firstMes}
            placeholder="输入主开场白..."
            rows={5}
            class="animate-in fade-in slide-in-from-right-2 duration-300"
            bind:isZenMode
        />
    {:else if alternateGreetings && alternateGreetings[currentIndex - 1] !== undefined}
        <RichTextarea
            bind:value={alternateGreetings[currentIndex - 1]}
            placeholder="输入备选开场白..."
            rows={5}
            class="animate-in fade-in slide-in-from-right-2 duration-300"
            bind:isZenMode
        />
    {/if}
</div>
