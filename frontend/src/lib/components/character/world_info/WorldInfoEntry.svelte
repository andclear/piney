<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import * as Select from "$lib/components/ui/select";
    import { Label } from "$lib/components/ui/label";
    import { Textarea } from "$lib/components/ui/textarea";
    import { Switch } from "$lib/components/ui/switch";
    import { Button } from "$lib/components/ui/button";
    import { Badge } from "$lib/components/ui/badge";
    import { Slider } from "$lib/components/ui/slider";
    import {
        ChevronDown,
        ChevronRight,
        Trash2,
        Link,
        Key,
        Zap,
        Settings2,
        Layers,
        Hash,
        Percent,
        Search,
        Eye,
        EyeOff,
        GripVertical
    } from "lucide-svelte";
    import { cn } from "$lib/utils";
    import { slide } from "svelte/transition";
    import { untrack } from "svelte";
    import RichTextarea from "$lib/components/character/RichTextarea.svelte";
    import { AiFeature } from "$lib/ai/types";

    let {
        entry,
        onDelete,
        lastSaved = 0,
        isOpen = $bindable(),
        onChange,
        onUpdate,
        mode = "character", // "character" | "global"
    } = $props();

    // --- Status Logic ---
    // 0 = Always (Zap/Blue), 1 = Key (Key/Green), 2 = Vector (Link)
    let status = $derived.by(() => {
        if (entry.constant) return "0"; // Always
        if (mode === "global") {
            if (entry.vectorized) return "2";
        } else {
            if (entry.extensions?.vectorized || entry.vectorized) return "2";
        }
        return "1"; // Key (Default)
    });

    function getStatus(e: any) {
        if (e.constant) return "0"; // Always
        if (mode === "global") {
            if (e.vectorized) return "2";
        } else {
            if (e.extensions?.vectorized || e.vectorized) return "2";
        }
        return "1"; // Key (Default)
    }

    function updateStatus(val: string) {
        if (mode === "character" && !entry.extensions) {
            onUpdate?.((e: any) => { e.extensions = {}; });
        }

        // Clean up root field logic omitted as it's handled on cleanup or not critical here
        // (Legacy cleanup logic preserved if needed, but let's focus on the update)
        if (mode === "character" && (entry as any).vectorized !== undefined) {
             onUpdate?.((e: any) => {
                delete (e as any).vectorized;
             });
        }

        if (val === "0") {
            // Always
            onUpdate?.((e: any) => {
                e.constant = true;
                if (mode === "global") e.vectorized = false;
                else e.extensions.vectorized = false;
            });
        } else if (val === "1") {
            // Key
             onUpdate?.((e: any) => {
                e.constant = false;
                if (mode === "global") e.vectorized = false;
                else e.extensions.vectorized = false;
             });
        } else if (val === "2") {
            // Vector
            onUpdate?.((e: any) => {
                e.constant = false;
                if (mode === "global") e.vectorized = true;
                else e.extensions.vectorized = true;
            });
        }
        // Explicitly call onChange if onUpdate is missing/didn't trigger it? 
        // onUpdate usually triggers onChange (in parent wrapper).
        // But if onUpdate is undefined? (Shouldn't be)
        if (!onUpdate && onChange) onChange();
    }

    // --- Position Logic ---
    // 0=DefBefore, 1=DefAfter, 2=NoteBefore, 3=NoteAfter, 5=ExBefore, 6=ExAfter, 4=Depth
    let position = $derived.by(() => {
        if (mode === "global") return entry.position?.toString() || "0";
        return entry.extensions?.position?.toString() || "0";
    });

    let role = $derived.by(() => {
        if (mode === "global") return entry.role?.toString() || "0";
        return entry.extensions?.role?.toString() || "0";
    });

    // Helper to get current compound selection for Select UI
    function getPositionValue(e: any) {
        const p =
            mode === "global"
                ? (e.position ?? 0)
                : (e.extensions?.position ?? 0);
        const r = mode === "global" ? (e.role ?? 0) : (e.extensions?.role ?? 0);

        if (p === 4) {
            return `4-${r}`; // 4-0, 4-1, 4-2
        }
        return p.toString();
    }

    let positionValue = $derived.by(() => getPositionValue(entry));

    function updatePosition(val: string) {
        if (mode === "character" && !entry.extensions) {
             onUpdate?.((e: any) => { e.extensions = {}; });
        }

        if (val.startsWith("4-")) {
            const [p, r] = val.split("-").map(Number);
            if (mode === "global") {
                onUpdate?.((e: any) => {
                    e.position = p;
                    e.role = r;
                });
            } else {
                onUpdate?.((e: any) => {
                    e.extensions.position = p;
                    e.extensions.role = r;
                });
            }
        } else {
            const numVal = Number(val);
            if (mode === "global") {
                onUpdate?.((e: any) => {
                    e.position = numVal;
                    e.role = 0;
                });
            } else {
                onUpdate?.((e: any) => {
                    e.extensions.position = numVal;
                    // Explicitly reset role to 0 for standard positions (non-depth)
                    e.extensions.role = 0;
                });
            }
        }
        onChange?.();
    }

    // --- Logic Options ---
    // 0=Any, 1=All, 2=Not All, 3=Not Any
    let logicType = $derived.by(() => {
        if (mode === "global") return entry.selectiveLogic?.toString() || "0";
        return entry.extensions?.selectiveLogic?.toString() || "0";
    });

    function updateLogic(val: string) {
        if (mode === "character" && !entry.extensions) {
             onUpdate?.((e: any) => { e.extensions = {}; });
        }

        if (mode === "global") {
            onUpdate?.((e: any) => { e.selectiveLogic = Number(val); });
        } else {
            onUpdate?.((e: any) => { e.extensions.selectiveLogic = Number(val); });
        }
        onChange?.();
    }

    // --- Keys Helper (Array <-> String) ---
    // JSON uses "keys" (array of strings).
    // We use granular localKeys for the UI chips.

    function addKey(key: string) {
        if (!key) return;
        const trimmed = key.trim();
        if (!trimmed) return;
        // Avoid duplicates if desired, or allow them. ST allows duplicates usually.
        // Let's allow but maybe check? No, straightforward push.
        if (!localKeys.includes(trimmed)) {
            localKeys = [...localKeys, trimmed];

            if (mode === "global") {
                onUpdate?.((e: any) => { e.key = localKeys; });
            } else {
                onUpdate?.((e: any) => {
                    e.keys = localKeys;
                    // Ensure we don't accidentally save to legacy 'key' or 'extensions.key'
                    if ((e as any).key) delete (e as any).key;
                    if (e.extensions && (e.extensions as any).key) {
                        delete (e.extensions as any).key;
                    }
                });
            }
        }
    }

    function removeKey(idx: number) {
        localKeys = localKeys.filter((_: string, i: number) => i !== idx);
        if (mode === "global") {
            entry.key = localKeys;
        } else {
            entry.keys = localKeys;
        }
        onChange?.();
    }

    function handleKeysKeyDown(e: KeyboardEvent) {
        const input = e.currentTarget as HTMLInputElement;
        const val = input.value;

        if (e.key === "Enter") {
            e.preventDefault();
            if (val) {
                addKey(val);
                input.value = "";
            }
        } else if (e.key === "Backspace" && !val && localKeys.length > 0) {
            // Remove last key if backspace pressed on empty input
            removeKey(localKeys.length - 1);
        } else if (e.key === ",") {
            e.preventDefault(); // Prevent literal comma
            if (val) {
                addKey(val);
                input.value = "";
            }
        }
    }

    function handleKeysPaste(e: ClipboardEvent) {
        e.preventDefault();
        const text = e.clipboardData?.getData("text");
        if (!text) return;

        const parts = text
            .split(/[,，\n]/)
            .map((s) => s.trim())
            .filter((s) => s);
        parts.forEach((p) => {
            if (!localKeys.includes(p)) {
                localKeys.push(p);
            }
        });
        localKeys = localKeys; // trigger reactivity
        if (mode === "global") {
            entry.key = localKeys;
        } else {
            entry.keys = localKeys;
        }
        onChange?.();
    }

    // Deprecated updateKeys (replaced by chip logic)
    function updateKeysRaw(val: string) {
        // Fallback if we needed raw string editing, but we are moving to chips.
        // keeping empty or removing if unused.
    }

    // --- Local State for UI Responsiveness & Reactivity Fix ---
    // Since entry comes from legacy parent, it might not be a proxy. We use local states.
    let localStatus = $state("1");
    let localLogic = $state("0");
    let localPosition = $state("0");

    // Helper to extract fields based on mode
    function extractFields(e: any) {
        if (mode === "global") {
            return {
                enabled: !(e.disable ?? false), // Inverted
                comment: e.comment || "",
                content: e.content || "",
                keys: e.key || [],
                secondaryKeys: e.keysecondary || [],
                probability: e.probability ?? 100,
                order: e.order ?? 0,
                depth: e.depth ?? 4,
                scanDepth: e.scanDepth ?? e.scan_depth ?? null,
            };
        } else {
            return {
                enabled: e.enabled ?? true,
                comment: e.comment || "",
                content: e.content || "",
                keys: e.keys || e.key || [],
                secondaryKeys: e.secondary_keys || e.keysecondary || [],
                probability: e.extensions?.probability ?? 100,
                order: e.insertion_order ?? 0,
                depth: e.extensions?.depth ?? 4,
                scanDepth: e.extensions?.scan_depth ?? null,
            };
        }
    }

    // Initial values
    // svelte-ignore state_referenced_locally
    let fields = extractFields(entry);

    let localEnabled = $state(fields.enabled);
    let localComment = $state(fields.comment);
    let localContent = $state(fields.content);
    let localKeys = $state(fields.keys);
    let localSecondaryKeys = $state(fields.secondaryKeys);
    let localProbability = $state(fields.probability);
    let localOrder = $state(fields.order);
    let localDepth = $state(fields.depth);
    let localScanDepth = $state(fields.scanDepth);

    // Sync from derived/props (Upstream changes)
    $effect(() => {
        localStatus = status;
    });
    $effect(() => {
        localLogic = logicType;
    });
    $effect(() => {
        localPosition = positionValue;
    });

    // Sync Entry Prop updates to Locals (e.g. reload) - Identity check
    $effect(() => {
        // If entry identity changes, re-init locals
        if (entry.id !== originalEntry.id || entry.uid !== originalEntry.uid) {
            // Check uid too for global
            const f = extractFields(entry);
            localEnabled = f.enabled;
            localComment = f.comment;
            localContent = f.content;
            localKeys = f.keys;
            localSecondaryKeys = f.secondaryKeys;
            localProbability = f.probability;
            localOrder = f.order;
            localDepth = f.depth;
            localScanDepth = f.scanDepth;
        }
    });

    // Sync Locals -> Entry (Downstream mutations)
    $effect(() => {
        if (mode === "global") {
             if (entry.disable !== !localEnabled) {
                 onUpdate?.((e: any) => { e.disable = !localEnabled; });
             }
        } else {
             if (entry.enabled !== localEnabled) {
                 onUpdate?.((e: any) => { e.enabled = localEnabled; });
             }
        }
    });

    // Update keys effects are separate? No, let's use the pattern.
    // Actually we handle key updates via localKeys mutation above in addKey/removeKey.
    // But for secondary keys we should do the same or use effect.
    // Let's use effect for consistency if possible, OR just use the add/remove pattern for both.
    // The current addKey/removeKey writes directly to entry AND updates localKeys.
    // Let's adopt that for Secondary Keys too.

    $effect(() => {
        if (entry.comment !== localComment) {
            onUpdate?.((e: any) => { e.comment = localComment; });
        }
    });

    $effect(() => {
        if (entry.content !== localContent) {
            onUpdate?.((e: any) => { e.content = localContent; });
        }
    });

    $effect(() => {
        if (mode === "global") {
             // Check for changes
             const d = entry.depth ?? 4;
             const sd = entry.scanDepth ?? entry.scan_depth ?? null;
             const p = entry.probability ?? 100;
             const o = entry.order ?? 0;
             const newSd = localScanDepth === "" || localScanDepth === null ? null : Number(localScanDepth);

             if (d !== localDepth || sd !== newSd || p !== localProbability || o !== localOrder) {
                onUpdate?.((e: any) => {
                    e.probability = localProbability;
                    e.order = localOrder;
                    e.depth = localDepth;
                    e.scanDepth = newSd;
                });
             }
        } else {
             const ext = entry.extensions || {};
             const d = ext.depth ?? 4;
             const sd = ext.scan_depth ?? null;
             const p = ext.probability ?? 100;
             const o = entry.insertion_order ?? 0;
             const newSd = localScanDepth === "" || localScanDepth === null ? null : Number(localScanDepth);

             if (d !== localDepth || sd !== newSd || p !== localProbability || o !== localOrder) {
                 onUpdate?.((e: any) => {
                    if (!e.extensions) e.extensions = {};
                    e.extensions.probability = localProbability;
                    e.insertion_order = localOrder;
                    e.extensions.depth = localDepth;
                    e.extensions.scan_depth = newSd;
                 });
             }
        }
    });

    // --- Labels based on LOCAL state ---
    let statusLabel = $derived.by(() => {
        if (localStatus === "0") return "🔵 始终触发";
        if (localStatus === "2") return "🔗 向量化";
        return "🟢 关键词触发";
    });

    let logicLabel = $derived.by(() => {
        if (localLogic === "0") return "与任意 (Any)";
        if (localLogic === "1") return "与所有 (All)";
        if (localLogic === "2") return "非所有 (Not All)";
        if (localLogic === "3") return "非任何 (Not Any)";
        return "逻辑";
    });

    let positionLabel = $derived.by(() => {
        const val = localPosition;
        if (val === "0") return "角色定义之前";
        if (val === "1") return "角色定义之后";
        if (val === "5") return "示例消息前 (↑EM)";
        if (val === "6") return "示例消息后 (↓EM)";
        if (val === "2") return "作者注释之前";
        if (val === "3") return "作者注释之后";
        if (val === "4-0") return "@D ⚙ [系统] 在深度";
        if (val === "4-1") return "@D 👤 [用户] 在深度";
        if (val === "4-2") return "@D 🤖 [AI] 在深度";
        return "选择位置";
    });

    // Wrappers for updates
    function handleStatusChange(v: string) {
        localStatus = v; // Optimistic update
        updateStatus(v);
    }
    function handleLogicChange(v: string) {
        localLogic = v;
        updateLogic(v);
    }
    function handlePositionChange(v: string) {
        localPosition = v;
        updatePosition(v);
    }

    // --- Dirty Checking Logic ---
    // svelte-ignore state_referenced_locally
    let originalEntry = $state(JSON.parse(JSON.stringify(entry)));

    // Reset snapshot when entry object identity changes or lastSaved updates
    // Unified effect for maintaining originalEntry
    $effect(() => {
        // Dependencies we WANT to trigger reset:
        const _ls = lastSaved;
        const _id = entry.id;

        // Reset snapshot, but DO NOT track entry content changes
        untrack(() => {
            originalEntry = JSON.parse(JSON.stringify(entry));
        });
    });

    // Check against locals for reactivity
    let isEnabledDirty = $derived.by(() => {
        if (mode === "global")
            return localEnabled !== !(originalEntry.disable ?? false);
        return localEnabled !== (originalEntry.enabled ?? true);
    });

    let isCommentDirty = $derived(localComment !== originalEntry.comment);
    // Status: Compare localStatus vs calculated original status
    let isStatusDirty = $derived(localStatus !== getStatus(originalEntry));

    // Keys compare: use localKeys directly
    let isKeysDirty = $derived.by(() => {
        const origKeys =
            mode === "global"
                ? originalEntry.key || []
                : originalEntry.keys || originalEntry.key || [];
        return JSON.stringify(localKeys) !== JSON.stringify(origKeys);
    });

    let isSecondaryKeysDirty = $derived.by(() => {
        const origKeys =
            mode === "global"
                ? originalEntry.keysecondary || []
                : originalEntry.secondary_keys || originalEntry.keysecondary || [];
        return JSON.stringify(localSecondaryKeys) !== JSON.stringify(origKeys);
    });

    // Logic: Compare localLogic vs original
    let isLogicDirty = $derived.by(() => {
        const origLogic =
            mode === "global"
                ? originalEntry.selectiveLogic?.toString() || "0"
                : originalEntry.extensions?.selectiveLogic?.toString() || "0";
        return localLogic !== origLogic;
    });

    let isContentDirty = $derived(localContent !== originalEntry.content);

    // Position: Compare localPosition only
    let isLogicPosDirty = $derived.by(() => {
        const origPos = getPositionValue(originalEntry);
        return localPosition !== origPos;
    });

    let isDepthDirty = $derived.by(() => {
        const origDepth =
            mode === "global"
                ? (originalEntry.depth ?? 4)
                : (originalEntry.extensions?.depth ?? 4);
        return (localDepth ?? 4) !== origDepth;
    });

    let isOrderDirty = $derived.by(() => {
        const origOrder =
            mode === "global"
                ? (originalEntry.order ?? 0)
                : (originalEntry.insertion_order ?? 0);
        return localOrder !== origOrder;
    });

    let isScanDepthDirty = $derived.by(() => {
        const origScan =
            mode === "global"
                ? (originalEntry.scanDepth ?? originalEntry.scan_depth ?? null)
                : (originalEntry.extensions?.scan_depth ?? null);
        return (localScanDepth ?? null) !== origScan;
    });

    let isProbDirty = $derived.by(() => {
        const origProb =
            mode === "global"
                ? (originalEntry.probability ?? 100)
                : (originalEntry.extensions?.probability ?? 100);
        return (localProbability ?? 100) !== origProb;
    });

    // Unified Entry Dirty State
    let isEntryDirty = $derived(
        isEnabledDirty ||
            isCommentDirty ||
            isStatusDirty ||
            isKeysDirty ||
            isSecondaryKeysDirty ||
            isLogicDirty ||
            isContentDirty ||
            isLogicPosDirty ||
            isDepthDirty ||
            isOrderDirty ||
            isScanDepthDirty ||
            isProbDirty,
    );
</script>

<div
    class={cn(
        "rounded-xl border bg-card/50 shadow-sm transition-all duration-300 group",
        isOpen
            ? "border-primary ring-1 ring-primary/100 shadow-md bg-card"
            : "border-border/40 hover:bg-accent/40",
    )}
>
    <!-- Header / Toggle -->
    <div
        class={cn(
            "sticky top-0 z-10 flex items-center gap-3 p-3 cursor-pointer select-none transition-colors",
            isOpen ? "bg-primary/5 rounded-t-xl" : "bg-card group-hover:bg-accent/40 rounded-xl"
        )}
        role="button"
        tabindex="0"
        onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                e.stopPropagation();
                isOpen = !isOpen;
            }
        }}
        onclick={(e) => {
            e.stopPropagation();
            isOpen = !isOpen;
        }}
    >
        <!-- Drag Handle -->
        <div class="cursor-grab active:cursor-grabbing text-muted-foreground hover:text-foreground">
            <GripVertical class="h-4 w-4" />
        </div>

        <!-- Enable Switch -->
        <div 
            role="none" 
            onclick={(e) => e.stopPropagation()} 
            onkeydown={(e) => e.stopPropagation()}
            onmousedown={(e) => e.stopPropagation()}
            ontouchstart={(e) => e.stopPropagation()}
        >
            <Switch
                bind:checked={localEnabled}
                class="data-[state=checked]:bg-primary data-[state=unchecked]:bg-input scale-75 origin-left"
            />
        </div>

        <div 
            class="flex-1 flex items-center gap-2 overflow-hidden"
            onmousedown={(e) => e.stopPropagation()}
            ontouchstart={(e) => e.stopPropagation()}
            role="none"
        >
             <!-- Chevron (Rotating) -->
             <div class={cn("transition-transform duration-200", isOpen && "rotate-180")}>
                <ChevronDown class="h-4 w-4 text-muted-foreground" />
            </div>

            <!-- Title -->
            <span
                class={cn(
                    "font-medium truncate max-w-[200px] md:max-w-xs transition-colors",
                    !localEnabled &&
                        "text-muted-foreground line-through decoration-transparent", 
                    isEntryDirty && "text-amber-500",
                )}
            >
                {localComment || "未使用标题"}
            </span>

            <!-- Indicators -->
            <div
                class={cn(
                    "flex items-center gap-1.5 text-xs font-medium opacity-80",
                    !localEnabled && "grayscale opacity-50",
                )}
            >
                {#if localStatus === "0"}
                    <Badge
                        variant="secondary"
                        class="h-5 px-1.5 border-0 bg-secondary text-secondary-foreground"
                    >
                        🔵 始终
                    </Badge>
                {:else if localStatus === "2"}
                    <Badge
                        variant="secondary"
                        class="h-5 px-1.5 border-0 bg-secondary text-secondary-foreground"
                    >
                        🔗 向量
                    </Badge>
                {:else}
                    <Badge
                        variant="secondary"
                        class="h-5 px-1.5 border-0 bg-secondary text-secondary-foreground"
                    >
                        🟢 关键词
                    </Badge>
                {/if}
            </div>
            {#if localKeys && localKeys.length > 0}
                <span
                    class={cn(
                        "text-xs truncate hidden sm:block max-w-[150px]",
                        isKeysDirty
                            ? "text-amber-500" // Already styled, but kept for clarity
                            : "text-muted-foreground",
                    )}
                >
                    [{localKeys.join(", ")}]
                </span>
            {/if}

            {#if isEntryDirty}
                <span
                    class="w-2 h-2 rounded-full bg-amber-500 shadow-sm animate-pulse ml-2"
                    title="条目有未保存的更改"
                ></span>
            {/if}
        </div>

        <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8 text-muted-foreground hover:text-destructive hover:bg-destructive/10 -mr-1"
            onclick={(e) => {
                e.stopPropagation();
                onDelete(entry.id);
            }}
        >
            <Trash2 class="h-4 w-4" />
        </Button>
    </div>

    <!-- Details -->
    <!-- Details -->
    {#if isOpen}
        <div
            class="px-4 pb-4 pt-0 space-y-4"
        >
            <div class="h-px w-full bg-border/40"></div>

            <!-- Single Grid Layout for All Settings -->
            <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                <!-- 1. Title (Full Width) -->
                <div class="sm:col-span-2 lg:col-span-3 space-y-1.5">
                    <Label
                        class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                        >标题 (注释)</Label
                    >
                    <Input
                        bind:value={localComment}
                        placeholder="条目标题..."
                        class={cn(
                            "bg-background/50 border-1 ring-1 ring-border/50 focus-visible:ring-primary/50",
                            isCommentDirty &&
                                "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5",
                        )}
                    />
                </div>

                <!-- 2. Trigger Mode -->
                <div class="space-y-1.5">
                    <Label
                        class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                        >触发模式</Label
                    >
                    <Select.Root
                        type="single"
                        value={localStatus}
                        onValueChange={handleStatusChange}
                    >
                        <Select.Trigger
                            class={cn(
                                "w-full h-10 px-3 py-2 rounded-md bg-background/50 border-1 ring-1 ring-border/50 focus:ring-primary/50 text-sm flex items-center justify-between",
                                isStatusDirty &&
                                    "border-amber-500/50 ring-amber-500/50 bg-amber-500/5 text-amber-700",
                            )}
                        >
                            <span class="truncate">{statusLabel}</span>
                        </Select.Trigger>
                        <Select.Content>
                            <Select.Item value="0" label="🔵 始终触发"
                                >🔵 始终触发</Select.Item
                            >
                            <Select.Item value="1" label="🟢 关键词触发"
                                >🟢 关键词触发</Select.Item
                            >
                            <Select.Item value="2" label="🔗 向量化"
                                >🔗 向量化</Select.Item
                            >
                        </Select.Content>
                    </Select.Root>
                </div>

                <!-- 3. Keywords (Conditional) -->
                {#if localStatus === "1"}
                    <div class="space-y-1.5">
                        <Label
                            class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                            >主要关键字</Label
                        >
                        <div
                            class={cn(
                                "flex flex-wrap gap-1.5 p-2 rounded-md min-h-10 bg-background/50 border-1 ring-1 ring-border/50",
                                isKeysDirty &&
                                    "border-amber-500/50 focus-within:ring-amber-500/50 bg-amber-500/5",
                                !isKeysDirty && "focus-within:ring-primary/50",
                            )}
                        >
                            {#each localKeys as k, i}
                                <Badge
                                    variant="secondary"
                                    class="h-6 px-1.5 gap-1"
                                >
                                    {k}
                                    <button
                                        class="hover:text-destructive"
                                        onclick={() => removeKey(i)}
                                    >
                                        <Trash2 class="h-3 w-3" />
                                    </button>
                                </Badge>
                            {/each}
                            <input
                                class="flex-1 min-w-[60px] bg-transparent border-0 outline-none text-sm placeholder:text-muted-foreground/50"
                                placeholder={localKeys.length === 0
                                    ? "输入关键字, 按回车确认..."
                                    : ""}
                                onkeydown={handleKeysKeyDown}
                                onpaste={handleKeysPaste}
                            />
                        </div>
                    </div>

                {/if}

                <!-- 4. Logic (Conditional) -->
                {#if localStatus === "1"}
                    <div class="space-y-1.5">
                        <Label
                            class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                            >逻辑关系</Label
                        >
                        <Select.Root
                            type="single"
                            value={localLogic}
                            onValueChange={handleLogicChange}
                        >
                            <Select.Trigger
                                class={cn(
                                    "w-full h-10 px-3 py-2 rounded-md bg-background/50 border-1 ring-1 ring-border/50 focus:ring-primary/50 text-sm flex items-center justify-between",
                                    isLogicDirty &&
                                        "border-amber-500/50 ring-amber-500/50 bg-amber-500/5 text-amber-700",
                                )}
                            >
                                <span class="truncate">{logicLabel}</span>
                            </Select.Trigger>
                            <Select.Content>
                                <Select.Item value="0">与任意 (Any)</Select.Item
                                >
                                <Select.Item value="1">与所有 (All)</Select.Item
                                >
                                <Select.Item value="2"
                                    >非所有 (Not All)</Select.Item
                                >
                                <Select.Item value="3"
                                    >非任何 (Not Any)</Select.Item
                                >
                            </Select.Content>
                        </Select.Root>
                    </div>
                {/if}

                <!-- 5. Content (Full Width) -->
                <div class="sm:col-span-2 lg:col-span-3">
                    <RichTextarea
                        bind:value={localContent}
                        label="内容"
                        placeholder="世界书条目内容..."
                        rows={8}
                        isDirty={isContentDirty}
                        class="font-mono text-sm leading-relaxed"
                        aiFeature={AiFeature.OPTIMIZE_WORLDBOOK}
                    />
                </div>

                <!-- 6. Insertion Position -->
                <div class="space-y-1.5">
                    <Label
                        class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                        >插入位置</Label
                    >
                    <Select.Root
                        type="single"
                        value={localPosition}
                        onValueChange={handlePositionChange}
                    >
                        <Select.Trigger
                            class={cn(
                                "w-full h-10 px-3 py-2 rounded-md bg-background/50 border-1 ring-1 ring-border/50 focus:ring-primary/50 text-sm flex items-center justify-between",
                                isLogicPosDirty &&
                                    "border-amber-500/50 ring-amber-500/50 bg-amber-500/5 text-amber-700",
                            )}
                        >
                            <span class="truncate">{positionLabel}</span>
                        </Select.Trigger>
                        <Select.Content class="max-h-[300px]">
                            <Select.Item value="0">角色定义之前</Select.Item>
                            <Select.Item value="1">角色定义之后</Select.Item>
                            <Select.Item value="5">示例消息前 (↑EM)</Select.Item
                            >
                            <Select.Item value="6">示例消息后 (↓EM)</Select.Item
                            >
                            <Select.Item value="2">作者注释之前</Select.Item>
                            <Select.Item value="3">作者注释之后</Select.Item>
                            <Select.Separator />
                            <Select.Item value="4-0"
                                >@D ⚙ [系统] 在深度</Select.Item
                            >
                            <Select.Item value="4-1"
                                >@D 👤 [用户] 在深度</Select.Item
                            >
                            <Select.Item value="4-2"
                                >@D 🤖 [AI] 在深度</Select.Item
                            >
                        </Select.Content>
                    </Select.Root>
                </div>

                <!-- 7. Insertion Order -->
                <div class="space-y-1.5">
                    <Label
                        class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                        >插入顺序</Label
                    >
                    <Input
                        type="number"
                        bind:value={localOrder}
                        class={cn(
                            "bg-background/50 border-1 ring-1 ring-border/50",
                            isOrderDirty &&
                                "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5",
                        )}
                    />
                </div>

                <!-- 8. Probability -->
                <div class="space-y-1.5">
                    <Label
                        class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                        >激活概率 (%)</Label
                    >
                    <Input
                        type="number"
                        bind:value={localProbability}
                        max="100"
                        min="0"
                        step="1"
                        class={cn(
                            "bg-background/50 border-1 ring-1 ring-border/50",
                            isProbDirty &&
                                "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5",
                        )}
                    />
                </div>

                <!-- 9. Depth (Conditional) -->
                {#if localPosition.startsWith("4-")}
                    <div class="space-y-1.5">
                        <Label
                            class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                            >插入深度 (层级)</Label
                        >
                        <Input
                            type="number"
                            bind:value={localDepth}
                            class={cn(
                                "bg-background/50 border-1 ring-1 ring-border/50",
                                isDepthDirty &&
                                    "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5",
                            )}
                        />
                    </div>
                {/if}

                <!-- 10. Scan Depth -->
                <div class="space-y-1.5">
                    <Label
                        class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                        >扫描深度</Label
                    >
                    <Input
                        type="number"
                        bind:value={localScanDepth}
                        placeholder="默认 (Null)"
                        class={cn(
                            "bg-background/50 border-1 ring-1 ring-border/50",
                            isScanDepthDirty &&
                                "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5",
                        )}
                    />
                </div>
            </div>
        </div>
    {/if}
</div>
