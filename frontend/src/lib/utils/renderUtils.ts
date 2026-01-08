export function isFrontend(content: string): boolean {
    return ['html>', '<head>', '<body'].some(tag => content.includes(tag));
}

export function replaceVhInContent(content: string): string {
    // Basic detection
    const has_css_min_vh = /min-height\s*:\s*[^;{}]*\d+(?:\.\d+)?vh/gi.test(content);

    const convertVhToVariable = (value: string) =>
        value.replace(/(\d+(?:\.\d+)?)vh\b/gi, (match, val) => {
            const parsed = parseFloat(val);
            if (parsed === 100) {
                return `var(--TH-viewport-height)`;
            }
            return `calc(var(--TH-viewport-height) * ${parsed / 100})`;
        });

    // Replace in CSS min-height declarations
    let processed = content.replace(
        /(min-height\s*:\s*)([^;]*?\d+(?:\.\d+)?vh)(?=\s*[;}])/gi,
        (_m, prefix, value) => `${prefix}${convertVhToVariable(value)}`,
    );

    // Replace in CSS height declarations
    processed = processed.replace(
        /(height\s*:\s*)([^;]*?\d+(?:\.\d+)?vh)(?=\s*[;}])/gi,
        (_m, prefix, value) => `${prefix}${convertVhToVariable(value)}`,
    );

    return processed;
}

const third_party = `
<script src="https://testingcf.jsdelivr.net/npm/vue/dist/vue.global.prod.js"></script>
<script src="https://testingcf.jsdelivr.net/npm/jquery/dist/jquery.min.js"></script>
<script src="https://testingcf.jsdelivr.net/npm/jquery-ui/dist/jquery-ui.min.js"></script>
<link rel="stylesheet" href="https://testingcf.jsdelivr.net/npm/jquery-ui/themes/base/theme.min.css" />
<script src="https://testingcf.jsdelivr.net/npm/jquery-ui-touch-punch"></script>
<script src="https://testingcf.jsdelivr.net/npm/pixi.js/dist/pixi.min.js"></script>
<script src="https://testingcf.jsdelivr.net/npm/tailwindcss@3.4.1/lib/index.min.js"></script> <!-- Attempting to use a CDN version, might need specific build -->
<script src="https://cdn.tailwindcss.com"></script>
`;

const predefine_script = `
// Inherit lodash if available (mocking minimal needed)
window._ = window.parent._ || {
    get: (obj, path) => path.split('.').reduce((acc, part) => acc && acc[part], obj),
    has: (obj, path) => !!path.split('.').reduce((acc, part) => acc && acc[part], obj),
    pick: (obj, keys) => keys.reduce((acc, key) => { if (key in obj) acc[key] = obj[key]; return acc; }, {}),
    merge: (target, source) => Object.assign(target, source)
};

// Mock SillyTavern Context
window.SillyTavern = {
    getContext: () => ({
        characterId: window.parent?.cardId || 'unknown', // Example context
    })
};

const iframeId = window.frameElement?.id || window.name;
if (iframeId) {
    window.__TH_IFRAME_ID = iframeId;
    window.name = iframeId;
}
`;

const adjust_height_script = `
function getIframeName() {
    return window.name || window.__TH_IFRAME_ID;
}

function measureAndPost() {
    const body = document.body;
    let height = body.scrollHeight;
    
    // Check if height is 0 (can happen if content isn't rendered yet), maybe use offsetHeight as fallback
    if (height === 0) height = body.offsetHeight;

    window.parent.postMessage({ 
        type: 'TH_ADJUST_IFRAME_HEIGHT', 
        iframe_name: getIframeName(), 
        height 
    }, '*');
}

// Observe Resize
const resize_observer = new ResizeObserver(() => measureAndPost());
if (document.body) {
    resize_observer.observe(document.body);
    // Also observe mutations as fallback
    const mutation_observer = new MutationObserver(() => measureAndPost());
    mutation_observer.observe(document.body, { childList: true, subtree: true, attributes: true });
}

window.addEventListener('load', measureAndPost);
// Polling for safety in early render stages
setTimeout(measureAndPost, 50);
setTimeout(measureAndPost, 200);
setTimeout(measureAndPost, 500);
`;

const adjust_viewport_script = `
// Initialize viewport height
document.documentElement.style.setProperty('--TH-viewport-height', (window.parent.innerHeight) + 'px');

window.addEventListener('message', function (event) {
    if (event.data?.type === 'TH_UPDATE_VIEWPORT_HEIGHT') {
        document.documentElement.style.setProperty('--TH-viewport-height', (window.parent.innerHeight) + 'px');
    }
});
`;

export function createSrcContent(content: string, useBlobUrl: boolean): string {
    let processedContent = replaceVhInContent(content);

    return `
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
${useBlobUrl ? `<base href="${window.location.origin}"/>` : ''}
<style>
/* Reset and base styles */
*,*::before,*::after{box-sizing:border-box;}
html,body {
    margin:0 !important;
    padding:0;
    overflow:hidden !important;
    max-width: 100% !important; /* From docs/render.md */
    white-space: pre-wrap; /* Preserve text formatting/newlines */
    /* Removed min-height: 100% to allow proper auto-shrinking */
}
/* Default avatars - simplified for now */
.user_avatar,.user-avatar{background-color: #ccc;}
.char_avatar,.char-avatar{background-color: #888;}
</style>

${third_party}

<script>
${predefine_script}
</script>

<script>
${adjust_viewport_script}
</script>
</head>
<body>
${processedContent}

<script>
${adjust_height_script}
</script>
</body>
</html>
`;
}
