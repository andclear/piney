<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as Dialog from "$lib/components/ui/dialog";
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import { api } from "$lib/api";
    import { toast } from "svelte-sonner";
    import { Loader2, CheckCircle2, XCircle } from "lucide-svelte";
    import { Switch } from "$lib/components/ui/switch";
    import {
        Select,
        SelectContent,
        SelectItem,
        SelectTrigger,
        SelectValue,
    } from "$lib/components/ui/select";

    interface ChannelData {
        id: string;
        name: string;
        base_url: string;
        model_id: string;
        is_active: boolean;
    }

    let {
        open = $bindable(false),
        onCallback,
        editChannel = null as ChannelData | null,
    } = $props();

    let name = $state("");
    let baseUrl = $state("");
    let apiKey = $state("");
    let modelId = $state("");
    let isActive = $state(true);

    let isTesting = $state(false);
    let testResult = $state<"success" | "error" | null>(null);
    let testMessage = $state("");
    let latencyMs = $state<number | null>(null);
    let isSaving = $state(false);
    let isLoadingModels = $state(false);
    let availableModels = $state<string[]>([]);

    // 编辑模式：填充表单
    $effect(() => {
        if (editChannel && open) {
            name = editChannel.name;
            baseUrl = editChannel.base_url;
            modelId = editChannel.model_id;
            isActive = editChannel.is_active;
            apiKey = ""; // API Key 不返回，需要重新输入（如果要更新）
        } else if (!open) {
            // 关闭时重置
            name = "";
            baseUrl = "";
            apiKey = "";
            modelId = "";
            isActive = true;
            testResult = null;
            availableModels = [];
            latencyMs = null;
        }
    });

    const isEditMode = $derived(!!editChannel);

    async function testConnection() {
        if (!baseUrl || !apiKey || !modelId) {
            toast.error("请先填写完整配置");
            return;
        }

        isTesting = true;
        testResult = null;
        try {
            const res = await api.post("/ai/test", {
                base_url: baseUrl,
                api_key: apiKey,
                model_id: modelId,
            });
            if (res.success && res.data) {
                testResult = "success";
                latencyMs = res.data.latency_ms || null;
                const latencyText = latencyMs ? ` (${latencyMs}ms)` : "";
                toast.success(`连接测试成功${latencyText}`);
            } else {
                testResult = "error";
                testMessage = res.error || "连接失败";
                toast.error("连接失败", { description: testMessage });
            }
        } catch (e) {
            testResult = "error";
            testMessage = String(e);
            toast.error("连接失败", { description: String(e) });
        } finally {
            isTesting = false;
        }
    }

    async function fetchModels() {
        if (!baseUrl || !apiKey) {
            toast.error("请先填写 API Base URL 和 API Key");
            return;
        }

        isLoadingModels = true;
        availableModels = [];
        try {
            const query = new URLSearchParams({
                base_url: baseUrl,
                api_key: apiKey,
            });
            const res = await api.get<any>(`/ai/models?${query.toString()}`);

            if (res.success && res.data) {
                const data = res.data;
                // Parse standard OpenAI models response
                if (data.data && Array.isArray(data.data)) {
                    availableModels = data.data.map((m: any) => m.id);
                    toast.success(`成功获取 ${availableModels.length} 个模型`);
                } else {
                    toast.error("返回格式不符合 OpenAI 标准");
                }
            } else {
                toast.error("获取模型失败", { description: res.error });
            }
        } catch (e) {
            toast.error("获取模型失败", { description: String(e) });
        } finally {
            isLoadingModels = false;
        }
    }

    async function save() {
        // 编辑模式下 apiKey 可选（不更新则保留原值）
        if (!name || !baseUrl || !modelId) {
            toast.error("请填写所有必填项");
            return;
        }
        if (!isEditMode && !apiKey) {
            toast.error("请填写 API Key");
            return;
        }

        isSaving = true;
        try {
            let res;
            if (isEditMode && editChannel) {
                // 编辑模式：PUT
                const payload: Record<string, any> = {
                    name,
                    base_url: baseUrl,
                    model_id: modelId,
                    is_active: isActive,
                };
                if (apiKey) {
                    payload.api_key = apiKey; // 只有填写了才更新
                }
                res = await api.put(`/ai/channels/${editChannel.id}`, payload);
            } else {
                // 新增模式：POST
                res = await api.post("/ai/channels", {
                    name,
                    base_url: baseUrl,
                    api_key: apiKey,
                    model_id: modelId,
                    is_active: isActive,
                });
            }

            if (res.success) {
                toast.success(isEditMode ? "渠道更新成功" : "渠道添加成功");
                open = false;
                onCallback?.();
            } else {
                toast.error("保存失败", { description: res.error });
            }
        } catch (e) {
            toast.error("保存失败", { description: String(e) });
        } finally {
            isSaving = false;
        }
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-[500px]">
        <Dialog.Header>
            <Dialog.Title
                >{isEditMode ? "编辑 AI 渠道" : "添加 AI 渠道"}</Dialog.Title
            >
            <Dialog.Description>
                {isEditMode
                    ? "修改渠道配置。留空 API Key 则保持原值不变。"
                    : "配置兼容 OpenAI 协议的 AI 服务商。"}
            </Dialog.Description>
        </Dialog.Header>
        <div class="grid gap-4 py-4">
            <div class="grid grid-cols-4 items-center gap-4">
                <Label for="name" class="text-right">渠道名称</Label>
                <Input
                    id="name"
                    bind:value={name}
                    placeholder="例如：OpenAI"
                    class="col-span-3"
                />
            </div>
            <div class="grid grid-cols-4 items-center gap-4">
                <Label for="base_url" class="text-right">Base URL</Label>
                <Input
                    id="base_url"
                    bind:value={baseUrl}
                    placeholder="https://api.openai.com/v1"
                    class="col-span-3"
                />
            </div>
            <div class="grid grid-cols-4 items-center gap-4">
                <Label for="api_key" class="text-right">API Key</Label>
                <Input
                    id="api_key"
                    type="password"
                    bind:value={apiKey}
                    placeholder="sk-..."
                    class="col-span-3"
                />
            </div>

            <div class="grid grid-cols-4 items-start gap-4">
                <Label for="model" class="text-right pt-2">模型名称</Label>
                <div class="col-span-3 flex flex-col gap-2">
                    <div class="flex gap-2">
                        <div class="relative flex-1">
                            <Input
                                id="model"
                                bind:value={modelId}
                                placeholder="gemini-3-flash"
                            />
                        </div>
                        <Button
                            variant="outline"
                            size="sm"
                            onclick={fetchModels}
                            disabled={isLoadingModels}
                        >
                            {isLoadingModels ? "获取中..." : "获取列表"}
                        </Button>
                    </div>

                    {#if availableModels.length > 0}
                        <select
                            class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                            onchange={(e) => (modelId = e.currentTarget.value)}
                        >
                            <option value="">-- 快速选择 --</option>
                            {#each availableModels as m}
                                <option value={m}>{m}</option>
                            {/each}
                        </select>
                    {/if}
                    <p class="text-[10px] text-muted-foreground">
                        点击"获取列表"可尝试从服务商加载模型，也可以直接手动输入。
                    </p>
                </div>
            </div>

            <div class="grid grid-cols-4 items-center gap-4">
                <Label for="active" class="text-right">状态</Label>
                <div class="col-span-3 flex items-center gap-2">
                    <Switch id="active" bind:checked={isActive} />
                    <span class="text-sm text-muted-foreground"
                        >{isActive ? "启用" : "禁用"}</span
                    >
                </div>
            </div>

            {#if testResult}
                <div
                    class="rounded-md bg-muted p-3 flex items-start gap-2 text-sm"
                >
                    {#if testResult === "success"}
                        <CheckCircle2 class="h-4 w-4 text-green-500 mt-0.5" />
                        <span class="text-green-600">
                            连接测试通过{latencyMs
                                ? ` (${latencyMs}ms)`
                                : ""}，服务可用。
                        </span>
                    {:else}
                        <XCircle class="h-4 w-4 text-red-500 mt-0.5" />
                        <div class="flex flex-col">
                            <span class="text-red-500 font-medium"
                                >连接失败</span
                            >
                            <span
                                class="text-xs text-muted-foreground break-all"
                                >{testMessage}</span
                            >
                        </div>
                    {/if}
                </div>
            {/if}
        </div>
        <Dialog.Footer>
            <Button
                variant="outline"
                onclick={testConnection}
                disabled={isTesting}
            >
                {#if isTesting}
                    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                {/if}
                测试连接
            </Button>
            <Button onclick={save} disabled={isSaving}>
                {isSaving ? "保存中..." : "保存渠道"}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
