<script lang="ts">
    import { onMount } from "svelte";
    import { API_BASE } from "$lib/api";
    import { AiService } from "$lib/ai/service";
    import { Button } from "$lib/components/ui/button";
    import {
        Select,
        SelectContent,
        SelectItem,
        SelectTrigger,
        SelectValue,
    } from "$lib/components/ui/select";
    import { Textarea } from "$lib/components/ui/textarea";
    import { Label } from "$lib/components/ui/label";
    import * as Card from "$lib/components/ui/card";
    import { toast } from "svelte-sonner";
    import { Loader2, Bug, Play } from "lucide-svelte";

    let cards: any[] = [];
    let selectedCardId: string = "";
    let selectedCard: any = null;

    let systemPrompt = "";
    let userPrompt = "";
    let variables: any = {};

    let isLoading = false;
    let aiResponse: any = null;
    let parsedResult: any = null;
    let logs: string[] = [];

    onMount(async () => {
        await loadCards();
    });

    async function loadCards() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                const data = await res.json();
                cards = Array.isArray(data) ? data : (data.items || []);
            }
        } catch (e) {
            console.error(e);
            toast.error("加载角色卡失败");
        }
    }

    async function handleCardSelect(id: string) {
        selectedCardId = id;
        if (!id) {
            selectedCard = null;
            return;
        }
        
        // Fetch full details because list API doesn't return 'data' blob
        try {
            isLoading = true;
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${id}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                selectedCard = await res.json();
                await updateDebugInfo();
                // Reset results
                aiResponse = null;
                parsedResult = null;
            } else {
                toast.error("加载角色详情失败");
            }
        } catch (e) {
            console.error(e);
            toast.error("加载角色详情失败");
        } finally {
            isLoading = false;
        }
    }

    async function updateDebugInfo() {
        if (!selectedCard) return;
        const debugInfo = await AiService.getPromptDebugInfo(selectedCard);
        systemPrompt = debugInfo.systemPrompt;
        userPrompt = debugInfo.userPrompt;
        variables = debugInfo.variables;
    }

    async function runOverview() {
        if (!selectedCard) return;
        isLoading = true;
        aiResponse = null;
        parsedResult = null;
        logs = [];

        try {

            const feature = "overview"; // AiFeature.OVERVIEW
            const messages = [
                { role: "system", content: systemPrompt },
                { role: "user", content: userPrompt }
            ];

            // Manually execute to capture raw response
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/ai/execute`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    ...(token ? { 'Authorization': `Bearer ${token}` } : {})
                },
                body: JSON.stringify({
                    feature_id: feature,
                    messages
                })
            });

            if(!res.ok) {
                 const err = await res.json();
                 throw new Error(err.error || "Execute Failed");
            }
            
            const raw = await res.json();
            aiResponse = raw;

            // Now parse it manually to show "Parsed Result"
            const content = raw.choices?.[0]?.message?.content;
            if(content) {
                const cleaned = content.replace(/```json/g, '').replace(/```/g, '').trim();
                parsedResult = JSON.parse(cleaned);
            }

        } catch (e) {
            console.error(e);
            toast.error("执行失败: " + String(e));
        } finally {
            isLoading = false;
        }
    }
</script>

<div class="container mx-auto py-8 space-y-8 max-w-5xl">
    <div class="flex items-center gap-4 border-b pb-4">
        <div class="p-2 bg-primary/10 rounded-lg">
            <Bug class="h-6 w-6 text-primary" />
        </div>
        <div>
            <h1 class="text-2xl font-bold">AI 功能调试台</h1>
            <p class="text-muted-foreground text-sm">测试 AI 概览生成、查看 Prompt 构建结果及原始响应</p>
        </div>
    </div>

    <!-- Controls -->
    <Card.Root>
        <Card.Header>
            <Card.Title>测试配置</Card.Title>
        </Card.Header>
        <Card.Content class="flex items-end gap-4">
             <div class="flex-1 space-y-2">
                <Label>选择角色卡</Label>
                <select 
                    class="flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                    onchange={(e) => handleCardSelect(e.currentTarget.value)}
                >
                    <option value="">-- 请选择角色 --</option>
                    {#each cards as c}
                        <option value={c.id}>{c.name}</option>
                    {/each}
                </select>
             </div>
             
             <Button disabled={!selectedCard || isLoading} onclick={runOverview} class="gap-2">
                {#if isLoading}
                    <Loader2 class="h-4 w-4 animate-spin" /> 生成中...
                {:else}
                    <Play class="h-4 w-4" /> 执行测试
                {/if}
             </Button>
        </Card.Content>
    </Card.Root>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- Prompts -->
        <div class="space-y-6">
             <div class="space-y-2">
                <Label>System Prompt (系统提示词)</Label>
                <Textarea value={systemPrompt} readonly class="min-h-[150px] font-mono text-xs bg-muted" />
             </div>

             <div class="space-y-2">
                <Label>User Prompt (构建结果)</Label>
                <Textarea value={userPrompt} readonly class="min-h-[400px] font-mono text-xs bg-muted" />
             </div>
              
               <div class="space-y-2">
                <Label>Variables (变量)</Label>
                <pre class="p-4 bg-muted rounded-md overflow-auto max-h-[200px] text-xs font-mono">{JSON.stringify(variables, null, 2)}</pre>
             </div>
        </div>

        <!-- Results -->
        <div class="space-y-6">
             <div class="space-y-2">
                 <Label>Parsed Result (解析结果)</Label>
                 <div class="p-4 rounded-md border bg-card text-card-foreground min-h-[150px]">
                    {#if parsedResult}
                        <div class="space-y-2">
                            <p class="text-sm font-medium text-muted-foreground">Summary:</p>
                            <p class="text-sm leading-relaxed">{parsedResult.summary}</p>
                            {#if parsedResult.tags}
                                <div class="flex gap-2 flex-wrap mt-2">
                                    {#each parsedResult.tags as tag}
                                        <span class="px-2 py-1 bg-secondary rounded text-xs">{tag}</span>
                                    {/each}
                                </div>
                            {/if}
                        </div>
                    {:else}
                        <span class="text-muted-foreground text-sm">等待执行...</span>
                    {/if}
                 </div>
             </div>

             <div class="space-y-2">
                <Label>Raw AI Response (原始 JSON)</Label>
                <pre class="p-4 rounded-md border bg-muted overflow-auto max-h-[600px] text-xs font-mono whitespace-pre-wrap">{aiResponse ? JSON.stringify(aiResponse, null, 2) : '(空)'}</pre>
             </div>
        </div>
    </div>
</div>
