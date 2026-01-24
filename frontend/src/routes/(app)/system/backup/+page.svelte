<script lang="ts">
    import { DatabaseBackup, Upload, Download, RotateCcw, AlertTriangle, FileUp, HardDriveDownload } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button";
    import * as Card from "$lib/components/ui/card";
    import * as Tabs from "$lib/components/ui/tabs";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { toast } from "svelte-sonner";
    import { cn } from "$lib/utils";
    import { API_BASE } from "$lib/api";

    let activeTab = "export";
    let isRestoreDialogOpen = false;
    let selectedFile: File | null = null;
    let fileInput: HTMLInputElement;

    // --- 导出备份 ---
    let isExporting = false;
    
    async function handleExport() {
        if (isExporting) return;
        isExporting = true;
        
        const loadingToast = toast.loading("正在准备导出...", { duration: Infinity });
        
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/backup/export`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            
            if (res.ok) {
                // 获取文件名
                const contentDisposition = res.headers.get("content-disposition");
                let filename = "piney_backup.piney";
                if (contentDisposition) {
                    const match = contentDisposition.match(/filename="(.+)"/);
                    if (match) filename = match[1];
                }
                
                // 下载文件
                const blob = await res.blob();
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = filename;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);
                
                toast.dismiss(loadingToast);
                toast.success("导出成功！文件已开始下载");
            } else {
                const text = await res.text();
                toast.dismiss(loadingToast);
                toast.error(`导出失败: ${text}`);
            }
        } catch (e) {
            console.error(e);
            toast.dismiss(loadingToast);
            toast.error("导出失败：网络错误");
        } finally {
            isExporting = false;
        }
    }

    // --- 恢复备份 ---
    function triggerFileInput() {
        fileInput.click();
    }

    function handleFileSelect(e: Event) {
        const target = e.target as HTMLInputElement;
        if (target.files && target.files.length > 0) {
            const file = target.files[0];
            if (!file.name.endsWith(".piney")) {
                toast.error("请选择 .piney 格式的备份文件");
                target.value = ""; // reset
                return;
            }
            selectedFile = file;
            isRestoreDialogOpen = true;
        }
    }

    let isRestoring = false;

    async function handleRestoreConfirm() {
        if (!selectedFile || isRestoring) return;
        
        isRestoreDialogOpen = false;
        isRestoring = true;
        const loadingToast = toast.loading("正在上传并恢复数据...", { duration: Infinity });

        try {
            const token = localStorage.getItem("auth_token");
            const formData = new FormData();
            formData.append("backup", selectedFile);
            
            const res = await fetch(`${API_BASE}/api/backup/import`, {
                method: "POST",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
                body: formData,
            });

            toast.dismiss(loadingToast);
            
            if (res.ok) {
                toast.success("数据恢复成功！请重启服务以确保数据生效");
                selectedFile = null;
                if (fileInput) fileInput.value = "";
            } else {
                const text = await res.text();
                toast.error(`恢复失败: ${text}`);
            }
        } catch (e) {
            console.error(e);
            toast.dismiss(loadingToast);
            toast.error("恢复失败：网络错误");
        } finally {
            isRestoring = false;
        }
    }

    function handleRestoreCancel() {
        isRestoreDialogOpen = false;
        selectedFile = null;
        if (fileInput) fileInput.value = "";
    }
</script>

<div class="container py-8 space-y-8 max-w-4xl mx-auto">
    <div class="space-y-1">
        <h1 class="text-3xl font-bold tracking-tight">数据备份与恢复</h1>
        <p class="text-muted-foreground">
            管理您的系统数据，支持全量备份与一键恢复。
        </p>
    </div>

    <!-- Main Content Tabs -->
    <Tabs.Root bind:value={activeTab} class="w-full">
        <Tabs.List class="grid w-full grid-cols-2 mb-8 h-auto">
            <Tabs.Trigger value="export" class="text-base py-3">
                <DatabaseBackup class="mr-2 h-4 w-4" />
                导出数据
            </Tabs.Trigger>
            <Tabs.Trigger value="import" class="text-base py-3">
                <RotateCcw class="mr-2 h-4 w-4" />
                恢复数据
            </Tabs.Trigger>
        </Tabs.List>

        <!-- Tab 1: Export -->
        <Tabs.Content value="export" class="space-y-6 focus-visible:outline-none">
            <Card.Root class="border-primary/20 shadow-md">
                <Card.Header>
                    <Card.Title class="flex items-center gap-2 text-2xl">
                        <HardDriveDownload class="h-6 w-6 text-primary" />
                        系统全量备份
                    </Card.Title>
                    <Card.Description class="text-base mt-2">
                        将系统中的所有数据打包导出，生成 <code>.piney</code> 格式的备份文件。
                    </Card.Description>
                </Card.Header>
                <Card.Content class="space-y-6">
                    <div class="bg-muted/50 p-6 rounded-lg space-y-4 border border-border/50">
                        <h3 class="font-medium text-foreground">备份内容包含：</h3>
                        <ul class="grid grid-cols-2 gap-3 text-sm text-muted-foreground">
                            <li class="flex items-center gap-2">
                                <span class="h-1.5 w-1.5 rounded-full bg-primary/70"></span> 所有角色卡数据
                            </li>
                            <li class="flex items-center gap-2">
                                <span class="h-1.5 w-1.5 rounded-full bg-primary/70"></span> 所有上传的聊天记录
                            </li>
                            <li class="flex items-center gap-2">
                                <span class="h-1.5 w-1.5 rounded-full bg-primary/70"></span> 全局设置与偏好
                            </li>
                            <li class="flex items-center gap-2">
                                <span class="h-1.5 w-1.5 rounded-full bg-primary/70"></span> 世界书条目，等等...
                            </li>
                        </ul>
                    </div>

                    <div class="flex justify-end pt-4">
                        <Button size="lg" onclick={handleExport} class="w-full sm:w-auto font-bold text-lg px-8 shadow-lg shadow-primary/20">
                            <Download class="mr-2 h-5 w-5" />
                            立即备份数据
                        </Button>
                    </div>
                </Card.Content>
            </Card.Root>
        </Tabs.Content>

        <!-- Tab 2: Import -->
        <Tabs.Content value="import" class="space-y-6 focus-visible:outline-none">
            <Card.Root class="border-destructive/20 shadow-md">
                <Card.Header>
                    <Card.Title class="flex items-center gap-2 text-2xl text-destructive/90">
                        <RotateCcw class="h-6 w-6" />
                        全量数据恢复
                    </Card.Title>
                    <Card.Description class="text-base mt-2">
                        导入 <code>.piney</code> 备份文件，覆盖当前系统状态。
                    </Card.Description>
                </Card.Header>
                <Card.Content class="space-y-6">
                    
                    <div class="rounded-lg border border-destructive/20 bg-destructive/5 p-4 text-destructive-foreground">
                        <div class="flex items-center gap-2 mb-2">
                            <AlertTriangle class="h-5 w-5 text-destructive" />
                            <h5 class="font-bold tracking-wide text-destructive">警告：高风险操作</h5>
                        </div>
                        <div class="ml-7 text-sm opacity-90 text-destructive/90 leading-relaxed">
                            恢复操作将<strong>完全清除</strong>当前的数据库、角色卡文件和所有配置，并用备份文件中的数据进行<strong>覆盖</strong>。<br/>
                            此操作一旦开始，<strong>无法撤销</strong>。请务必确认您已备份了当前的重要数据。
                        </div>
                    </div>

                    <input 
                        bind:this={fileInput}
                        type="file" 
                        accept=".piney" 
                        class="hidden" 
                        onchange={handleFileSelect}
                    />

                    <div class="flex flex-col items-center justify-center p-10 border-2 border-dashed border-border rounded-xl bg-card hover:bg-accent/30 transition-colors cursor-pointer group"
                         onclick={triggerFileInput} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && triggerFileInput()}
                    >
                        <div class="w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4 group-hover:scale-110 transition-transform duration-300">
                            <FileUp class="h-8 w-8 text-muted-foreground group-hover:text-primary transition-colors" />
                        </div>
                        <h3 class="text-lg font-semibold mb-1 group-hover:text-primary transition-colors">点击选择备份文件</h3>
                        <p class="text-sm text-muted-foreground">支持 .piney 格式</p>
                    </div>

                </Card.Content>
            </Card.Root>
        </Tabs.Content>
    </Tabs.Root>

    <!-- Restore Confirmation Dialog -->
    <AlertDialog.Root open={isRestoreDialogOpen} onOpenChange={handleRestoreCancel}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title class="flex items-center gap-2 text-destructive">
                    <AlertTriangle class="h-5 w-5" />
                    确定要恢复数据吗？
                </AlertDialog.Title>
                <AlertDialog.Description class="space-y-3 pt-2">
                    <p>
                        您选择了备份文件：<span class="font-mono bg-muted px-1 py-0.5 rounded text-foreground">{selectedFile?.name}</span>
                    </p>
                    <p>
                        此操作将 <strong class="text-destructive">永久删除</strong> 当前系统中的所有数据，并使用备份文件进行覆盖。
                    </p>
                    <p class="font-bold">
                        操作无法撤销。请再次确认。
                    </p>
                </AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={handleRestoreCancel}>取消</AlertDialog.Cancel>
                <AlertDialog.Action onclick={handleRestoreConfirm} class="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                    确认恢复
                </AlertDialog.Action>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>
</div>
