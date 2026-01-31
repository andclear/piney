<script lang="ts">
    import { page } from "$app/stores";
    import { onMount } from "svelte";
    import { breadcrumbs } from "$lib/stores/breadcrumb";
    import { Button } from "$lib/components/ui/button";
    import { Save, Loader2, ArrowLeft } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import { goto } from "$app/navigation";
    import WorldInfoTab from "$lib/components/character/world_info/WorldInfoTab.svelte";
    import { useUnsavedChanges } from "$lib/hooks/use-unsaved-changes.svelte";
    import UnsavedGuard from "$lib/components/common/UnsavedGuard.svelte";

    import { API_BASE } from "$lib/api";

    let id = $page.params.id;
    let loading = $state(true);
    let saving = $state(false);
    let item: any = $state(null);

    // We wrap the world info data in { character_book: ... } to match WorldInfoTab expectation
    let viewData: any = $state({ character_book: { entries: [] } });
    let originalState = "";

    // Dirty state
    let isDirty = $state(false);
    let lastSaved = $state(0);
    let worldName = $state("");

    // Initialize Unsaved Changes Guard
    const unsaved = useUnsavedChanges(() => isDirty);

    onMount(async () => {
        await loadItem();
    });

    async function loadItem() {
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch(`${API_BASE}/api/world_info/${id}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {},
            });
            if (!res.ok) throw new Error("加载失败");
            item = await res.json();
            worldName = item.name;

            breadcrumbs.set([
                { label: "世界书", href: "/worldinfo" },
                { label: item.name },
            ]);

            // Parse data
            try {
                const parsed = JSON.parse(item.data);
                viewData = { character_book: parsed };

                // If entries is undefined, init it
                if (!viewData.character_book.entries) {
                    viewData.character_book.entries = [];
                }

                // Ensure extensions exists
                if (!viewData.character_book.extensions) {
                    viewData.character_book.extensions = {};
                }
            } catch (e) {
                console.error("JSON parse error", e);
                viewData = { character_book: { entries: [] } };
            }

            originalState = JSON.stringify(viewData.character_book);
        } catch (e) {
            toast.error("加载失败", { description: String(e) });
            goto("/worldinfo");
        } finally {
            loading = false;
        }
    }

    async function save() {
        saving = true;
        try {
            const token = localStorage.getItem("auth_token");

            // Unwrap data
            const payloadData = viewData.character_book;

            const res = await fetch(`${API_BASE}/api/world_info/${id}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                body: JSON.stringify({
                    name: worldName,
                    data: payloadData,
                }),
            });

            if (!res.ok) throw new Error("保存失败");

            item = await res.json(); // Update local item (timestamp etc)
            worldName = item.name; // ensure sync
            originalState = JSON.stringify(payloadData);
            isDirty = false;
            lastSaved = Date.now();
            toast.success("保存成功");

            // Update breadcrumb name if changed
            breadcrumbs.set([
                { label: "世界书", href: "/worldinfo" },
                { label: worldName },
            ]);
        } catch (e) {
            console.error(e);
            toast.error("保存失败");
        } finally {
            saving = false;
        }
    }

    // Reactivity for dirty check
    function handleChange() {
        // Triggered by WorldInfoTab
        // Force update reference logic if needed, but we check JSON here
        const contentDirty =
            JSON.stringify(viewData.character_book) !== originalState;
        const nameDirty = worldName !== item?.name;
        isDirty = contentDirty || nameDirty;
    }
</script>

<div class="container mx-auto py-6 space-y-6 flex flex-col">
    <UnsavedGuard controller={unsaved} />
    <!-- Header -->
    <div class="flex items-center justify-between shrink-0">
        <div class="flex items-center gap-4">
            <Button variant="ghost" size="icon" href="/worldinfo">
                <ArrowLeft class="h-5 w-5" />
            </Button>
            <h1 class="text-2xl font-bold tracking-tight">
                {worldName || item?.name || "加载中..."}
            </h1>
            {#if isDirty}
                <span class="text-sm text-yellow-500 font-medium"
                    >（未保存）</span
                >
            {/if}
        </div>
        <Button disabled={saving || !isDirty} onclick={save} class="gap-2">
            {#if saving}
                <Loader2 class="h-4 w-4 animate-spin" /> 保存中...
            {:else}
                <Save class="h-4 w-4" /> 保存
            {/if}
        </Button>
    </div>

    <!-- Content -->
    {#if loading}
        <div
            class="flex-1 flex items-center justify-center text-muted-foreground"
        >
            <Loader2 class="h-8 w-8 animate-spin" />
        </div>
    {:else}
        <WorldInfoTab
            bind:data={viewData}
            {lastSaved}
            onChange={handleChange}
            mode="global"
            bind:name={worldName}
        />

    {/if}
</div>
