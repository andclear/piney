<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import {
        Upload,
        FileJson,
        FileImage,
        Loader2,
        CheckCircle2,
        XCircle,
        AlertTriangle,
    } from "@lucide/svelte";
    import { Button } from "$lib/components/ui/button";
    import {
        Tabs,
        TabsContent,
        TabsList,
        TabsTrigger,
    } from "$lib/components/ui/tabs";
    import * as Card from "$lib/components/ui/card";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { Badge } from "$lib/components/ui/badge";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";
    import { API_BASE } from "$lib/api";
    import { Progress } from "$lib/components/ui/progress";
    import { breadcrumbs } from "$lib/stores/breadcrumb";

    let dragging = false;
    let uploading = false;
    let progress = 0;
    let currentFileIndex = 0;
    let totalFilesCount = 0;
    let extension_hint = ".png, .json";

    // Mode: 'card' | 'worldbook'
    let importType = "card";

    onMount(() => {
        breadcrumbs.set([
            { label: '导入数据' }
        ]);

        const tab = $page.url.searchParams.get("tab");
        if (tab === "worldbook") {
            importType = "worldbook";
        }
    });

    type ImportResult = {
        file_name: string;
        status: "success" | "error";
        reason?: string;
    };

    let importResults: ImportResult[] = [];
    let successCount = 0;
    let failCount = 0;

    function handleDragOver(e: DragEvent) {
        e.preventDefault();
        dragging = true;
    }

    function handleDragLeave(e: DragEvent) {
        e.preventDefault();
        dragging = false;
    }

    async function handleDrop(e: DragEvent) {
        e.preventDefault();
        dragging = false;

        if (e.dataTransfer && e.dataTransfer.files.length > 0) {
            await uploadFiles(e.dataTransfer.files);
        }
    }

    async function handleFileSelect(e: Event) {
        const target = e.target as HTMLInputElement;
        if (target.files && target.files.length > 0) {
            await uploadFiles(target.files);
        }
        // Reset input
        target.value = "";
    }

    async function uploadFiles(files: FileList) {
        uploading = true;
        importResults = []; // Clear previous results
        successCount = 0;
        failCount = 0;

        try {
            // Calculate total files for progress bar
            // Calculate total files for progress bar
            totalFilesCount = files.length;
            currentFileIndex = 0;
            progress = 0;

            // Validate files first
            const filePromises = Array.from(files).map((file) => {
                return new Promise<{
                    file: File;
                    valid: boolean;
                    reason?: string;
                }>((resolve) => {
                    // Skip validation for PNGs (Card mode) or if explicitly allowing all
                    if (
                        importType === "card" &&
                        file.name.toLowerCase().endsWith(".png")
                    ) {
                        resolve({ file, valid: true });
                        return;
                    }

                    const reader = new FileReader();
                    reader.onload = (e) => {
                        try {
                            const text = e.target?.result as string;
                            const json = JSON.parse(text);

                            if (importType === "worldbook") {
                                // Check if it's a world book
                                if (
                                    json.entries &&
                                    typeof json.entries === "object"
                                ) {
                                    resolve({ file, valid: true });
                                } else {
                                    // Check if it looks like a character card
                                    if (
                                        json.spec ||
                                        json.data?.name ||
                                        json.name
                                    ) {
                                        resolve({
                                            file,
                                            valid: false,
                                            reason: "检测到角色卡格式，请切换到角色卡导入页。",
                                        });
                                    } else {
                                        resolve({
                                            file,
                                            valid: false,
                                            reason: "无效的世界书格式 (缺少 entries 字段)",
                                        });
                                    }
                                }
                            } else {
                                // Card mode validation (optional, currently loose)
                                resolve({ file, valid: true });
                            }
                        } catch (err) {
                            resolve({
                                file,
                                valid: false,
                                reason: "JSON 解析失败",
                            });
                        }
                    };
                    reader.readAsText(file);
                });
            });

            const validatedFiles = await Promise.all(filePromises);

            const endpoint =
                importType === "card"
                    ? `${API_BASE}/api/cards/import`
                    : `${API_BASE}/api/world_info/import`;

            const token = localStorage.getItem("auth_token");
            const headers: HeadersInit = {};
            if (token) {
                headers["Authorization"] = `Bearer ${token}`;
            }

            for (let i = 0; i < validatedFiles.length; i++) {
                const item = validatedFiles[i];
                currentFileIndex = i + 1;
                if (!item.valid) {
                    importResults = [
                        ...importResults,
                        {
                            file_name: item.file.name,
                            status: "error",
                            reason: item.reason,
                        },
                    ];
                    failCount++;
                    // Update progress even for skipped files
                    // Update progress even for skipped files
                    progress = Math.round(((i + 1) / totalFilesCount) * 100);
                    continue;
                }

                // Upload single file
                const formData = new FormData();
                formData.append("files", item.file); // Backend expects "files"

                try {
                    const res = await fetch(endpoint, {
                        method: "POST",
                        headers,
                        body: formData,
                    });

                    if (res.ok) {
                        const json: ImportResult[] = await res.json();
                        // Append results (usually just one)
                        importResults = [...importResults, ...json];
                        
                        json.forEach(r => {
                            if (r.status === "success") successCount++;
                            else failCount++;
                        });
                    } else {
                        const text = await res.text();
                        importResults = [
                            ...importResults,
                            {
                                file_name: item.file.name,
                                status: "error",
                                reason: text,
                            },
                        ];
                        failCount++;
                    }
                } catch (err) {
                    console.error(err);
                    importResults = [
                        ...importResults,
                        {
                            file_name: item.file.name,
                            status: "error",
                            reason: "网络错误",
                        },
                    ];
                    failCount++;
                }

                // Update progress after each file
                progress = Math.round(((i + 1) / totalFilesCount) * 100);
            }

            if (failCount === 0) {
                toast.success(`成功导入 ${successCount} 个文件`);
            } else {
                toast.warning(`导入完成：${successCount} 成功，${failCount} 失败`);
            }
        } catch (error) {
            console.error(error);
            toast.error("上传过程中发生网络错误");
        } finally {
            uploading = false;
        }
    }



    $: extension_hint = importType === "card" ? ".png, .json" : ".json";
</script>

<div class="container py-6 space-y-6 max-w-3xl mx-auto">
    <div class="space-y-1">
        <h1 class="text-2xl font-bold tracking-tight">导入数据</h1>
        <p class="text-muted-foreground">
            支持 PNG 角色卡、JSON 数据，兼容 SillyTavern v2/v3 标准角色卡。
        </p>
    </div>

    <Tabs bind:value={importType} class="w-full">
        <TabsList class="grid w-full grid-cols-2">
            <TabsTrigger value="card">角色卡导入</TabsTrigger>
            <TabsTrigger value="worldbook">世界书导入</TabsTrigger>
        </TabsList>

        <div class="mt-6 space-y-6">
            <!-- 上传区域 -->
            <div
                role="button"
                tabindex="0"
                class={cn(
                    "border-2 border-dashed rounded-xl p-12 flex flex-col items-center justify-center text-center transition-colors cursor-pointer bg-card hover:bg-accent/50",
                    dragging ? "border-primary bg-accent" : "border-border",
                    uploading ? "opacity-50 pointer-events-none" : "",
                )}
                on:dragover={handleDragOver}
                on:dragleave={handleDragLeave}
                on:drop={handleDrop}
                on:click={() => document.getElementById("file-upload")?.click()}
                on:keydown={(e) =>
                    e.key === "Enter" &&
                    document.getElementById("file-upload")?.click()}
            >
                <input
                    id="file-upload"
                    type="file"
                    multiple
                    accept={importType === "card" ? ".png,.json" : ".json"}
                    class="hidden"
                    on:change={handleFileSelect}
                />

                <div
                    class="w-16 h-16 rounded-full bg-secondary flex items-center justify-center mb-4"
                >
                    {#if uploading}
                        <Loader2
                            class="w-8 h-8 animate-spin text-muted-foreground"
                        />
                    {:else}
                        <Upload class="w-8 h-8 text-muted-foreground" />
                    {/if}
                </div>

                <div class="space-y-2">
                    <h3 class="text-lg font-semibold">拖拽文件到这里</h3>
                    <p class="text-sm text-muted-foreground">或点击选择文件</p>
                    {#if uploading}
                        <div class="w-64 mt-4 mx-auto">
                            <Progress value={progress} class="h-2" />
                            <p class="text-xs text-muted-foreground mt-2">正在处理... {currentFileIndex} / {totalFilesCount}</p>
                        </div>
                    {/if}
                </div>

                <div
                    class="flex items-center gap-4 mt-8 text-xs text-muted-foreground uppercase"
                >
                    {#if importType === "card"}
                        <div class="flex items-center gap-1">
                            <FileImage class="w-4 h-4" /> PNG
                        </div>
                    {/if}
                    <div class="flex items-center gap-1">
                        <FileJson class="w-4 h-4" /> JSON
                    </div>
                </div>
            </div>

            <!-- 结果反馈区域 -->
            {#if importResults.length > 0}
                <div
                    class="rounded-lg border bg-card text-card-foreground shadow-sm animate-in fade-in slide-in-from-top-4 duration-300"
                >
                    <div class="p-6">
                        <h3
                            class="text-lg font-semibold leading-none tracking-tight mb-4"
                        >
                            导入结果 ({successCount} 成功 / {failCount} 失败)
                        </h3>
                        <div
                            class="space-y-2 max-h-[300px] overflow-y-auto pr-2"
                        >
                            {#each importResults as result}
                                <div
                                    class={cn(
                                        "flex items-start gap-3 p-3 rounded-md text-sm",
                                        result.status === "error"
                                            ? "bg-destructive/10 text-destructive"
                                            : "bg-muted",
                                    )}
                                >
                                    {#if result.status === "error"}
                                        <XCircle
                                            class="w-5 h-5 shrink-0 mt-0.5"
                                        />
                                    {:else}
                                        <CheckCircle2
                                            class="w-5 h-5 shrink-0 mt-0.5 text-primary"
                                        />
                                    {/if}

                                    <div class="flex-1 space-y-1">
                                        <p class="font-medium">
                                            {result.file_name}
                                        </p>
                                        {#if result.reason}
                                            <p class="text-xs opacity-90">
                                                {result.reason}
                                            </p>
                                        {/if}
                                    </div>
                                </div>
                            {/each}
                        </div>
                    </div>
                </div>
            {/if}
        </div>

        <p class="text-center text-sm text-muted-foreground mt-4">
            当前模式：{importType === "card"
                ? "角色卡 (Character Card)"
                : "世界书 (World Info)"}
        </p>
    </Tabs>
</div>

<!-- 第一步：警告对话框 -->

