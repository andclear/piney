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
        BrainCircuit,
        CheckCircle2,
        XCircle,
        Ghost,
        Sparkles,
        Info,
    } from "lucide-svelte";
    import { Textarea } from "$lib/components/ui/textarea";
    import { api } from "$lib/api";
    import { onMount } from "svelte";
    import { toast } from "svelte-sonner";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
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
    let configGlobal = $state("");
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
        breadcrumbs.set([
            { label: '设置' }
        ]);
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
                configGlobal = res.data.ai_config_global || "";
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
                ai_config_global: configGlobal,
            });
            toast.success("默认模型配置已保存");
        } catch (e) {
            toast.error("保存配置失败", { description: String(e) });
        } finally {
            isSavingConfig = false;
        }
    }

</script>

<div class="h-full flex py-6 flex-col gap-6 p-4 sm:p-6">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-2xl font-bold tracking-tight">系统设置</h1>
            <p class="text-muted-foreground mt-1">管理应用配置与 AI 接入。</p>
        </div>
    </div>

    <Tabs value="ai" class="w-full p4">
        <TabsList class="mb-4 gap-4">
            <TabsTrigger value="ai" class="gap-2">
                <Sparkles class="h-4 w-4" />
                API 配置
            </TabsTrigger>
            <TabsTrigger value="prompts" class="gap-2">
                <Ghost class="h-4 w-4" />
                全局提示词
            </TabsTrigger>
            <TabsTrigger value="about" class="gap-2">
                <Info class="h-4 w-4" />
                关于
            </TabsTrigger>
        </TabsList>

        <!-- About Tab -->
        <TabsContent value="about">
            <Card.Root>
                <Card.Content class="flex flex-col items-center py-8 space-y-8">
                    <!-- Logo & Header -->
                    <div class="flex flex-col items-center">
                        <div class="relative mb-6 group">
                            <div class="absolute inset-0 bg-primary/20 blur-xl rounded-full transform rotate-6 group-hover:rotate-12 transition-all duration-500 opacity-0 group-hover:opacity-100"></div>
                            <img 
                                src="/logo.png" 
                                alt="Piney Logo" 
                                class="relative w-32 h-32 rounded-3xl shadow-2xl rotate-6 group-hover:rotate-0 transition-all duration-500 ease-out bg-white dark:bg-zinc-900/50 p-1 backdrop-blur-sm ring-1 ring-border/50" 
                            />
                        </div>
                        
                        <h2 class="text-3xl font-bold tracking-tight bg-gradient-to-r from-primary to-primary/50 bg-clip-text text-transparent mb-2">小兄许 / Piney</h2>
                        <Badge variant="secondary" class="font-mono text-xs px-2 py-0.5 border-primary/20">0.2.6.3 版本</Badge>
                    </div>

                    <!-- Description -->
                    <div class="max-w-2xl text-center text-xs text-muted-foreground leading-relaxed px-6 py-4 bg-muted/40 rounded-lg border border-border/50 font-mono">
                        SillyTavern角色卡工作站，支持角色卡/世界书/正则/美化/图库的创建、导入、编辑、修改功能。
                    </div>

                    <!-- Socials -->
                    <div class="flex flex-col items-center gap-3 mt-2">
                        <p class="text-sm font-medium text-muted-foreground">欢迎加入小酒窝</p>
                        <div class="flex gap-3">
                            <a href="https://jiuwo.me" target="_blank">
                                <Button variant="outline" size="sm" class="gap-2">
                                    <svg viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" class="w-4 h-4"><path d="M512 1024H0V512C0 230.4 230.4 0 512 0s512 230.4 512 512-230.4 512-512 512z" fill="#000000" class="dark:fill-white"></path><path d="M386.56 765.44l-199.68 71.68L222.72 640c-25.6-40.96-43.52-87.04-51.2-138.24-20.48-174.08 102.4-330.24 273.92-350.72s330.24 102.4 350.72 273.92-102.4 330.24-273.92 350.72c-46.08 7.68-92.16 2.56-135.68-10.24z" fill="#E4001E"></path><path d="M399.36 811.52l-209.92 25.6 79.36-184.32c-17.92-43.52-23.04-94.72-17.92-145.92 17.92-174.08 174.08-299.52 348.16-281.6s299.52 174.08 281.6 348.16-174.08 299.52-348.16 281.6c-51.2-5.12-94.72-20.48-133.12-43.52z" fill="#00B0F5"></path><path d="M186.88 837.12l58.88-189.44c-20.48-43.52-33.28-92.16-33.28-143.36 0-174.08 140.8-314.88 314.88-314.88S844.8 330.24 844.8 504.32 704 819.2 529.92 819.2c-48.64 0-94.72-10.24-135.68-30.72l-207.36 48.64z" fill="#FF5B00"></path><path d="M186.88 837.12l79.36-184.32c-17.92-43.52-23.04-94.72-17.92-145.92 17.92-174.08 174.08-299.52 348.16-281.6 74.24 7.68 138.24 40.96 186.88 87.04 38.4 53.76 61.44 117.76 61.44 189.44C844.8 678.4 704 819.2 529.92 819.2c-48.64 0-94.72-10.24-135.68-30.72l-207.36 48.64z" fill="#00AD42"></path><path d="M186.88 837.12l79.36-184.32c-17.92-43.52-23.04-94.72-17.92-145.92 17.92-174.08 174.08-299.52 348.16-281.6 56.32 5.12 107.52 25.6 151.04 56.32 28.16 40.96 48.64 89.6 53.76 143.36 20.48 174.08-102.4 330.24-273.92 350.72-48.64 5.12-94.72 0-138.24-12.8l-202.24 74.24z" fill="#FFF9A4"></path></svg>
                                    小酒窝论坛
                                </Button>
                            </a>
                            <a href="https://discord.gg/atsXVr7ve8" target="_blank">
                                <Button variant="outline" size="sm" class="gap-2">
                                    <svg viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" class="w-4 h-4"><path d="M404.906935 471.06782A61.822209 61.822209 0 0 0 348.744505 535.937603a61.822209 61.822209 0 0 0 57.9039 65.305151 61.822209 61.822209 0 0 0 58.339268-65.305151 61.386842 61.386842 0 0 0-60.080738-64.869783zM619.543196 471.06782a61.822209 61.822209 0 0 0-57.9039 64.869783 61.822209 61.822209 0 0 0 57.9039 65.305151 61.822209 61.822209 0 0 0 58.339268-65.305151 61.386842 61.386842 0 0 0-58.339268-64.869783z" fill="#5866f2"></path><path d="M512.007382 0a511.992381 511.992381 0 1 0 511.992381 511.557013A511.557013 511.557013 0 0 0 512.007382 0z m320.430606 674.384522a531.583926 531.583926 0 0 1-161.521406 81.413755h-2.176839a421.435906 421.435906 0 0 1-33.087943-53.550224v-2.612206a354.389284 354.389284 0 0 0 50.067282-24.380589v-3.047574a91.862579 91.862579 0 0 1-10.013456-8.271986 381.817447 381.817447 0 0 1-324.784283 0l-10.013456 7.836618v3.047574a337.845313 337.845313 0 0 0 50.50265 24.38059v2.612206a377.028403 377.028403 0 0 1-33.087943 53.550223h-2.176838a534.6315 534.6315 0 0 1-161.086039-81.413754 545.080324 545.080324 0 0 1 92.297947-367.014947 556.399884 556.399884 0 0 1 130.610301-40.924561 341.763622 341.763622 0 0 1 16.543971 33.523311 493.271571 493.271571 0 0 1 147.589641 0 341.763622 341.763622 0 0 1 16.543971-33.523311 2.176838 2.176838 0 0 1 2.176839 0 556.399884 556.399884 0 0 1 130.610301 40.924561 541.597383 541.597383 0 0 1 101.0053 367.450314z" fill="#5866f2"></path></svg>
                                    小酒窝-DC分窝
                                </Button>
                            </a>
                        </div>
                    </div>

                    <div class="w-full max-w-4xl grid grid-cols-1 md:grid-cols-2 gap-8 text-sm mt-4">
                        <!-- Credits -->
                        <div class="space-y-4 p-6 rounded-xl bg-muted/30 border border-muted/50">
                            <h3 class="font-semibold text-lg flex items-center gap-2 text-foreground">
                                <Sparkles class="w-4 h-4 text-amber-500" /> 致谢
                            </h3>
                            <ul class="space-y-4">
                                <li class="group">
                                    <div class="flex items-center gap-2">
                                        <span class="font-medium text-foreground group-hover:text-primary transition-colors">@我不理解</span>
                                        <Badge variant="outline" class="text-[10px] h-4">技术支持</Badge>
                                    </div>
                                    <p class="text-xs text-muted-foreground mt-1 leading-relaxed">小红书号：8032060592，感谢老师为项目提供的大量技术支持，有兴趣的话可以关注一波老师。</p>
                                </li>
                                <li class="group">
                                    <span class="font-medium text-foreground group-hover:text-primary transition-colors">@随风飘逸</span>
                                    <Badge variant="outline" class="text-[10px] h-4">竟肘击我</Badge>
                                    <p class="text-xs text-muted-foreground mt-1 leading-relaxed">因为随风的肘击，于是有了聊天记录的“随风模式”和“图库”功能。</p>
                                </li>
                                <li class="group">
                                    <span class="font-medium text-foreground group-hover:text-primary transition-colors">@薯片</span>
                                    <Badge variant="outline" class="text-[10px] h-4">模板提供</Badge>
                                    <p class="text-xs text-muted-foreground mt-1 leading-relaxed">感谢薯片提供的角色卡写卡模板，非常好用。</p>
                                </li>
                            </ul>
                        </div>

                        <!-- Teachers -->
                        <div class="space-y-4 p-6 rounded-xl bg-muted/30 border border-muted/50">
                            <h3 class="font-semibold text-lg flex items-center gap-2 text-foreground">
                                <BrainCircuit class="w-4 h-4 text-purple-500" /> 感谢老师
                            </h3>
                            <ul class="space-y-4">
                                <li class="group">
                                    <div class="font-medium text-foreground group-hover:text-primary transition-colors">@KAKAA @青空莉想做舞台少女的狗</div>
                                    <a href="https://n0vi028.github.io/JS-Slash-Runner-Doc/" target="_blank" class="text-xs text-primary/80 hover:text-primary hover:underline mt-1 block">
                                        酒馆助手 ↗
                                    </a>
                                    <p class="text-xs text-muted-foreground mt-0.5">深刻学习了怎么渲染聊天记录，但是能力有限没怎么学会 T_T。</p>
                                </li>
                                <li class="group">
                                    <div class="font-medium text-foreground group-hover:text-primary transition-colors">@lucialili</div>
                                    <a href="https://discord.com/channels/1291925535324110879/1455494729763393713" target="_blank" class="text-xs text-primary/80 hover:text-primary hover:underline mt-1 block">
                                        SillyReader-旅程 ↗
                                    </a>
                                    <p class="text-xs text-muted-foreground mt-0.5">学习了一下jsonl格式聊天记录中检索标签对的逻辑。</p>
                                </li>
                                <li class="group">
                                    <div class="font-medium text-foreground group-hover:text-primary transition-colors">@温柔半两</div>
                                    <a href="https://discord.com/channels/1379304008157499423/1463927945184673822/1463927945184673822" target="_blank" class="text-xs text-primary/80 hover:text-primary hover:underline mt-1 block">
                                        角色卡百宝箱-尾巴镇 ↗
                                    </a>
                                    <p class="text-xs text-muted-foreground mt-0.5">得到了添加"小剧场"存储的灵感。</p>
                                </li>
                            </ul>
                        </div>
                    </div>

                    <!-- Footer & Agreement -->
                    <div class="pt-8 w-full border-t flex flex-col items-center gap-3 mt-4">
                         <a href="/agreement" class="text-sm font-medium text-muted-foreground hover:text-primary transition-colors">
                            用户协议
                        </a>
                        
                        <div class="text-[10px] font-mono tracking-wider uppercase">
                            Made with ❤️ by <span class="hover:text-primary transition-colors cursor-default">laopobao</span>
                        </div>
                    </div>
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
                        <!-- 全局 AI 模型 -->
                        <!-- 全局 AI 模型 -->
                        <div class="space-y-2 min-w-0">
                            <Label for="config-global">全局 AI 模型</Label>
                            <p class="text-xs text-muted-foreground mb-2">
                                所有 AI 功能要使用的模型。
                            </p>
                            <Select type="single" bind:value={configGlobal}>
                                <SelectTrigger class="w-full">
                                    <SelectValue class="block truncate w-full">
                                        {#if configGlobal}
                                            {@const selected = channels.find(
                                                (c) => c.id === configGlobal,
                                            )}
                                            {#if selected}
                                                {selected.name} ({selected.model_id})
                                            {:else}
                                                <span class="text-muted-foreground">选择渠道...</span>
                                            {/if}
                                        {:else}
                                            <span class="text-muted-foreground">选择渠道...</span>
                                        {/if}
                                    </SelectValue>
                                </SelectTrigger>
                                <SelectContent class="max-h-[40vh] overflow-y-auto">
                                    <SelectItem value="" label="未配置">未配置</SelectItem>
                                    {#each channels as c}
                                        <SelectItem
                                            value={c.id}
                                            label={`${c.name} (${c.model_id})`}
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
                            <Card.Title>全局提示词（可选）</Card.Title>
                            <Card.Description class="mt-2"
                                >会附在所有 AI 调用的 Prompt 之前。</Card.Description
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
                                    toast.success("已保存");
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
                        placeholder="请输入全局提示词（非必要设置项）"
                        class="min-h-[200px] resize-y font-mono text-sm"
                    />
                    <p class="text-xs text-muted-foreground mt-4">
                        提示：全局提示词会作为 System Prompt 附加到每次 AI
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
