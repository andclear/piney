<script lang="ts">
    import { onMount } from "svelte";
    import * as Card from "$lib/components/ui/card";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Textarea } from "$lib/components/ui/textarea";
    import { Label } from "$lib/components/ui/label";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { toast } from "svelte-sonner";
    import {
        History,
        Plus,
        RotateCcw,
        Trash2,
        Loader2,
        Info,
        GitCommit,
    } from "lucide-svelte";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { API_BASE } from "$lib/api";
    import { Badge } from "$lib/components/ui/badge";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { format } from "date-fns";
    import { cn } from "$lib/utils";

    interface Props {
        cardId: string;
        onRestore?: () => void;
        onClaim?: () => void; // 认领后回调
        currentVersion?: string | null;
        source?: string; // "import" | "local"
    }

    let { cardId, onRestore, onClaim, currentVersion = null, source = "import" }: Props = $props();

    interface Version {
        id: string;
        version_number: string;
        note: string;
        created_at: string;
    }

    let versions = $state<Version[]>([]);
    let loading = $state(true);

    // Create Version Dialog
    let isCreateDialogOpen = $state(false);
    let isCreating = $state(false);
    let newVersionNumber = $state("");
    let newVersionNote = $state("");

    // Restore Dialog
    let isRestoreDialogOpen = $state(false);
    let versionToRestore = $state<Version | null>(null);
    let isRestoring = $state(false);

    // Delete Dialog
    let isDeleteDialogOpen = $state(false);
    let versionToDelete = $state<Version | null>(null);
    let isDeleting = $state(false);

    // Claim Ownership Dialog
    let isClaimDialogOpen = $state(false);
    let isClaiming = $state(false);

    onMount(() => {
        loadVersions();
    });

    async function loadVersions() {
        try {
            loading = true;
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}/versions`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                versions = await res.json();
            } else {
                console.error("Failed to load versions");
            }
        } catch (e) {
            console.error(e);
            toast.error("加载版本历史失败");
        } finally {
            loading = false;
        }
    }

    function openCreateDialog() {
        if (!newVersionNumber) newVersionNumber = "V";
        isCreateDialogOpen = true;
    }

    async function handleCreateVersion() {
        const versionRegex = /^[Vv]?[0-9.]+$/;
        if (!versionRegex.test(newVersionNumber)) {
            toast.error("版本号格式错误：仅允许字母V、数字和小数点（例如 V1.0.1）");
            return;
        }
        
        // Validate note is required
        if (!newVersionNote.trim()) {
            toast.error("请填写版本备注");
            return;
        }
        
        let finalVersion = newVersionNumber;
        if (finalVersion.startsWith('v')) {
            finalVersion = 'V' + finalVersion.substring(1);
        }
        if (!finalVersion.startsWith('V')) {
            finalVersion = 'V' + finalVersion;
        }

        try {
            isCreating = true;
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}/versions`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({
                    version_number: finalVersion,
                    note: newVersionNote,
                }),
            });

            if (!res.ok) throw new Error("创建版本失败");

            toast.success("版本已创建");
            isCreateDialogOpen = false;
            newVersionNumber = "";
            newVersionNote = "";
            loadVersions();
        } catch (e) {
            console.error(e);
            toast.error("创建失败", { description: String(e) });
        } finally {
            isCreating = false;
        }
    }

    async function confirmRestore() {
        if (!versionToRestore) return;
        try {
            isRestoring = true;
            const token = localStorage.getItem("auth_token");
            const res = await fetch(
                `${API_BASE}/api/cards/${cardId}/versions/${versionToRestore.id}/restore`,
                {
                    method: "POST",
                    headers: token ? { Authorization: `Bearer ${token}` } : {},
                },
            );

            if (!res.ok) throw new Error("恢复版本失败");

            toast.success(`已切换至版本 ${versionToRestore.version_number}`);
            isRestoreDialogOpen = false;
            if (onRestore) onRestore();
        } catch (e) {
            console.error(e);
            toast.error("恢复失败", { description: String(e) });
        } finally {
            isRestoring = false;
        }
    }

    async function confirmDelete() {
        if (!versionToDelete) return;
        try {
            isDeleting = true;
            const token = localStorage.getItem("auth_token");
            const res = await fetch(
                `${API_BASE}/api/cards/${cardId}/versions/${versionToDelete.id}`,
                {
                    method: "DELETE",
                    headers: token ? { Authorization: `Bearer ${token}` } : {},
                },
            );

            if (!res.ok) throw new Error("删除版本失败");

            toast.success("版本已删除");
            isDeleteDialogOpen = false;
            loadVersions();
        } catch (e) {
            console.error(e);
            toast.error("删除失败", { description: String(e) });
        } finally {
            isDeleting = false;
        }
    }

    // 认领作品 - 将 source 从 import 改为 local
    async function confirmClaim() {
        try {
            isClaiming = true;
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ source: "local" }),
            });

            if (!res.ok) throw new Error("认领失败");

            toast.success("已成功认领此作品");
            isClaimDialogOpen = false;
            if (onClaim) onClaim();
        } catch (e) {
            console.error(e);
            toast.error("认领失败", { description: String(e) });
        } finally {
            isClaiming = false;
        }
    }

    function formatDate(dateStr: string) {
        if (!dateStr) return "";
        const d = dateStr.endsWith("Z") ? new Date(dateStr) : new Date(dateStr + "Z");
        return format(d, "yyyy-MM-dd HH:mm"); // Removed seconds as requested
    }

    function isCurrent(v: Version) {
        if (!currentVersion) {
            return v.version_number === "V1" || v.version_number === "V1.0";
        }
        return v.version_number === currentVersion;
    }
</script>

<div class="space-y-6">
    <div class="flex items-center justify-between">
        <div>
            <h3 class="text-lg font-medium">版本历史</h3>
            <p class="text-sm text-muted-foreground">
                管理角色卡的快照版本。您可以随时创建新版本或回退旧版本。
            </p>
        </div>
        <Button onclick={openCreateDialog} class="gap-2 shadow-sm">
            <Plus class="h-4 w-4" /> 创建版本
        </Button>
    </div>

    {#if loading}
        <div class="flex flex-col items-center justify-center py-16 text-muted-foreground">
            <Loader2 class="h-8 w-8 animate-spin mb-3 text-primary/60" />
            <p>正在加载历史记录...</p>
        </div>
    {:else if versions.length === 0}
        <div class="flex flex-col items-center justify-center py-16 border-2 border-dashed rounded-xl bg-muted/20">
            <History class="h-10 w-10 text-muted-foreground/40 mb-3" />
            <p class="text-muted-foreground font-medium">暂无版本历史</p>
            <p class="text-xs text-muted-foreground mt-1">
                点击右上角的“创建版本”来保存当前的第一个快照。
            </p>
        </div>
    {:else}
        <ScrollArea class="h-[600px] pr-4">
            <div class="relative py-4 pl-4"> 
                <!-- Main Container for Timeline -->
                {#each versions as version, i (version.id)}
                    <div class="flex gap-6 relative pb-10 last:pb-0 group">
                        <!-- 1. Axis Column -->
                        <div class="flex flex-col items-center relative flex-none w-6">
                            <!-- Line connecting to next (except last) -->
                            {#if i !== versions.length - 1}
                                <div class="absolute top-3 bottom-[-40px] w-px bg-border group-hover:bg-primary/20 transition-colors"></div>
                            {/if}
                            
                            <!-- Dot -->
                            <div 
                                class={cn(
                                    "relative z-10 h-3.5 w-3.5 rounded-full border-[3px] transition-all duration-300 mt-1.5",
                                    isCurrent(version) 
                                        ? "bg-primary border-primary ring-4 ring-primary/10 scale-110" 
                                        : "bg-background border-muted-foreground/30 group-hover:border-primary group-hover:scale-110"
                                )}
                            ></div>
                        </div>

                        <!-- 2. Content Column -->
                        <div class="flex-1 min-w-0">
                            <!-- Card -->
                            <div class={cn(
                                "rounded-xl border p-4 transition-all duration-300",
                                isCurrent(version) 
                                    ? "bg-primary/5 border-primary/30 shadow-md"
                                    : "bg-card border-border/60 hover:border-primary/30 hover:shadow-sm"
                            )}>
                                <!-- Header: Version + Date -->
                                <div class="flex items-center justify-between mb-2">
                                    <div class="flex items-center gap-3">
                                        <div class="flex items-center gap-2">
                                            <h4 class={cn("font-mono font-bold text-lg", isCurrent(version) ? "text-primary" : "")}>
                                                {version.version_number}
                                            </h4>
                                            {#if isCurrent(version)}
                                                <Badge variant="default" class="h-5 px-1.5 text-[10px] font-normal shadow-none pointer-events-none">当前使用</Badge>
                                            {/if}
                                        </div>
                                    </div>
                                    <span class="text-xs font-mono text-muted-foreground/70 bg-muted/50 px-2 py-1 rounded-md">
                                        {formatDate(version.created_at)}
                                    </span>
                                </div>

                                <!-- Note -->
                                {#if version.note}
                                    <p class="text-sm text-foreground/80 leading-normal mb-1">
                                        {version.note}
                                    </p>
                                {:else}
                                    <p class="text-xs text-muted-foreground/40 italic mb-1">
                                        无备注信息
                                    </p>
                                {/if}

                                <!-- Actions Bar -->
                                <div class="flex items-center justify-end gap-2 pt-2 border-t border-border/30 opacity-60 group-hover:opacity-100 transition-opacity">
                                    {#if !isCurrent(version)}
                                        <Button
                                            variant="secondary"
                                            size="sm"
                                            class="h-7 text-xs gap-1.5"
                                            onclick={() => {
                                                versionToRestore = version;
                                                isRestoreDialogOpen = true;
                                            }}
                                        >
                                            <RotateCcw class="h-3 w-3" />
                                            使用此版本
                                        </Button>
                                    {/if}
                                    {#if !isCurrent(version)}
                                     <Button
                                        variant="ghost"
                                        size="icon"
                                        class="h-7 w-7 text-muted-foreground hover:text-destructive hover:bg-destructive/10"
                                        onclick={() => {
                                            versionToDelete = version;
                                            isDeleteDialogOpen = true;
                                        }}
                                    >
                                        <Trash2 class="h-3.5 w-3.5" />
                                    </Button>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        </ScrollArea>
    {/if}

    <!-- 认领作品链接 (仅导入的卡片显示) -->
    {#if source === "import"}
        <div class="pt-6 mt-4 border-t border-dashed text-center">
            <button
                class="text-xs text-muted-foreground/70 hover:text-muted-foreground transition-colors cursor-pointer underline-offset-2 hover:underline"
                onclick={() => isClaimDialogOpen = true}
            >
                这是您自己创建的角色卡？点击这里认领作品，然后即可以在设定标签页中编辑署名
            </button>
        </div>
    {/if}
</div>

<!-- Create Version Dialog -->
<Dialog.Root bind:open={isCreateDialogOpen}>
    <Dialog.Content class="sm:max-w-[425px]">
        <Dialog.Header>
            <Dialog.Title>创建版本快照</Dialog.Title>
            <Dialog.Description>
                将当前角色卡的所有数据保存为一个新的历史版本。
            </Dialog.Description>
        </Dialog.Header>
        
        <div class="bg-blue-500/10 text-blue-500 border border-blue-500/20 rounded-lg p-3 text-xs flex gap-2">
            <Info class="h-4 w-4 shrink-0 mt-0.5" />
            <div class="space-y-1">
                <p class="font-semibold">版本管理说明</p>
                <p>保存版本不会覆盖当前编辑状态，而是创建一个“备份点”。您可以随时回滚到此版本。</p>
            </div>
        </div>

        <div class="grid gap-4 py-4">
            <div class="grid gap-2">
                <Label for="version_num">版本号</Label>
                <Input
                    id="version_num"
                    bind:value={newVersionNumber}
                    placeholder="例如 V1.0"
                    class="font-mono"
                />
                <p class="text-[10px] text-muted-foreground">
                    仅允许大写V、数字和小数点，但建议写整数版本 (如 V2)
                </p>
            </div>
            <div class="grid gap-2">
                <Label for="note">备注 <span class="text-destructive">*</span></Label>
                <Textarea
                    id="note"
                    bind:value={newVersionNote}
                    placeholder="记录本次版本的变更重点..."
                    class="h-20 resize-none"
                    required
                />
            </div>
        </div>

        <Dialog.Footer>
            <Button variant="ghost" onclick={() => (isCreateDialogOpen = false)}>取消</Button>
            <Button onclick={handleCreateVersion} disabled={isCreating}>
                {#if isCreating}
                    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                    创建中...
                {:else}
                    创建版本
                {/if}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>

<!-- Restore Alert Dialog -->
<AlertDialog.Root bind:open={isRestoreDialogOpen}>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>确认切换版本？</AlertDialog.Title>
            <AlertDialog.Description>
                您即将把当前角色卡重置为版本 <span class="font-mono font-bold text-foreground">{versionToRestore?.version_number}</span>。
                <br/><br/>
                <span class="text-destructive font-semibold">警告：</span> 当前所有未保存或未创建版本的修改将会丢失！建议先创建一个当前状态的快照。
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
            <AlertDialog.Cancel>取消</AlertDialog.Cancel>
            <AlertDialog.Action 
                class="bg-destructive hover:bg-destructive/90"
                onclick={confirmRestore}
                disabled={isRestoring}
            >
                {#if isRestoring}
                    <Loader2 class="h-4 w-4 animate-spin mr-2" />
                    正在恢复...
                {:else}
                    确认切换/恢复
                {/if}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<!-- Delete Alert Dialog -->
<AlertDialog.Root bind:open={isDeleteDialogOpen}>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>删除版本记录</AlertDialog.Title>
            <AlertDialog.Description>
                确定要永久删除版本 <span class="font-mono font-bold text-foreground">{versionToDelete?.version_number}</span> 吗？此操作无法撤销。
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
            <AlertDialog.Cancel>取消</AlertDialog.Cancel>
            <AlertDialog.Action 
                class="bg-destructive hover:bg-destructive/90"
                onclick={confirmDelete}
                disabled={isDeleting}
            >
                {#if isDeleting}
                    <Loader2 class="h-4 w-4 animate-spin mr-2" />
                    删除中...
                {:else}
                    确认删除
                {/if}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<!-- Claim Ownership Alert Dialog -->
<AlertDialog.Root bind:open={isClaimDialogOpen}>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>认领作品</AlertDialog.Title>
            <AlertDialog.Description class="space-y-3">
                <p>
                    此操作专用于作者找回自己作品的署名编辑权限。
                </p>
                <p class="text-destructive font-medium">
                    修改他人作品的署名是不礼貌的行为。
                </p>
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
            <AlertDialog.Cancel>取消</AlertDialog.Cancel>
            <AlertDialog.Action 
                onclick={confirmClaim}
                disabled={isClaiming}
            >
                {#if isClaiming}
                    <Loader2 class="h-4 w-4 animate-spin mr-2" />
                    认领中...
                {:else}
                    我确认为原作者，认领作品
                {/if}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>
