<script lang="ts">
    import AppSidebar from "$lib/components/app-sidebar.svelte";
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import * as Breadcrumb from "$lib/components/ui/breadcrumb/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { SidebarTrigger } from "$lib/components/ui/sidebar/index.js";
    import { breadcrumbs } from "$lib/stores/breadcrumb";

    let { children } = $props();
</script>

<Sidebar.Provider>
    <AppSidebar />
    <Sidebar.Inset>
        <header
            class="flex h-12 shrink-0 items-center gap-2 border-b border-border transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-12"
        >
            <div class="flex items-center gap-2 px-4">
                <SidebarTrigger class="-ml-1" />
                <Separator orientation="vertical" class="mr-2 h-4" />
                <Breadcrumb.Root>
                    <Breadcrumb.List>
                        {#each $breadcrumbs as item, i}
                            {#if i > 0}
                                <Breadcrumb.Separator class="hidden md:block" />
                            {/if}
                            <Breadcrumb.Item class={i < $breadcrumbs.length - 1 ? "hidden md:block" : ""}>
                                {#if item.href && i < $breadcrumbs.length - 1}
                                    <Breadcrumb.Link href={item.href}
                                        >{item.label}</Breadcrumb.Link
                                    >
                                {:else}
                                    <Breadcrumb.Page
                                        >{item.label}</Breadcrumb.Page
                                    >
                                {/if}
                            </Breadcrumb.Item>
                        {/each}
                    </Breadcrumb.List>
                </Breadcrumb.Root>
            </div>
        </header>
        <div class="flex flex-1 flex-col gap-4 p-4 pt-0">
            {@render children()}
        </div>
    </Sidebar.Inset>
</Sidebar.Provider>
