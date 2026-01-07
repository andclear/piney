<script lang="ts">
    // import { auth } from "$lib/stores/auth.svelte"; // Remove store import if verified unused or keep for other things

    let fileInput: HTMLInputElement;
    let logs: string[] = [];
    let savedJson: string | null = null;
    let error: string | null = null;
    let loading = false;

    async function handleDebugUpload() {
        if (!fileInput.files || fileInput.files.length === 0) return;

        logs = ["开始上传文件..."];
        savedJson = null;
        error = null;
        loading = true;

        const file = fileInput.files[0];
        const formData = new FormData();
        formData.append("file", file);

        try {
            const token = localStorage.getItem("auth_token");
            if (!token) {
                logs.push("错误: 未找到登录凭证 (Token)，请重新登录");
                loading = false;
                return;
            }

            // 使用 relative path，依靠 Vite proxy 转发
            const res = await fetch(`/api/cards/debug_import`, {
                method: "POST",
                headers: {
                    Authorization: `Bearer ${token}`,
                },
                body: formData,
            });

            if (!res.ok) {
                const text = await res.text();
                throw new Error(`Server Error: ${res.status} ${text}`);
            }

            const data = await res.json();
            logs = data.logs || [];
            savedJson = data.saved_json || null;
            error = data.error || null;

            if (error) {
                logs.push(`错误中止: ${error}`);
            } else {
                logs.push("=== 流程结束 ===");
            }
        } catch (e) {
            logs.push(`请求异常: ${e}`);
            error = String(e);
        } finally {
            loading = false;
        }
    }

    function downloadJson() {
        if (!savedJson) return;
        const blob = new Blob([savedJson], { type: "application/json" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "extracted_card_debug.json";
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }
</script>

<div class="space-y-6">
    <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold tracking-tight">导入调试工具</h1>
    </div>

    <div class="rounded-lg border bg-card text-card-foreground shadow-sm p-6">
        <div class="space-y-4">
            <p class="text-sm text-muted-foreground">
                此工具用于验证角色卡 (PNG) 的元数据是否能被完整提取。
                系统会尝试解析上传的图片，并输出详细的处理日志和最终入库的 JSON
                数据。
            </p>

            <div class="flex flex-col gap-4">
                <label for="file-upload" class="font-medium text-sm"
                    >1. 选择文件 (PNG)</label
                >
                <input
                    id="file-upload"
                    type="file"
                    accept=".png"
                    class="block w-full text-sm text-slate-500
                      file:mr-4 file:py-2 file:px-4
                      file:rounded-full file:border-0
                      file:text-sm file:font-semibold
                      file:bg-violet-50 file:text-violet-700
                      hover:file:bg-violet-100"
                    bind:this={fileInput}
                />

                <div class="pt-2">
                    <label class="font-medium text-sm block mb-2"
                        >2. 点击调试</label
                    >
                    <button
                        class="px-6 py-2.5 bg-green-600 text-white font-semibold rounded-md shadow hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        on:click={handleDebugUpload}
                        disabled={loading}
                    >
                        {loading ? "处理中..." : "开始调试"}
                    </button>
                    <p class="text-xs text-muted-foreground mt-1">
                        点击按钮后将上传并解析。
                    </p>
                </div>
            </div>
        </div>
    </div>

    {#if logs.length > 0}
        <div
            class="rounded-lg border bg-zinc-950 text-green-400 font-mono text-xs shadow-sm p-4 h-96 overflow-y-auto"
        >
            {#each logs as log}
                <div class="border-b border-zinc-900 py-1 last:border-0">
                    {log}
                </div>
            {/each}
        </div>
    {/if}

    <div
        class="rounded-lg border bg-card text-card-foreground shadow-sm p-6 mt-6"
    >
        <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold">提取结果验证</h3>
            <button
                class="px-6 py-2 rounded font-medium transition-colors {savedJson
                    ? 'bg-blue-600 hover:bg-blue-700 text-white'
                    : 'bg-gray-200 text-gray-500 cursor-not-allowed'}"
                on:click={() => {
                    if (!savedJson) {
                        alert("请先上传文件并开始调试，提取成功后才能下载。");
                        return;
                    }
                    downloadJson();
                }}
            >
                {savedJson
                    ? "下载完整 JSON (已就绪)"
                    : "请先执行调试以生成 JSON"}
            </button>
        </div>
        {#if savedJson}
            <div
                class="bg-muted p-4 rounded overflow-auto max-h-60 text-xs text-zinc-900 dark:text-zinc-100"
            >
                <pre>{savedJson}</pre>
            </div>
        {:else}
            <div
                class="bg-muted/50 p-8 rounded text-center text-muted-foreground border border-dashed"
            >
                {#if loading}
                    <p>正在处理中...</p>
                {:else if error}
                    <p class="text-red-500">处理出错，无法获取 JSON</p>
                {:else}
                    <p>等待操作。成功提取后此处将显示 JSON 内容。</p>
                {/if}
            </div>
        {/if}
    </div>
</div>
