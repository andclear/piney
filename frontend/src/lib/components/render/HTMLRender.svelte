<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { createSrcContent, isFrontend, replaceVhInContent } from '$lib/utils/renderUtils';

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
    
    // State
    let srcifiedContent = $state('');
    let isDark = $state(false);
    let isLoaded = $state(false);

    // Watch for class changes on <html> element to detect dark mode
    let observer: MutationObserver;

    onMount(() => {
        // Initial check
        if (typeof document !== 'undefined') {
            isDark = document.documentElement.classList.contains('dark');
        }

        // Setup observer
        if (typeof window !== 'undefined') {
            observer = new MutationObserver((mutations) => {
                mutations.forEach((mutation) => {
                    if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
                        isDark = document.documentElement.classList.contains('dark');
                    }
                });
            });
            
            observer.observe(document.documentElement, {
                attributes: true,
                attributeFilter: ['class']
            });
        }
    });

    onDestroy(() => {
        if (observer) observer.disconnect();
        if (typeof window !== 'undefined') {
            window.removeEventListener('message', handleMessage);
            window.removeEventListener('resize', handleResize);
        }
    });

    // Handle iframe load event
    function handleLoad() {
        console.log("TH-Render: Iframe Loaded", iframeId);
        isLoaded = true;
        // Trigger an immediate update just in case content changed while loading
        if (iframeRef && iframeRef.contentWindow) {
            const processedContent = replaceVhInContent(content);
            try {
                iframeRef.contentWindow.postMessage({ 
                    type: 'TH_UPDATE_CONTENT', 
                    content: processedContent,
                    isDark: isDark 
                }, '*');
            } catch (e) {
                console.error("TH-Render: PostMessage Failed", e);
            }
        }
    }

    // Effect to handle content updates
    $effect(() => {
        // Capture dependent values
        const currentContent = content;
        const currentDark = isDark;
        const loaded = isLoaded;

        if (loaded && iframeRef && iframeRef.contentWindow) {
             // Hot Update: Post message update without reloading iframe
             const processedContent = replaceVhInContent(currentContent);
             iframeRef.contentWindow.postMessage({ 
                 type: 'TH_UPDATE_CONTENT', 
                 content: processedContent,
                 isDark: currentDark 
             }, '*');
        } else {
             // Initial Load logic (or if not loaded yet)
             // Only update srcifiedContent if it's materially different effectively (though Svelte handles derived)
             // or if we haven't loaded yet.
             
             // To prevent infinite reload loops if srcdoc updates reload the iframe -> onload -> effect:
             // We generally only set srcifiedContent ONCE or when we specifically want to reload.
             // If we rely on srcdoc for the first paint.
             
             if (!srcifiedContent) {
                 srcifiedContent = createSrcContent(currentContent, useBlobUrl, currentDark);
             }
             // If content changes significantly while !isLoaded, we might want to update srcifiedContent?
             // But simpler to just let handleLoad sync it.
        }
    });
    
    // Also watch for major mode changes that might require full reload (e.g. Blob vs raw)
    // But for now, we stick to srcdoc usually.

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
    
    // Helper to derive Blob URL if mode is switched
    let finalSrc = $derived(useBlobUrl ? URL.createObjectURL(new Blob([srcifiedContent], { type: 'text/html' })) : undefined);
    let finalSrcDoc = $derived(useBlobUrl ? undefined : srcifiedContent);

</script>

<div class="w-full relative min-h-[50px] TH-render" style="background:transparent;">
    <iframe
        bind:this={iframeRef}
        id={iframeId}
        name={iframeId}
        src={finalSrc}
        srcdoc={finalSrcDoc}
        onload={handleLoad}
        class="w-full border-none overflow-hidden block bg-white/5"
        style="background:transparent;"
        sandbox="allow-scripts allow-popups allow-forms allow-same-origin allow-modals"
        loading="lazy"
        allowtransparency={true}
        title="Rendered Content"
    ></iframe>
</div>
