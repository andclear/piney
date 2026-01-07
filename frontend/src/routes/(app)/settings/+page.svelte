<script lang="ts">
    import {
        Tabs,
        TabsContent,
        TabsList,
        TabsTrigger,
    } from "$lib/components/ui/tabs";
    import * as Card from "$lib/components/ui/card";
    import { Button } from "$lib/components/ui/button";
    import { Label } from "$lib/components/ui/label";
    import { Switch } from "$lib/components/ui/switch";
    import { Badge } from "$lib/components/ui/badge";
    import {
        Plus,
        Trash2,
        Edit2,
        Zap,
        Settings2,
        Box,
        BrainCircuit,
        CheckCircle2,
        XCircle,
        Ghost,
        Sparkles,
    } from "lucide-svelte";
    import { Textarea } from "$lib/components/ui/textarea";
    import { api } from "$lib/api";
    import { onMount } from "svelte";
    import { toast } from "svelte-sonner";
    import ChannelDialog from "$lib/components/settings/channel-dialog.svelte";
    import {
        Select,
        SelectContent,
        SelectItem,
        SelectTrigger,
        SelectValue,
    } from "$lib/components/ui/select";

    // Data Models
    interface AiChannel {
        id: string;
        name: string;
        base_url: string;
        model_id: string;
        is_active: boolean;
    }

    // State
    let channels = $state<AiChannel[]>([]);
    let isDialogOpen = $state(false);
    let editingChannel = $state<AiChannel | null>(null);
    let isLoading = $state(false);
    let isTestingAll = $state(false);

    // Default Assignments (Stored in settings)
    let configOverview = $state("");
    let configTranslation = $state("");
    let isSavingConfig = $state(false);

    // Prompt Configuration
    let globalPrompt = $state("");
    let isSavingPrompt = $state(false);

    // Test results per channel: { channelId: { success: boolean, latency_ms?: number, error?: string } }
    let channelTestResults = $state<
        Record<
            string,
            { success: boolean; latency_ms?: number; error?: string }
        >
    >({});

    // Initial Load
    onMount(async () => {
        await Promise.all([loadChannels(), loadSettings()]);
    });

    async function loadChannels() {
        isLoading = true;
        try {
            const res = await api.get<AiChannel[]>("/ai/channels");
            if (res.success && res.data) {
                channels = res.data;
            } else {
                toast.error("加载渠道失败", { description: res.error });
            }
        } catch (e) {
            toast.error("加载渠道错误", { description: String(e) });
        } finally {
            isLoading = false;
        }
    }

    async function loadSettings() {
        try {
            const res = await api.get<any>("/settings");
            if (res.success && res.data) {
                // Settings API returns a Settings object with specific fields
                // AI config is not yet in the backend Settings struct,
                // so these will be undefined for now (feature pending backend update)
                configOverview = res.data.ai_config_overview || "";
                configTranslation = res.data.ai_config_translation || "";
                globalPrompt = res.data.global_prompt || "";
            }
        } catch (e) {
            console.error("Failed to load settings", e);
        }
    }

    async function deleteChannel(id: string) {
        if (!confirm("确定要删除这个渠道吗？此操作不可恢复。")) return;
        try {
            const res = await api.delete(`/ai/channels/${id}`);
            if (res.success) {
                toast.success("删除成功");
                await loadChannels();
            } else {
                toast.error("删除失败", { description: res.error });
            }
        } catch (e) {
            toast.error("删除失败", { description: String(e) });
        }
    }

    async function testAllChannels() {
        if (channels.length === 0) return;
        isTestingAll = true;
        channelTestResults = {}; // 清空之前的结果
        try {
            const res = await api.post<any>("/ai/channels/test");
            if (res.success && Array.isArray(res.data)) {
                const results = res.data;
                const successCount = results.filter(
                    (r: any) => r.success,
                ).length;
                const failCount = results.length - successCount;

                // 存储每个渠道的测试结果
                const newResults: Record<
                    string,
                    { success: boolean; latency_ms?: number; error?: string }
                > = {};
                for (const r of results) {
                    newResults[r.id] = {
                        success: r.success,
                        latency_ms: r.latency_ms,
                        error: r.message,
                    };
                }
                channelTestResults = newResults;

                if (failCount === 0) {
                    toast.success(`所有 ${successCount} 个渠道测试通过`);
                } else {
                    toast.warning(
                        `测试完成: ${successCount} 通过, ${failCount} 失败`,
                    );
                    // Show detailed errors for failures
                    results
                        .filter((r: any) => !r.success)
                        .forEach((r: any) => {
                            toast.error(`${r.name}: ${r.message}`, {
                                duration: 5000,
                            });
                        });
                }
            } else {
                toast.error("批量测试失败", { description: res.error });
            }
        } catch (e) {
            toast.error("测试出错", { description: String(e) });
        } finally {
            isTestingAll = false;
        }
    }

    async function saveFeatureConfig() {
        isSavingConfig = true;
        try {
            // Update settings
            await api.patch("/settings", {
                ai_config_overview: configOverview,
                ai_config_translation: configTranslation,
            });
            toast.success("默认模型配置已保存");
        } catch (e) {
            toast.error("保存配置失败", { description: String(e) });
        } finally {
            isSavingConfig = false;
        }
    }
</script>

<div class="h-full flex flex-col gap-6 p-4 sm:p-6">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-3xl font-bold tracking-tight">系统设置</h1>
            <p class="text-muted-foreground mt-1">管理应用配置与 AI 接入。</p>
        </div>
    </div>

    <Tabs value="general" class="w-full p4">
        <TabsList class="mb-4 gap-4">
            <TabsTrigger value="general" class="gap-2">
                <Settings2 class="h-4 w-4" />
                通用设置
            </TabsTrigger>
            <TabsTrigger value="ai" class="gap-2">
                <Sparkles class="h-4 w-4" />
                AI 配置
            </TabsTrigger>
            <TabsTrigger value="prompts" class="gap-2">
                <Ghost class="h-4 w-4" />
                提示词配置
            </TabsTrigger>
        </TabsList>

        <!-- General Tab -->
        <TabsContent value="general">
            <Card.Root>
                <Card.Header>
                    <Card.Title>通用设置</Card.Title>
                    <Card.Description
                        >系统常规选项（开发中...）</Card.Description
                    >
                </Card.Header>
                <Card.Content
                    class="min-h-[200px] flex items-center justify-center text-muted-foreground border-dashed"
                >
                    更多设置项即将上线
                </Card.Content>
            </Card.Root>
        </TabsContent>

        <!-- AI Configuration Tab -->
        <TabsContent value="ai" class="space-y-6">
            <!-- Channel Management -->
            <Card.Root>
                <Card.Header>
                    <div
                        class="flex flex-col sm:flex-row sm:items-center justify-between gap-4"
                    >
                        <div>
                            <Card.Title>AI 渠道管理</Card.Title>
                            <Card.Description
                                >配置兼容 OpenAI 接口的 AI 模型服务。</Card.Description
                            >
                        </div>
                        <div class="flex gap-2 w-full sm:w-auto">
                            <Button
                                variant="outline"
                                size="sm"
                                onclick={testAllChannels}
                                disabled={isTestingAll || channels.length === 0}
                                class="flex-1 sm:flex-none"
                            >
                                <Zap class="h-4 w-4 mr-2" />
                                {isTestingAll ? "测试中..." : "一键检测可用性"}
                            </Button>
                            <Button
                                size="sm"
                                onclick={() => {
                                    editingChannel = null;
                                    isDialogOpen = true;
                                }}
                                class="flex-1 sm:flex-none"
                            >
                                <Plus class="h-4 w-4 mr-2" />
                                添加渠道
                            </Button>
                        </div>
                    </div>
                </Card.Header>
                <Card.Content>
                    {#if isLoading}
                        <div class="text-center py-10 text-muted-foreground">
                            加载中...
                        </div>
                    {:else if channels.length === 0}
                        <div
                            class="text-center py-12 border-2 border-dashed rounded-lg"
                        >
                            <BrainCircuit
                                class="h-10 w-10 mx-auto text-muted-foreground/30 mb-3"
                            />
                            <h3 class="text-lg font-medium">暂无 AI 渠道</h3>
                            <p class="text-muted-foreground text-sm mb-4">
                                请点击右上角添加您的第一个 AI 服务配置
                            </p>
                            <Button
                                variant="outline"
                                onclick={() => (isDialogOpen = true)}
                                >立即添加</Button
                            >
                        </div>
                    {:else}
                        <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                            {#each channels as channel}
                                <div
                                    class="relative group border rounded-lg p-4 hover:border-primary/50 transition-all bg-card text-card-foreground shadow-sm"
                                >
                                    <div
                                        class="flex justify-between items-start mb-2"
                                    >
                                        <div
                                            class="font-semibold flex items-center gap-2"
                                        >
                                            {channel.name}
                                            {#if !channel.is_active}
                                                <Badge
                                                    variant="secondary"
                                                    class="text-[10px] h-4"
                                                    >已禁用</Badge
                                                >
                                            {/if}
                                        </div>
                                        <div
                                            class="opacity-100 sm:opacity-0 sm:group-hover:opacity-100 transition-opacity flex gap-1"
                                        >
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                class="h-7 w-7"
                                                onclick={() => {
                                                    editingChannel = channel;
                                                    isDialogOpen = true;
                                                }}
                                            >
                                                <Edit2
                                                    class="h-3.5 w-3.5 text-muted-foreground"
                                                />
                                            </Button>
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                class="h-7 w-7"
                                                onclick={() =>
                                                    deleteChannel(channel.id)}
                                            >
                                                <Trash2
                                                    class="h-3.5 w-3.5 text-destructive"
                                                />
                                            </Button>
                                        </div>
                                    </div>
                                    <div
                                        class="text-xs text-muted-foreground space-y-1"
                                    >
                                        <div
                                            class="flex items-center gap-1.5 truncate"
                                            title={channel.base_url}
                                        >
                                            <span
                                                class="font-medium text-foreground/70"
                                                >URL:</span
                                            >
                                            {channel.base_url}
                                        </div>
                                        <div
                                            class="flex items-center gap-1.5 truncate"
                                        >
                                            <span
                                                class="font-medium text-foreground/70"
                                                >模型:</span
                                            >
                                            {channel.model_id}
                                        </div>
                                        <!-- 测试结果显示 -->
                                        {#if channelTestResults[channel.id]}
                                            {@const result =
                                                channelTestResults[channel.id]}
                                            <div
                                                class="flex items-center gap-1.5 mt-1"
                                            >
                                                {#if result.success}
                                                    <CheckCircle2
                                                        class="h-3.5 w-3.5 text-green-500"
                                                    />
                                                    <span
                                                        class="text-green-600"
                                                    >
                                                        可用 {result.latency_ms
                                                            ? `(${result.latency_ms}ms)`
                                                            : ""}
                                                    </span>
                                                {:else}
                                                    <XCircle
                                                        class="h-3.5 w-3.5 text-red-500"
                                                    />
                                                    <span
                                                        class="text-red-500 truncate"
                                                        title={result.error}
                                                    >
                                                        不可用
                                                    </span>
                                                {/if}
                                            </div>
                                        {/if}
                                    </div>
                                </div>
                            {/each}
                        </div>
                    {/if}
                </Card.Content>
            </Card.Root>

            <!-- Feature Defaults -->
            <Card.Root>
                <Card.Header>
                    <div
                        class="flex flex-col sm:flex-row sm:items-center justify-between gap-4"
                    >
                        <div>
                            <Card.Title>默认模型配置</Card.Title>
                            <Card.Description
                                >为系统功能指定默认使用的 AI 渠道。</Card.Description
                            >
                        </div>
                        <Button
                            size="sm"
                            onclick={saveFeatureConfig}
                            disabled={isSavingConfig}
                            class="w-full sm:w-auto"
                        >
                            {isSavingConfig ? "保存中..." : "保存配置"}
                        </Button>
                    </div>
                </Card.Header>
                <Card.Content class="space-y-6">
                    <div class="grid gap-6 md:grid-cols-2">
                        <!-- AI 智能概览 -->
                        <div class="space-y-2">
                            <Label for="config-overview">AI 智能概览</Label>
                            <p class="text-xs text-muted-foreground mb-2">
                                用于生成角色卡简介、标签推荐。
                            </p>
                            <Select type="single" bind:value={configOverview}>
                                <SelectTrigger class="w-full">
                                    {#if configOverview}
                                        {@const selected = channels.find(
                                            (c) => c.id === configOverview,
                                        )}
                                        <span class="truncate">
                                            {selected
                                                ? `${selected.name} (${selected.model_id})`
                                                : "选择渠道..."}
                                        </span>
                                    {:else}
                                        <span class="text-muted-foreground"
                                            >选择渠道...</span
                                        >
                                    {/if}
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="">未配置</SelectItem>
                                    {#each channels as c}
                                        <SelectItem
                                            value={c.id}
                                            disabled={!c.is_active}
                                        >
                                            {c.name} ({c.model_id})
                                        </SelectItem>
                                    {/each}
                                </SelectContent>
                            </Select>
                        </div>

                        <!-- AI 辅助翻译-->
                        <div class="space-y-2">
                            <Label for="config-translation">AI 辅助翻译</Label>
                            <p class="text-xs text-muted-foreground mb-2">
                                用于翻译角色卡内容、世界书条目等。
                            </p>
                            <Select
                                type="single"
                                bind:value={configTranslation}
                            >
                                <SelectTrigger class="w-full">
                                    {#if configTranslation}
                                        {@const selected = channels.find(
                                            (c) => c.id === configTranslation,
                                        )}
                                        <span class="truncate">
                                            {selected
                                                ? `${selected.name} (${selected.model_id})`
                                                : "选择渠道..."}
                                        </span>
                                    {:else}
                                        <span class="text-muted-foreground"
                                            >选择渠道...</span
                                        >
                                    {/if}
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="">未配置</SelectItem>
                                    {#each channels as c}
                                        <SelectItem
                                            value={c.id}
                                            disabled={!c.is_active}
                                        >
                                            {c.name} ({c.model_id})
                                        </SelectItem>
                                    {/each}
                                </SelectContent>
                            </Select>
                        </div>
                    </div>
                </Card.Content>
            </Card.Root>
        </TabsContent>

        <!-- Prompts Configuration Tab -->
        <TabsContent value="prompts" class="space-y-6">
            <Card.Root>
                <Card.Header>
                    <div
                        class="flex flex-col sm:flex-row sm:items-center justify-between gap-4"
                    >
                        <div>
                            <Card.Title>破限提示词</Card.Title>
                            <Card.Description class="mt-2"
                                >会附在所有 AI 调用的 Prompt 前面。</Card.Description
                            >
                        </div>
                        <Button
                            size="sm"
                            onclick={async () => {
                                isSavingPrompt = true;
                                try {
                                    await api.patch("/settings", {
                                        global_prompt: globalPrompt,
                                    });
                                    toast.success("破限提示词已保存");
                                } catch (e) {
                                    toast.error("保存失败", {
                                        description: String(e),
                                    });
                                } finally {
                                    isSavingPrompt = false;
                                }
                            }}
                            disabled={isSavingPrompt}
                            class="w-full sm:w-auto"
                        >
                            {isSavingPrompt ? "保存中..." : "保存提示词"}
                        </Button>
                    </div>
                </Card.Header>
                <Card.Content>
                    <Textarea
                        bind:value={globalPrompt}
                        placeholder="这里输入破限提示词——用来破限的，否则过于R18的角色卡无法应用AI指令。你可以直接到你现在使用的预设里复制一份破限条目中的内容……"
                        class="min-h-[200px] resize-y font-mono text-sm"
                    />
                    <p class="text-xs text-muted-foreground mt-4">
                        提示：破限提示词会作为 System Prompt 附加到每次 AI
                        调用中。如果你不需要，清除所有内容，然后点击保存即可。
                    </p>
                </Card.Content>
            </Card.Root>
        </TabsContent>
    </Tabs>

    <ChannelDialog
        bind:open={isDialogOpen}
        onCallback={() => {
            editingChannel = null;
            loadChannels();
        }}
        editChannel={editingChannel}
    />
</div>
