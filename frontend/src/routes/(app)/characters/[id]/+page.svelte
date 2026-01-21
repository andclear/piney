<script lang="ts">
    import { page } from "$app/stores";
    import { beforeNavigate, goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { AiService } from "$lib/ai/service";
    import * as Card from "$lib/components/ui/card";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Textarea } from "$lib/components/ui/textarea";
    import { Label } from "$lib/components/ui/label";
    import { Badge } from "$lib/components/ui/badge";
    import { Separator } from "$lib/components/ui/separator";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import { toast } from "svelte-sonner";
    import {
        ArrowLeft,
        Download,
        GitBranch,
        History,
        Sparkles,
        FileText,
        Globe,
        Regex,
        IdCard,
        Upload,
        ChevronLeft,
        X,
        Check,
        Stethoscope,
        ScrollText,
        Loader2,
        Save,
        AlertTriangle,
        User,
        MessageSquareQuote,
        Map,
        StickyNote,
        Terminal,
        Trash2,
        Bot,
    } from "lucide-svelte";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { cn } from "$lib/utils";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
    import RichTextarea from "$lib/components/character/RichTextarea.svelte";
    import GreetingsSwitcher from "$lib/components/character/GreetingsSwitcher.svelte";
    import WorldInfoTab from "$lib/components/character/world_info/WorldInfoTab.svelte";
    import ImageCropperDialog from "$lib/components/ui/ImageCropperDialog.svelte";
    import RegexTab from "$lib/components/character/regex/RegexTab.svelte";
    import ChatHistoryTab from "$lib/components/character/history/ChatHistoryTab.svelte";
    import VersionHistoryTab from "$lib/components/character/versions/VersionHistoryTab.svelte";
    import DoctorDialog from "$lib/components/character/DoctorDialog.svelte";
    import { doctorTasks } from "$lib/ai/doctor";
    import { AiFeature } from "$lib/ai/types";

    import { API_BASE, resolveUrl } from "$lib/api";
    let cardId = $page.params.id;
    let card: any = null;
    let loading = true;
    let activeTab = $page.url.searchParams.get("tab") || "overview";

    // Doctor Dialog State
    let showDoctorDialog = false;
    $: isDoctorRunning = $doctorTasks[cardId]?.status === 'analyzing';

    // Data for Overview
    let editingNote = "";
    let editingSummary = "";
    let isSavingNote = false;
    let coverInput: HTMLInputElement;
    let avatarKey: number | null = null;
    
    // Cropper State
    let showCropper = false;
    let cropperImageSrc: string | null = null;
    let selectedFileType = "image/png";

    // Persona Tab State
    let formName = "";
    let formDescription = "";
    let formCreator = "";
    let formVersion = "";
    let formFirstMes = "";
    let formAltGreetings: string[] = [];
    let formScenario = "";
    let formMesExample = "";
    let formPersonality = "";
    let isSavingPersona = false;
    let originalFormState = {
        name: "",
        description: "",
        creator: "",
        firstMes: "",
        altGreetings: [] as string[],
        scenario: "",
        mesExample: "",
        personality: "",
    };
    let originalWorldInfoState = "{}"; // Store as JSON string for easy deep comparison

    // World Info State
    let isSavingWorldInfo = false;
    let lastSaved = Date.now();
    
    // 额外设定项（对话示例、世界观与逻辑）- 仅 source=local 时使用
    let showExtraSettings = false;

    // AI Generation State
    let isGenDialogOpen = false;
    let genInput = "";
    let genUseYaml = false;
    let genIncludeWorldInfo = false;
    let isGenerating = false;
    let isDescZenMode = false;

    function handleGenDialogChange(open: boolean) {
        if (!open && isGenerating) {
             toast.warning("AI 正在生成中，关闭窗口可能会中断任务");
             // We return, but shadcn Dialog might close internally if we don't control it via `open` prop.
             // With `bind:open={isGenDialogOpen}`, preventing update requires setting it back to true? 
             // Actually `onOpenChange` is the event. If we just bind, we can't intercept easily.
             // We will switch to `open={...} onOpenChange={...}` in markup next.
             // Here we update state.
             return; 
        }
        isGenDialogOpen = open;
    }

    async function handleGenerateCharacter() {
        if (!genInput.trim()) {
            toast.error("请输入描述内容");
            return;
        }

        // isGenDialogOpen = false; // Keep open
        
        // isDescZenMode = true; // Remove zen mode
        isGenerating = true;
        let accumulatedContent = "";

        // let toastId = toast.loading("AI 正在构建角色..."); // No global toast, use dialog UI

        // Build World Info Context
        let worldInfoContext = "";
        // Try V2 path first, then V1/Root path
        const wb = card.data?.data?.character_book || card.data?.character_book;
        let entriesData: any[] = [];
        if (wb?.entries) {
            if (Array.isArray(wb.entries)) {
                 entriesData = wb.entries;
            } else {
                 entriesData = Object.values(wb.entries);
            }
        }

        // Filter enabled entries (checking both enabled/disable flags)
        const enabledEntries = entriesData.filter((e: any) => e.enabled !== false && e.disable !== true); 

        if (genIncludeWorldInfo && enabledEntries.length > 0) {
            const formattedEntries = enabledEntries.map((e: any, index: number) => {
                const title = e.comment || `条目 ${index + 1}`;
                const content = e.content || "";
                return `Name: ${title}\nContent: ${content}`;
            }).join("\n---\n");
            
            worldInfoContext = `## 世界观背景资料 (World Setting)\n（若此处有内容，请务必基于以下设定构建角色的背景与能力，严禁产生设定冲突）\n${formattedEntries}`;
        }

        try {
            const content = await AiService.generateCharacter(
                genInput,
                genUseYaml,
                worldInfoContext
            );
            formDescription = content;
            toast.success("角色生成完成");
            isGenDialogOpen = false;
        } catch (err: any) {
             toast.error("生成失败: " + (err.message || err));
        } finally {
            isGenerating = false;
        }
    }

    function openGenDialog() {
        genInput = "";
        genUseYaml = false;
        isGenDialogOpen = true;
    }

    function updateFormSnapshot() {
        originalFormState = {
            name: formName,
            description: formDescription,
            creator: formCreator,
            firstMes: formFirstMes,
            altGreetings: JSON.parse(JSON.stringify(formAltGreetings)), // Deep copy
            scenario: formScenario,
            mesExample: formMesExample,
            personality: formPersonality,
        };
        // Update World Info snapshot
        originalWorldInfoState = JSON.stringify({
            entries: card?.data?.data?.character_book?.entries || [],
            extensions: {
                ...card?.data?.data?.extensions,
                regex_scripts: card?.data?.data?.extensions?.regex_scripts || []
            }
        });
        lastSaved = Date.now();
    }

    // Granular Dirty States
    let isNameDirty = false;
    let isDescDirty = false;
    let isGreetingsDirty = false;
    let isScenarioDirty = false;
    let isMesExampleDirty = false;
    let isPersonalityDirty = false;
    let isWorldInfoDirty = false;
    let isDirty = false;

    $: {
        isNameDirty = formName !== originalFormState.name;
        isDescDirty = formDescription !== originalFormState.description;
        isGreetingsDirty =
            formFirstMes !== originalFormState.firstMes ||
            JSON.stringify(formAltGreetings) !==
                JSON.stringify(originalFormState.altGreetings);
        isScenarioDirty = formScenario !== originalFormState.scenario;
        isMesExampleDirty = formMesExample !== originalFormState.mesExample;
        isPersonalityDirty = formPersonality !== originalFormState.personality;

        // Normalize for comparison only (do not mutate source)
        const currentWorldInfoState = JSON.stringify({
            entries: card?.data?.data?.character_book?.entries || [],
            extensions: {
                ...card?.data?.data?.extensions,
                regex_scripts: card?.data?.data?.extensions?.regex_scripts || []
            }
        });
        
        // Also normalize original snapshot during comparison
        // (Wait, originalWorldInfoState is a string needed for robust comp)
        // We need to parse original, normalize, and compare, OR construct original with normalization.
        
        // Actually, better to just normalize the structures before stringifying.
        // Let's rely on the snapshot being "correct" at load time, 
        // but if load time had undefined regex_scripts, snapshot has undefined.
        // If current state (due to some other component) has [], then we have mismatch.
        
        // The issue is that RegexTab MIGHT still be setting it if we add a script?
        // But if we just view page, we shouldn't set it.
        // If RegexTab is rendered, we removed the forced init.
        // So `card.data.data.extensions.regex_scripts` should remain undefined if it was undefined.
        
        isWorldInfoDirty = currentWorldInfoState !== originalWorldInfoState;

        isDirty =
            isNameDirty ||
            isDescDirty ||
            isGreetingsDirty ||
            isScenarioDirty ||
            isMesExampleDirty ||
            isPersonalityDirty ||
            isWorldInfoDirty;
    }

    function handleBeforeUnload(e: BeforeUnloadEvent) {
        if (isDirty) {
            e.preventDefault();
            e.returnValue = "";
            return "";
        }
    }

    // Unsaved Changes Dialog State
    let showUnsavedDialog = false;
    let pendingTarget: string | null = null;
    let bypassCheck = false;

    beforeNavigate(({ cancel, to }) => {
        if (bypassCheck) return;
        if (isDirty) {
            cancel();
            pendingTarget = to?.url?.href || null;
            showUnsavedDialog = true;
        }
    });

    function confirmDiscard() {
        bypassCheck = true;
        showUnsavedDialog = false;
        if (pendingTarget) {
            goto(pendingTarget);
        }
    }

    function cancelDiscard() {
        showUnsavedDialog = false;
        pendingTarget = null;
    }

    // Delete Card State
    let showDeleteDialog = false;

    // AI Overview Generation
    let isGeneratingOverview = false;
    async function generateOverview() {
        if (isGeneratingOverview) return;
        isGeneratingOverview = true;
        toast.info("正在通过 AI 分析角色卡...", { duration: 2000 });

        try {
            // Use New AI Service Layer
            // The prompt is now built on the client side
            const result = await AiService.generateOverview(card);

            // Success handling
            card.custom_summary = result.summary;
            editingSummary = result.summary;

            let successMsg = "概览已更新";

            if (result.tags && Array.isArray(result.tags)) {
                tags = result.tags;
                card.tags = JSON.stringify(tags);
                successMsg = "概览与标签生成成功";
                
                // We need to update the `viewData` (parsed JSON) if it exists
                // The DB "data" field is a strings, but `card.data` in the app is an object (parsed in loadCard)
                try {
                    let currentData = typeof card.data === 'string' ? JSON.parse(card.data) : (card.data || {});
                    
                    // Update tags in data JSON (support V1 root tags & V2 data.tags)
                    // Root
                    currentData.tags = tags;
                    // V2
                    if (currentData.data) {
                         currentData.data.tags = tags;
                    }
                    
                    // CRITICAL: Assign back as OBJECT to prevent breaking reactive dirty checks (which expect object access)
                    card.data = currentData;
                } catch (jsonErr) {
                    console.error("Failed to sync tags to data JSON", jsonErr);
                }
            }

            // Trigger Save to persist changes to DB
            const token = localStorage.getItem("auth_token");
            const updatePayload: any = { custom_summary: result.summary };
            if (result.tags && Array.isArray(result.tags)) {
                updatePayload.tags = result.tags;
            }

            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify(updatePayload),
            });

            if (!res.ok) throw new Error("保存概览失败");
            
            toast.success(successMsg);
            
            // Sync snapshot so it doesn't show as dirty
            updateFormSnapshot();

        } catch (e) {
            console.error(e);
            toast.error("生成失败", { description: String(e) });
        } finally {
            isGeneratingOverview = false;
        }
    }

    // Tags Management
    let tags: string[] = [];
    let isEditingTags = false;
    let newTag = "";

    async function saveTags(newTags: string[]) {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ tags: newTags }), // Send array directly, backend handles JSON serialization
            });
            if (!res.ok) {
                const errText = await res.text();
                throw new Error(`Status: ${res.status}, Body: ${errText}`);
            }
            tags = newTags;
            // Also update local card object
            card.tags = JSON.stringify(newTags);

            // Sync tags to card.data (Object) as well to prevent consistency issues
            try {
                let currentData = typeof card.data === 'string' ? JSON.parse(card.data) : (card.data || {});
                currentData.tags = newTags;
                if (currentData.data) {
                     currentData.data.tags = newTags;
                }
                card.data = currentData;
            } catch (ignore) {}

            updateFormSnapshot();
            toast.success("标签已更新");
        } catch (e) {
            console.error(e);
            toast.error("保存标签失败", { description: String(e) });
            // Revert or reload?
        }
    }

    function addTag() {
        if (!newTag.trim()) return;
        if (tags.includes(newTag.trim())) {
            toast.error("标签已存在");
            return;
        }
        const updated = [...tags, newTag.trim()];
        saveTags(updated);
        newTag = "";
        // Don't close edit mode, allow adding more
    }

    function removeTag(tagToRemove: string) {
        const updated = tags.filter((t) => t !== tagToRemove);
        saveTags(updated);
    }

    onMount(async () => {
        await loadCard();
    });

    async function loadCard() {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (!res.ok) throw new Error("加载角色卡失败");
            card = await res.json();

            // Use updated_at timestamp for cache busting on initial load
            if (!avatarKey && card.updated_at) {
                avatarKey = new Date(card.updated_at).getTime();
            }

            breadcrumbs.set([
                { label: "角色库", href: "/characters" },
                { label: card.name || "详细信息" },
            ]);

            editingNote = card.user_note || "";
            editingSummary = card.custom_summary || "";
            tags = tryParseTags(card.tags || "[]");

            // Parse JSON Data for Persona Tab
            try {
                if (typeof card.data === "string") {
                    card.data = JSON.parse(card.data || "{}");
                }
                const jsonData = card.data || {};
                const v2Data = jsonData.data || {};

                // Mapping Logic
                formName = card.name || "";
                formDescription = card.description || "";

                // Priority: V2 -> Root -> Fallback
                formFirstMes = v2Data.first_mes || jsonData.first_mes || "";
                formAltGreetings = v2Data.alternate_greetings || [];
                formMesExample =
                    v2Data.mes_example || jsonData.mes_example || "";
                formScenario = v2Data.scenario || jsonData.scenario || "";
                formPersonality = v2Data.personality || jsonData.personality || "";
                formVersion = v2Data.character_version || "";
                formVersion = v2Data.character_version || "";
                formCreator =
                    v2Data.creator || jsonData.creator || card.author || "";

                updateFormSnapshot();
                
                // 从 localStorage 读取额外设定项开关状态
                const extraSettingsKey = `piney_extra_settings_${cardId}`;
                showExtraSettings = localStorage.getItem(extraSettingsKey) === "true";
            } catch (jsonErr) {
                console.error("Failed to parse card data JSON", jsonErr);
                toast.error("角色卡数据解析失败，部分字段可能无法显示");
            }
        } catch (e) {
            console.error(e);
            toast.error("加载失败", { description: String(e) });
        } finally {
            loading = false;
        }
    }

    async function saveNote() {
        isSavingNote = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({ user_note: editingNote }),
            });
            if (!res.ok) throw new Error("保存失败");
            await loadCard();
            toast.success("备注已保存");
        } catch (e) {
            toast.error("保存失败");
        } finally {
            isSavingNote = false;
        }
    }

    async function savePersona() {
        isSavingPersona = true;
        try {
            const token = localStorage.getItem("auth_token");
            const payload = {
                name: formName,
                description: formDescription,
                first_mes: formFirstMes,
                alternate_greetings: formAltGreetings,
                mes_example: formMesExample,
                scenario: formScenario,
                personality: formPersonality,
                character_version: formVersion,
                creator: formCreator, // 创作者（仅 source=local 时会发送）
            };

            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify(payload),
            });

            if (!res.ok) throw new Error("保存失败");

            if (card.name !== formName) {
                breadcrumbs.set([
                    { label: "角色库", href: "/characters" },
                    { label: formName },
                ]);
            }

            await loadCard();
            toast.success("设定已保存");
        } catch (e) {
            console.error(e);
            toast.error("保存设定失败", { description: String(e) });
        } finally {
            isSavingPersona = false;
            updateFormSnapshot();
        }
    }

    async function saveWorldInfo() {
        isSavingWorldInfo = true;
        try {
            const token = localStorage.getItem("auth_token");
            const wbData = card.data?.data?.character_book;
            const extData = card.data?.data?.extensions;

            // Send both character_book and extensions (for world name sync)
            const payload: Record<string, unknown> = {
                character_book: wbData,
            };
            
            // 仅当 extensions 存在且有 world 字段时发送 extensions
            // 这确保世界书名称能正确保存
            if (extData && extData.world !== undefined) {
                payload.extensions = extData;
            }

            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify(payload),
            });

            if (!res.ok) throw new Error("保存失败");
            await loadCard();
            toast.success("世界书已保存");
            lastSaved = Date.now();
            updateFormSnapshot();
        } catch (e) {
            console.error(e);
            toast.error("保存世界书失败", { description: String(e) });
        } finally {
            isSavingWorldInfo = false;
        }
    }

    // Save Regex Scripts ONLY (does not affect other extension fields)
    async function saveRegex() {
        isSavingWorldInfo = true;
        try {
            const token = localStorage.getItem("auth_token");
            
            // Send ONLY regex_scripts field for partial update
            const payload = {
                regex_scripts: card.data?.data?.extensions?.regex_scripts || []
            };

            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify(payload),
            });

            if (!res.ok) throw new Error("保存失败");
            await loadCard();
            toast.success("正则已保存");
            lastSaved = Date.now();
            updateFormSnapshot();
        } catch (e) {
            console.error(e);
            toast.error("保存正则失败", { description: String(e) });
        } finally {
            isSavingWorldInfo = false;
        }
    }

    async function exportCard() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}/export`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (!res.ok) throw new Error("导出失败");
            
            const blob = await res.blob();
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            
            // Determine extension based on content-type
            const contentType = res.headers.get("content-type") || "";
            const ext = contentType.includes("application/json") ? "json" : "png";
            
            a.download = `${card.name || "character"}.${ext}`; 
            document.body.appendChild(a);
            a.click();
            window.URL.revokeObjectURL(url);
            document.body.removeChild(a);
        } catch (e) {
            toast.error("导出失败");
            console.error(e);
        }
    }

    async function handleCoverUpload(e: Event) {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (!file) return;

        // Reset input so same file can be selected again
        (e.target as HTMLInputElement).value = "";

        selectedFileType = file.type || "image/png";
        
        // Read file for cropper
        const reader = new FileReader();
        reader.onload = () => {
             if (typeof reader.result === 'string') {
                 cropperImageSrc = reader.result;
                 showCropper = true;
             }
        };
        reader.readAsDataURL(file);
    }

    async function handleCropConfirm(e: CustomEvent<Blob>) {
        const blob = e.detail;
        if (!blob) return;

        const formData = new FormData();
        // Use a generic name, backend handles persistence
        formData.append("file", blob, "cover.png"); 

        const loadingToast = toast.loading("正在更新封面...");
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}/cover`, {
                method: "POST",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
                body: formData,
            });
            if (!res.ok) throw new Error("上传失败");

            toast.success("封面更新成功");
            avatarKey = Date.now(); // Force refresh image
            await loadCard(); // Reload to get new version/avatar url
        } catch (e) {
            toast.error("更新封面失败");
        } finally {
            toast.dismiss(loadingToast);
        }
    }

    async function deleteCard() {
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
                method: "DELETE",
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (res.ok) {
                toast.success("已移至回收站");
                goto("/characters");
            } else {
                toast.error("删除失败");
            }
        } catch (e) {
            toast.error("删除失败");
            console.error(e);
        }
    }

    const menuItems = [
        { id: "overview", label: "概览", icon: FileText },
        { id: "persona", label: "设定", icon: IdCard },
        { id: "world_info", label: "世界书", icon: Globe }, 
        { id: "regex", label: "正则", icon: Regex },
        { id: "versions", label: "版本历史", icon: GitBranch }, // New Tab
        { id: "chat", label: "聊天记录", icon: History },
    ];

    // Helper function
    function tryParseTags(jsonStr: string): string[] {
        try {
            const parsed = JSON.parse(jsonStr);
            if (Array.isArray(parsed)) return parsed;
            return [];
        } catch {
            return [];
        }
    }
</script>

<div
    class="container h-[calc(100vh-4rem)] max-w-7xl py-6 flex flex-col md:flex-row gap-6"
>
    <!-- 左侧导航 (Mobile: 顶部横向滚动 / Desktop: 侧边栏) -->
    <div class="w-full md:w-32 flex-shrink-0 flex flex-col gap-4">
        <Button variant="ghost" class="w-fit -ml-2 mb-2" href="/characters">
            <ChevronLeft class="mr-2 h-4 w-4" /> 返回列表
        </Button>

        {#if card}{/if}

        <!-- 导航菜单 -->
        <div
            class="flex md:flex-col overflow-x-auto md:overflow-visible gap-2 pb-2 md:pb-0 scrollbar-hide"
        >
            {#each menuItems as item}
                <button
                    class={cn(
                        "flex items-center gap-3 px-4 py-2 rounded-lg text-sm font-medium transition-colors whitespace-nowrap",
                        activeTab === item.id
                            ? "bg-primary text-primary-foreground"
                            : "hover:bg-accent hover:text-accent-foreground text-muted-foreground",
                    )}
                    onclick={() => (activeTab = item.id)}
                >
                    <item.icon class="h-4 w-4" />
                    {item.label}
                </button>
            {/each}
        </div>
    </div>

    <Separator orientation="vertical" class="hidden md:block h-full" />

    <!-- 右侧内容区域 -->
    <div class="flex-1 min-h-0 overflow-y-auto pr-2">
        {#if loading}
            <div class="flex items-center justify-center h-64">
                <span class="loading loading-spinner text-primary"
                    >加载中...</span
                >
            </div>
        {:else if !card}
            <div class="text-center py-12 text-muted-foreground">
                未找到角色卡
            </div>
        {:else}
            <!-- 概览页 -->
            <div class={activeTab === "overview" ? "" : "hidden"}>
                <div
                    class="animate-in fade-in slide-in-from-bottom-4 duration-500"
                >
                    <div class="flex flex-col md:flex-row gap-8 items-start">
                        <!-- Left Column: Cover & Actions -->
                        <div
                            class="flex-shrink-0 w-40 mx-auto md:mx-0 space-y-4"
                        >
                            <div
                                class="aspect-[2/3] w-full rounded-xl overflow-hidden border bg-muted shadow-sm relative group"
                            >
                                <img
                                    src={resolveUrl(`${card.avatar || "/default.webp"}?t=${avatarKey}`)}
                                    alt="封面"
                                    class={cn(
                                        "w-full h-full object-cover transition-transform duration-500 group-hover:scale-105",
                                        card.cover_blur && "blur-xl",
                                    )}
                                />
                                <!-- Cover Overlay -->
                                <div
                                    class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center gap-2 text-white"
                                >
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        class="bg-transparent text-white border-white hover:bg-white hover:text-black h-8 text-xs"
                                        onclick={() => coverInput.click()}
                                    >
                                        <Upload class="mr-2 h-3 w-3" /> 更换封面
                                    </Button>
                                    <input
                                        type="file"
                                        bind:this={coverInput}
                                        onchange={handleCoverUpload}
                                        accept="image/*"
                                        class="hidden"
                                    />
                                    <p class="text-[10px] opacity-80">
                                        512x768
                                    </p>
                                </div>
                            </div>
                            <Button
                                class="w-full"
                                variant="outline"
                                size="sm"
                                onclick={exportCard}
                            >
                                <Download class="mr-2 h-4 w-4" /> 导出卡片
                            </Button>
                            <Button
                                class="w-full !text-destructive !hover:text-destructive !hover:bg-destructive/10"
                                variant="outline"
                                size="sm"
                                onclick={() => (showDeleteDialog = true)}
                            >
                                <Trash2 class="mr-2 h-4 w-4" /> 删除卡片
                            </Button>

                            <!-- Token Stats (Moved to Left) -->
                            <div
                                class="grid grid-cols-2 md:grid-cols-1 gap-2 pt-2"
                            >
                                {@render ReviewStat({
                                    label: "总 Token",
                                    value: card.token_count_total ?? '-',
                                    compact: true,
                                })}
                                {@render ReviewStat({
                                    label: "设定",
                                    value: card.token_count_spec ?? '-',
                                    compact: true,
                                    color: "text-blue-500"
                                })}
                                {@render ReviewStat({
                                    label: "世界书",
                                    value: card.token_count_wb ?? '-',
                                    compact: true,
                                    color: "text-purple-500"
                                })}
                                {@render ReviewStat({
                                    label: "其他",
                                    value: card.token_count_other ?? '-',
                                    compact: true,
                                    color: "text-muted-foreground"
                                })}
                            </div>
                        </div>

                        <!-- Right Column: Content -->
                        <div class="flex-1 w-full min-w-0 space-y-6">
                            <!-- Header Section -->
                            <div class="space-y-3">
                                <div
                                    class="flex flex-col md:flex-row md:items-end gap-3 justify-between"
                                >
                                    <div
                                        class="space-y-1 text-center md:text-left"
                                    >
                                        <h1
                                            class="text-3xl font-bold tracking-tight text-foreground"
                                        >
                                            {card.name}
                                        </h1>
                                        {#if card.author}
                                            <span>Created by {card.author}</span>
                                        {/if}
                                    </div>
                                </div>

                                <!-- Tags -->
                                <div
                                    class="flex flex-wrap gap-2 justify-center md:justify-start items-center min-h-[28px]"
                                >
                                    {#each tags as tag}
                                        <Badge
                                            variant="secondary"
                                            class="px-2 py-0.5 text-xs font-normal text-muted-foreground bg-muted hover:bg-muted/80 group flex items-center gap-1"
                                        >
                                            {tag}
                                            {#if isEditingTags}
                                                <button
                                                    class="text-muted-foreground hover:text-destructive transition-colors ml-0.5"
                                                    onclick={() =>
                                                        removeTag(tag)}
                                                    aria-label="Remove tag"
                                                >
                                                    <X class="h-3 w-3" />
                                                </button>
                                            {/if}
                                        </Badge>
                                    {/each}

                                    {#if isEditingTags}
                                        <div
                                            class="flex items-center gap-2 animate-in fade-in slide-in-from-left-2 duration-200"
                                        >
                                            <Input
                                                bind:value={newTag}
                                                placeholder="新标签..."
                                                class="h-6 text-xs w-24 px-2"
                                                onkeydown={(e) => {
                                                    if (e.key === "Enter") {
                                                        e.preventDefault();
                                                        addTag();
                                                    }
                                                }}
                                            />
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                class="h-6 w-6 p-0 text-muted-foreground hover:text-foreground"
                                                onclick={() => {
                                                    // Try to add pending tag if any
                                                    if (newTag.trim()) {
                                                        addTag();
                                                    }
                                                    isEditingTags = false;
                                                }}
                                                title="完成"
                                            >
                                                <Check class="h-3.5 w-3.5" />
                                            </Button>
                                        </div>
                                    {:else}
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            class="h-5 text-[10px] px-2 text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
                                            onclick={() =>
                                                (isEditingTags = true)}
                                        >
                                            + 编辑标签
                                        </Button>
                                    {/if}
                                </div>
                            </div>

                            <Separator />

                            <!-- Main Content Area -->
                            <div class="grid gap-6">
                                <!-- AI Overview -->
                                <div
                                    class="bg-muted/30 rounded-xl p-5 border border-border/40 space-y-3 relative group"
                                >
                                    <div
                                        class="flex items-center justify-between font-medium text-sm text-foreground/80"
                                    >
                                        <div class="flex items-center gap-2">
                                            <Sparkles
                                                class="h-4 w-4 text-primary"
                                            />
                                            <span>AI 智能概览</span>
                                        </div>
                                        <div
                                            class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
                                        >
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                class="h-6 w-6 text-muted-foreground hover:text-foreground"
                                                title="重新生成"
                                                disabled={isGeneratingOverview}
                                                onclick={generateOverview}
                                            >
                                                {#if isGeneratingOverview}
                                                    <Loader2
                                                        class="h-3.5 w-3.5 animate-spin"
                                                    />
                                                {:else}
                                                    <Sparkles
                                                        class="h-3.5 w-3.5"
                                                    />
                                                {/if}
                                            </Button>
                                        </div>
                                    </div>
                                    {#if card.custom_summary}
                                        <p
                                            class="text-sm leading-relaxed text-muted-foreground/90 animate-in fade-in"
                                        >
                                            {card.custom_summary}
                                        </p>
                                    {:else}
                                        <div
                                            class="text-center py-6 border border-dashed rounded-lg bg-background/50 flex flex-col items-center justify-center gap-3"
                                        >
                                            <p
                                                class="text-xs text-muted-foreground"
                                            >
                                                暂无 AI
                                                概览内容，点击下方按钮生成。
                                            </p>
                                            <Button
                                                size="sm"
                                                variant="secondary"
                                                class="h-8 text-xs gap-2"
                                                disabled={isGeneratingOverview}
                                                onclick={generateOverview}
                                            >
                                                {#if isGeneratingOverview}
                                                    <Loader2
                                                        class="h-3.5 w-3.5 animate-spin"
                                                    />
                                                    生成中...
                                                {:else}
                                                    <Sparkles
                                                        class="h-3.5 w-3.5"
                                                    />
                                                    生成概览
                                                {/if}
                                            </Button>
                                        </div>
                                    {/if}
                                </div>

                                <!-- User Note -->
                                <div class="space-y-2">
                                    <div
                                        class="flex items-center justify-between"
                                    >
                                        <Label
                                            class="text-xs font-medium text-muted-foreground"
                                            >个人备注</Label
                                        >
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            class="h-6 text-xs hover:bg-transparent hover:text-primary p-0"
                                            onclick={saveNote}
                                            disabled={isSavingNote}
                                        >
                                            {isSavingNote
                                                ? "保存中..."
                                                : "保存更改"}
                                        </Button>
                                    </div>
                                    <Textarea
                                        bind:value={editingNote}
                                        placeholder="记录一些关于角色的想法..."
                                        class="min-h-[100px] resize-none bg-background/50 focus:bg-background transition-colors text-sm"
                                    />
                                </div>

                                <!-- Dr. Piney (Review System) -->
                                <div
                                    class="bg-primary/5 rounded-xl p-5 border border-primary/20 flex flex-col sm:flex-row sm:items-center justify-between gap-4"
                                >
                                    <div class="flex items-center gap-4">
                                        <div
                                            class="p-2 bg-primary/10 rounded-lg shrink-0"
                                        >
                                            <Stethoscope
                                                class="h-6 w-6 text-primary"
                                            />
                                        </div>
                                        <div class="space-y-1">
                                            <h3
                                                class="font-semibold text-sm flex items-center gap-2"
                                            >
                                                小皮医生 (Dr.Piney)
                                                <Badge
                                                    variant="outline"
                                                    class="text-[10px] h-4 px-1 border-primary/30 text-primary"
                                                    >Beta</Badge
                                                >
                                            </h3>
                                            <p
                                                class="text-xs text-muted-foreground leading-relaxed"
                                            >
                                                AI 驱动的角色卡质量诊断与优化（实验功能，如果角色卡很庞大，会相对比较消耗token，诊断结果仅供参考）。
                                            </p>
                                        </div>
                                    </div>
                                    <Button
                                        size="sm"
                                        class="h-8 text-xs gap-2 shrink-0 w-full sm:w-auto"
                                        onclick={() => (showDoctorDialog = true)}
                                    >
                                        {#if isDoctorRunning}
                                            <Loader2 class="h-3.5 w-3.5 animate-spin" />
                                            诊断中...
                                        {:else}
                                            <Sparkles class="h-3.5 w-3.5" />
                                            开始诊断
                                        {/if}
                                    </Button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <div class={activeTab === "persona" ? "" : "hidden"}>
                <div
                    class="space-y-6 max-w-4xl mx-auto animate-in fade-in slide-in-from-bottom-4 duration-500 pb-10"
                >
                    <div class="flex items-center justify-between">
                        <h2 class="text-lg font-semibold">角色设定详细信息</h2>
                        <Button
                            onclick={savePersona}
                            disabled={isSavingPersona}
                            class="gap-2"
                        >
                            {#if isSavingPersona}
                                <Loader2 class="h-4 w-4 animate-spin" /> 保存中...
                            {:else}
                                <Save class="h-4 w-4" /> 保存设定
                            {/if}
                        </Button>
                    </div>



                    <div class="space-y-8">
                        <!-- Identity Section -->
                        <div class="space-y-4">
                            <div class="flex items-center gap-2 mb-2">
                                <div
                                    class="p-1.5 rounded-md bg-primary/10 text-primary"
                                >
                                    <IdCard class="h-4 w-4" />
                                </div>
                                <h3
                                    class="text-lg font-semibold tracking-tight"
                                >
                                    身份设定
                                </h3>
                            </div>

                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                <div
                                    class="space-y-2 p-3 md:p-4 rounded-xl border border-border/40 bg-card/50 shadow-sm hover:border-primary/20 transition-all duration-300 group"
                                >
                                    <Label
                                        class="text-xs font-medium text-muted-foreground uppercase tracking-wider"
                                        >角色名称</Label
                                    >
                                    <Input
                                        bind:value={formName}
                                        placeholder="给角色起个名字..."
                                        class={cn(
                                            "border-0 bg-secondary/20 h-10 text-lg font-medium focus-visible:ring-1 focus-visible:bg-background transition-all shadow-none",
                                            isNameDirty &&
                                                "bg-amber-500/10 focus-visible:bg-amber-500/10",
                                        )}
                                    />
                                </div>

                                <!-- 创作者字段：source=local 时可编辑且始终显示；source=import 时只读且仅原始值有值时显示 -->
                                {#if card.source === "local" || originalFormState.creator}
                                    <div
                                        class="space-y-2 p-3 md:p-4 rounded-xl border border-border/40 bg-card/50 shadow-sm hover:border-primary/20 transition-all duration-300 group"
                                    >
                                        <Label
                                            class="text-xs font-medium text-muted-foreground uppercase tracking-wider"
                                            >创作者</Label
                                        >
                                        {#if card.source === "local"}
                                            <Input
                                                bind:value={formCreator}
                                                placeholder="输入创作者名称..."
                                                class="border-0 bg-secondary/20 h-10 font-medium focus-visible:ring-1 focus-visible:bg-background transition-all shadow-none"
                                            />
                                        {:else}
                                            <div
                                                class="flex items-center h-10 px-3 rounded-md bg-muted/50 text-muted-foreground border border-transparent"
                                            >
                                                {formCreator}
                                            </div>
                                        {/if}
                                    </div>
                                {/if}
                            </div>

                            <!-- 角色描述：source=local 时始终显示；source=import 时仅原始值有值时显示 -->
                            {#if card.source === "local" || originalFormState.description}
                                <div
                                    class="p-3 md:p-4 rounded-xl border border-border/40 bg-card/50 shadow-sm hover:border-primary/20 transition-all duration-300"
                                >
                                    <RichTextarea
                                        bind:value={formDescription}
                                        bind:isZenMode={isDescZenMode}
                                        label="角色描述"
                                        placeholder="详细描述角色的外貌、性格..."
                                        class="border-0 bg-transparent shadow-none p-0 focus-visible:ring-0"
                                        isDirty={isDescDirty}
                                        icon={User}
                                        aiFeature={AiFeature.OPTIMIZE_DESCRIPTION}
                                        extraActions={genButton}
                                    />
                                </div>
                            {/if}
                        </div>

                        <!-- Dialogue Section -->
                        <div class="space-y-4">
                            <div class="flex items-center gap-2 mb-2">
                                <div
                                    class="p-1.5 rounded-md bg-blue-500/10 text-blue-500"
                                >
                                    <MessageSquareQuote class="h-4 w-4" />
                                </div>
                                <h3
                                    class="text-lg font-semibold tracking-tight"
                                >
                                    对话行为
                                </h3>
                            </div>

                            <div
                                class="p-1 rounded-2xl border border-border/50 bg-muted/30"
                            >
                                <div
                                    class="bg-background rounded-xl shadow-sm border border-border/20 overflow-hidden"
                                >
                                    <GreetingsSwitcher
                                        bind:firstMes={formFirstMes}
                                        bind:alternateGreetings={
                                            formAltGreetings
                                        }
                                        regexScripts={
                                            card?.data?.data?.extensions?.regex_scripts || 
                                            card?.data?.extensions?.regex_scripts || 
                                            []
                                        }
                                        isDirty={isGreetingsDirty}
                                        class="border-0 shadow-none bg-transparent"
                                    />
                                </div>
                            </div>

                            <!-- 对话示例：有原始值时始终显示；否则仅在勾选开关时显示 (仅 source=local 有此逻辑) -->
                            {#if (card.source === "import" && originalFormState.mesExample) || (card.source === "local" && (originalFormState.mesExample || showExtraSettings))}
                                <div
                                    class="p-3 md:p-4 rounded-xl border border-border/40 bg-card/50 shadow-sm hover:border-primary/20 transition-all duration-300"
                                >
                                    <RichTextarea
                                        bind:value={formMesExample}
                                        label="对话示例"
                                        placeholder="对话示例..."
                                        rows={5}
                                        class="border-0 bg-transparent shadow-none p-0 focus-visible:ring-0 font-mono text-sm leading-relaxed"
                                        isDirty={isMesExampleDirty}
                                        icon={ScrollText}
                                        aiFeature={AiFeature.OPTIMIZE_FIRST_MES}
                                    />
                                </div>
                            {/if}
                        </div>

                        <!-- World & Logic Section：有原始值时始终显示；否则仅在勾选开关时显示 (仅 source=local 有此逻辑) -->
                        {#if (card.source === "import" && (originalFormState.scenario || originalFormState.personality)) || (card.source === "local" && (originalFormState.scenario || originalFormState.personality || showExtraSettings))}
                            <div class="space-y-4">
                                <div class="flex items-center gap-2 mb-2">
                                    <div
                                        class="p-1.5 rounded-md bg-purple-500/10 text-purple-500"
                                    >
                                        <Globe class="h-4 w-4" />
                                    </div>
                                    <h3
                                        class="text-lg font-semibold tracking-tight"
                                    >
                                        世界观与逻辑
                                    </h3>
                                </div>

                                <div class="grid gap-4">
                                    <!-- Personality：有原始值时始终显示；否则需勾选开关 (source=local) / 不显示 (source=import) -->
                                    {#if (card.source === "import" && originalFormState.personality) || (card.source === "local" && (originalFormState.personality || showExtraSettings))}
                                        <div
                                            class="p-3 md:p-4 rounded-xl border border-border/40 bg-card/50 shadow-sm hover:border-primary/20 transition-all duration-300"
                                        >
                                            <RichTextarea
                                                bind:value={formPersonality}
                                                label="角色设定摘要 (Personality)"
                                                placeholder="输入角色性格摘要..."
                                                class="border-0 bg-transparent shadow-none p-0 focus-visible:ring-0"
                                                isDirty={isPersonalityDirty}
                                                icon={Sparkles}
                                                aiFeature={AiFeature.OPTIMIZE_DESCRIPTION}
                                            />
                                        </div>
                                    {/if}

                                    <!-- Scenario：有原始值时始终显示；否则需勾选开关 (source=local) / 不显示 (source=import) -->
                                    {#if (card.source === "import" && originalFormState.scenario) || (card.source === "local" && (originalFormState.scenario || showExtraSettings))}
                                        <div
                                            class="p-3 md:p-4 rounded-xl border border-border/40 bg-card/50 shadow-sm hover:border-primary/20 transition-all duration-300"
                                        >
                                            <RichTextarea
                                                bind:value={formScenario}
                                                label="情景 (Scenario)"
                                                placeholder="输入情景..."
                                                class="border-0 bg-transparent shadow-none p-0 focus-visible:ring-0"
                                                isDirty={isScenarioDirty}
                                                icon={Map}
                                                aiFeature={AiFeature.OPTIMIZE_SCENARIO}
                                            />
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        {/if}

                        <!-- 额外设定项开关（仅 source=local 且不是所有额外设定项都有值时显示，始终在最底部） -->
                        {#if card.source === "local" && !(originalFormState.mesExample && originalFormState.personality && originalFormState.scenario)}
                            <label class="flex items-center gap-2 mt-4 cursor-pointer select-none">
                                <input 
                                    type="checkbox" 
                                    bind:checked={showExtraSettings}
                                    onchange={() => {
                                        // 保存到 localStorage
                                        const key = `piney_extra_settings_${cardId}`;
                                        localStorage.setItem(key, showExtraSettings.toString());
                                    }}
                                    class="h-3.5 w-3.5 rounded border-muted-foreground/30 text-muted-foreground focus:ring-0 focus:ring-offset-0"
                                />
                                <span class="text-xs text-muted-foreground/60">
                                    点击启用更多设定项（非必要）
                                </span>
                            </label>
                        {/if}
                    </div>
                </div>
            </div>

            
            <!-- World Info Tab -->
            <div class={activeTab === "world_info" ? "" : "hidden"}>
                <div class="space-y-6 max-w-4xl mx-auto pb-10">
                    <div class="flex items-center justify-between mb-4">
                        <div class="space-y-1">
                            <h2 class="text-lg font-semibold">
                                角色专属世界书
                            </h2>
                            <p class="text-xs text-muted-foreground">
                                配置与角色绑定的世界书设定
                            </p>
                        </div>
                        <Button
                            onclick={saveWorldInfo}
                            disabled={isSavingWorldInfo}
                            class="gap-2"
                        >
                            {#if isSavingWorldInfo}
                                <Loader2 class="h-4 w-4 animate-spin" /> 保存中...
                            {:else}
                                <Save class="h-4 w-4" /> 保存世界书
                            {/if}
                        </Button>
                    </div>

                    {#if card && card.data && card.data.data}
                        <WorldInfoTab
                            bind:data={card.data.data}
                            {lastSaved}
                            source={card.source}
                            onChange={() => (card = card)}
                        />
                    {:else}
                        <div class="text-center py-20 text-muted-foreground">
                            数据加载未完成或格式错误
                        </div>
                    {/if}
                </div>
                </div>


            <!-- Regex Tab -->
            <div class={activeTab === "regex" ? "" : "hidden"}>
                 <div class="space-y-6 max-w-4xl mx-auto pb-10">
                    <div class="flex items-center justify-between mb-4">
                        <div class="space-y-1">
                            <h2 class="text-lg font-semibold">
                                正则
                            </h2>
                            <p class="text-xs text-muted-foreground">
                                配置针对此角色的正则表达式替换规则
                            </p>
                        </div>
                        <Button
                            onclick={saveRegex}
                            disabled={isSavingWorldInfo}
                            class="gap-2"
                        >
                            {#if isSavingWorldInfo}
                                <Loader2 class="h-4 w-4 animate-spin" /> 保存中...
                            {:else}
                                <Save class="h-4 w-4" /> 保存正则
                            {/if}
                        </Button>
                    </div>

                    {#if card && card.data && card.data.data}
                        <RegexTab
                            bind:data={card.data.data}
                            {lastSaved}
                            onChange={() => (card = card)}
                        />
                    {:else}
                         <div class="text-center py-20 text-muted-foreground">
                            数据加载未完成或格式错误
                        </div>
                    {/if}
                </div>
            </div>
            <!-- Version History Tab -->
            <div class={activeTab === "versions" ? "" : "hidden"}>
                 <div class="space-y-6 max-w-4xl mx-auto pb-10">
                     <VersionHistoryTab
                         {cardId}
                         currentVersion={card.version}
                         source={card.source}
                         onRestore={loadCard}
                         onClaim={loadCard}
                     />
                </div>
            </div>

            <!-- Chat History Tab -->
            <div class={activeTab === "chat" ? "" : "hidden"}>
                 <div class="space-y-6 max-w-4xl mx-auto pb-10">
                    <div class="flex items-center justify-between mb-4">
                        <div class="space-y-1">
                            <h2 class="text-lg font-semibold">
                                聊天记录
                            </h2>
                            <p class="text-xs text-muted-foreground">
                                管理与此角色相关的聊天记录文件
                            </p>
                        </div>
                    </div>

                    <ChatHistoryTab {cardId} />
                </div>
            </div>
        {/if}
    </div>
</div>

<svelte:window onbeforeunload={handleBeforeUnload} />


<Dialog.Root bind:open={showUnsavedDialog}>
    <Dialog.Content class="sm:max-w-[425px]">
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2 text-destructive">
                <AlertTriangle class="h-5 w-5" />
                未保存的更改
            </Dialog.Title>
            <Dialog.Description class="pt-2">
                当前页面有未保存的编辑内容。如果离开，您的更改将会丢失。
            </Dialog.Description>
        </Dialog.Header>
        <Dialog.Footer class="mt-4 gap-2 sm:gap-0">
            <Button variant="outline" onclick={cancelDiscard}
                >取消（留在页面）</Button
            >
            <Button variant="destructive" onclick={confirmDiscard}
                >丢弃更改并离开</Button
            >
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>

<!-- AI Generation Dialog -->
<Dialog.Root open={isGenDialogOpen} onOpenChange={handleGenDialogChange}>
    <Dialog.Content class="sm:max-w-[500px]">
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2">
                <Bot class="h-5 w-5 text-primary" />
                AI 角色生成
            </Dialog.Title>
            <Dialog.Description>
                通过简短描述快速构建角色档案。
            </Dialog.Description>
        </Dialog.Header>
        
        <div class="space-y-4 py-4">
            <div class="space-y-2">
                <Label>描述你的角色想法</Label>
                <Textarea 
                    bind:value={genInput} 
                    placeholder="描述的越详细，AI生成的越完整..."
                    rows={4}
                />
            </div>
            
            <div class="flex items-start gap-2">
                <Checkbox id="use-yaml" bind:checked={genUseYaml} class="mt-0.5 border-muted-foreground/50" />
                <div class="grid gap-1.5 leading-none">
                    <Label
                        for="use-yaml"
                        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    >
                        使用 YAML 格式创建
                    </Label>
                    <p class="text-sm text-muted-foreground">
                        勾选后将严格按照 YAML 结构生成详细档案。
                    </p>
                </div>
            </div>

            <div class="flex items-start gap-2">
                <Checkbox id="include-wi" bind:checked={genIncludeWorldInfo} class="mt-0.5 border-muted-foreground/50" />
                <div class="grid gap-1.5 leading-none">
                    <Label
                        for="include-wi"
                        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                    >
                        附加世界书
                    </Label>
                    <p class="text-sm text-muted-foreground">
                        勾选此项将会在提示词中附加世界书内容，可能消耗大量token。
                    </p>
                </div>
            </div>
        </div>

        <Dialog.Footer>
            <Button variant="outline" onclick={() => isGenDialogOpen = false}>取消</Button>
            <Button onclick={handleGenerateCharacter} disabled={isGenerating}>
                {#if isGenerating}
                    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                    生成中，别关...
                {:else}
                    <Sparkles class="mr-2 h-4 w-4" />
                    开始生成
                {/if}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>

{#snippet genButton()}
    {#if card.source === "local"}
        {#if isGenerating}
             <Button variant="ghost" size="sm" class="h-5 text-[10px] px-2 text-primary" disabled>
                 <Loader2 class="mr-1 h-3 w-3 animate-spin" /> AI生成中...
             </Button>
        {:else}
             <Button variant="ghost" size="sm" class="h-5 text-[10px] px-2 text-muted-foreground hover:text-primary" onclick={openGenDialog}>
                 <Bot class="mr-1 h-3 w-3" /> AI生成
             </Button>
        {/if}
    {/if}
{/snippet}

<AlertDialog.Root bind:open={showDeleteDialog}>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>你是认真的吗？</AlertDialog.Title>
            <AlertDialog.Description>
                此操作将把该角色卡移至回收站。你可以在角色库的回收站中恢复它。
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
            <AlertDialog.Cancel>取消</AlertDialog.Cancel>
            <AlertDialog.Action
                class="bg-destructive !text-destructive-foreground hover:bg-destructive/90"
                onclick={deleteCard}>删除</AlertDialog.Action
            >
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

{#snippet ReviewStat({
    label,
    value,
    compact = false,
}: {
    label: string;
    value: string | number;
    compact?: boolean;
})}
    <div class={cn("rounded-lg border bg-card", compact ? "p-3" : "p-4")}>
        <div class="text-[10px] text-muted-foreground mb-0.5">{label}</div>
        <div class={cn("font-mono font-bold", compact ? "text-sm" : "text-xl")}>
            {value}
        
    <!-- Cropper Dialog -->
    <ImageCropperDialog 
        bind:open={showCropper} 
        imageSrc={cropperImageSrc} 
        on:confirm={handleCropConfirm} 
    />

    <!-- Doctor Dialog -->
    <DoctorDialog
        bind:open={showDoctorDialog}
        cardId={cardId}
        onClose={() => (showDoctorDialog = false)}
    />
</div>
    </div>
{/snippet}
