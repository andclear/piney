<script lang="ts">
    import { createIframeContent } from "$lib/utils/renderUtils";
    import { onMount, onDestroy } from "svelte";

    // Receives the raw frontend code (processed by pipeline but identified as frontend)
    let { content = "" } = $props();

    let iframeSrcDoc = $state("");
    let iframeHeight = $state(200); // Default height
    let iframeRef: HTMLIFrameElement | undefined = $state();
    let iframeId = "th-iframe-" + Math.random().toString(36).slice(2, 9);

    $effect(() => {
        if (content) {
            iframeSrcDoc = createIframeContent(content);
        }
    });

    const handleMessage = (e: MessageEvent) => {
        // Check for height adjustment message
        if (e.data?.type === 'TH_ADJUST_IFRAME_HEIGHT' && e.data?.name === iframeId) {
            iframeHeight = e.data.height;
        }
    };

    onMount(() => {
        window.addEventListener('message', handleMessage);
    });

    onDestroy(() => {
        if (typeof window !== 'undefined') {
            window.removeEventListener('message', handleMessage);
        }
    });
</script>

<div class="th-iframe-container rounded-md overflow-hidden border border-border/50 bg-card my-4">
    <iframe
        bind:this={iframeRef}
        id={iframeId}
        name={iframeId}
        srcdoc={iframeSrcDoc}
        width="100%"
        height={iframeHeight}
        frameborder="0"
        scrolling="no"
        sandbox="allow-scripts"
        title="Frontend Preview"
        class="w-full block"
    ></iframe>
</div>

<style>
    iframe {
        border: none;
        display: block;
        margin: 0;
        padding: 0;
    }
</style>
