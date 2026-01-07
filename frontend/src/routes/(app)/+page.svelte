<script lang="ts">
    import { onMount } from "svelte";
    import { breadcrumbs } from "$lib/stores/breadcrumb";

    onMount(() => {
        breadcrumbs.set([{ label: "控制台" }]);
    });
</script>

<h1>主页</h1>
