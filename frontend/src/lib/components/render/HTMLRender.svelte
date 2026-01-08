<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createSrcContent, isFrontend } from '$lib/utils/renderUtils';
    
    // Simple ID generator since strict UUID isn't required for iframe IDs
    function generateId() {
        return Math.random().toString(36).substring(2, 9);
    }

    let { content = '', useBlobUrl = false } = $props<{
        content: string,
        useBlobUrl?: boolean
    }>();

    let iframeRef: HTMLIFrameElement;
    let iframeId = `th-render-${generateId()}`;
    let srcifiedContent = $state('');

    // Regenerate content when input changes
    $effect(() => {
        if (content && isFrontend(content)) {
             srcifiedContent = createSrcContent(content, useBlobUrl);
        } else {
             // Fallback for non-frontend content if needed, or just wrap it
             srcifiedContent = createSrcContent(content, useBlobUrl);
        }
    });

    function handleMessage(event: MessageEvent) {
        if (event.data?.type === 'TH_ADJUST_IFRAME_HEIGHT' && event.data?.iframe_name === iframeId) {
            if (iframeRef) {
                iframeRef.style.height = `${event.data.height}px`;
            }
        }
    }

    function handleResize() {
        if (iframeRef && iframeRef.contentWindow) {
            iframeRef.contentWindow.postMessage({ type: 'TH_UPDATE_VIEWPORT_HEIGHT' }, '*');
        }
    }

    onMount(() => {
        window.addEventListener('message', handleMessage);
        window.addEventListener('resize', handleResize);
    });

    onDestroy(() => {
        if (typeof window !== 'undefined') {
            window.removeEventListener('message', handleMessage);
            window.removeEventListener('resize', handleResize);
        }
        // Cleanup Blob URL if we used one (implementation detail for future optimization)
    });
    
    // Helper to derive Blob URL if mode is switched (simplified for this version to just use srcdoc for stability first)
    // If useBlobUrl is true, we should convert srcifiedContent to a blob url.
    let finalSrc = $derived(useBlobUrl ? URL.createObjectURL(new Blob([srcifiedContent], { type: 'text/html' })) : undefined);
    let finalSrcDoc = $derived(useBlobUrl ? undefined : srcifiedContent);

</script>

<div class="w-full relative min-h-[50px]">
    <iframe
        bind:this={iframeRef}
        id={iframeId}
        name={iframeId}
        src={finalSrc}
        srcdoc={finalSrcDoc}
        class="w-full border-none overflow-hidden block"
        sandbox="allow-scripts allow-popups allow-forms allow-same-origin allow-modals"
        loading="lazy"
        title="Rendered Content"
    ></iframe>
</div>
