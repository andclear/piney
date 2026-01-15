import { type RegexScript } from "$lib/utils/regexProcessor";

// Use aggressive frontend detection to overcome render2.md's limitation (Section 9.1),
// while maintaining the rest of the robust logic.
export function isFrontend(content: string): boolean {
    return /<[a-zA-Z][\w-]*(\s+[^>]*)?>/i.test(content);
}

// Section 6.1: Replace VH units
export function replaceVhInContent(content: string): string {
    const has_css_min_vh = /min-height\s*:\s*[^;{}]*\d+(?:\.\d+)?vh/gi.test(content);
    const has_inline_style_vh = /style\s*=\s*(["'])[\s\S]*?min-height\s*:\s*[^;]*?\d+(?:\.\d+)?vh[\s\S]*?\1/gi.test(content);
    const has_js_vh = /(\.style\.minHeight\s*=\s*(["']))([\s\S]*?vh)(\2)/gi.test(content) ||
        /(setProperty\s*\(\s*(["'])min-height\2\s*,\s*(["']))([\s\S]*?vh)(\3\s*\))/gi.test(content);

    if (!has_css_min_vh && !has_inline_style_vh && !has_js_vh) {
        return content;
    }

    const convertVhToVariable = (value: string) => {
        return value.replace(/(\d+(?:\.\d+)?)vh\b/gi, (match, v) => {
            const parsed = parseFloat(v);
            if (!isFinite(parsed)) return match;
            const VARIABLE_EXPRESSION = `var(--TH-viewport-height)`;
            if (parsed === 100) return VARIABLE_EXPRESSION;
            return `calc(${VARIABLE_EXPRESSION} * ${parsed / 100})`;
        });
    };

    // CSS
    content = content.replace(
        /(min-height\s*:\s*)([^;]*?\d+(?:\.\d+)?vh)(?=\s*[;}])/gi,
        (_m: string, prefix: string, value: string) => `${prefix}${convertVhToVariable(value)}`
    );

    // Inline Style
    content = content.replace(
        /(style\s*=\s*(["']))([^"'"]*?)(\2)/gi,
        (match: string, prefix: string, quote: string, styleContent: string, suffix: string) => {
            if (!/min-height\s*:\s*[^;]*vh/i.test(styleContent)) return match;
            const replaced = styleContent.replace(
                /(min-height\s*:\s*)([^;]*?\d+(?:\.\d+)?vh)/gi,
                (_m: string, p1: string, p2: string) => `${p1}${convertVhToVariable(p2)}`
            );
            return `${prefix}${replaced}${suffix}`;
        }
    );

    // JS .style.minHeight
    content = content.replace(
        /(\.style\.minHeight\s*=\s*(["']))([\s\S]*?)(\2)/gi,
        (match: string, prefix: string, q: string, val: string, suffix: string) => {
            if (!/\b\d+(?:\.\d+)?vh\b/i.test(val)) return match;
            return `${prefix}${convertVhToVariable(val)}${suffix}`;
        }
    );

    // JS setProperty
    content = content.replace(
        /(setProperty\s*\(\s*(["'])min-height\2\s*,\s*(["']))([\s\S]*?)(\3\s*\))/gi,
        (match: string, prefix: string, q1: string, q2: string, val: string, suffix: string) => {
            if (!/\b\d+(?:\.\d+)?vh\b/i.test(val)) return match;
            return `${prefix}${convertVhToVariable(val)}${suffix}`;
        }
    );

    return content;
}

// Section 6.3: Third Party Libs
const third_party = `
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@fortawesome/fontawesome-free/css/all.min.css" />
<script src="https://cdn.tailwindcss.com"></script>
<script src="https://cdn.jsdelivr.net/npm/jquery/dist/jquery.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/jquery-ui/dist/jquery-ui.min.js"></script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/jquery-ui/themes/base/theme.min.css" />
<script src="https://cdn.jsdelivr.net/npm/jquery-ui-touch-punch"></script>
<script src="https://cdn.jsdelivr.net/npm/vue/dist/vue.runtime.global.prod.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/vue-router/dist/vue-router.global.prod.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/pixi.js/dist/pixi.min.js"></script>
`;

// Section 6.4: Predefine Script (Simplified for our environment where parent might not have everything)
// We Mock 'SillyTavern', 'TavernHelper' etc if missing, to prevent crash, or forward if available.
// Since we are in a Svelte app, 'window.parent' is the Vue/Svelte app.
// We probably don't have 'TavernHelper' on window.parent. 
// For now, we inject a minimal mock or try to check.
// Using explicit string for script content.
const predefine_script = `
(function(){
    try {
        try {
            if(window.parent && window.parent._) {
                window._ = window.parent._;
            }
        } catch(e) { } // Ignore SecurityError if parent is cross-origin
        
        // Mock SillyTavern Context
        const mockContext = { 
            getContext: () => ({}),
            getChatMessages: () => [],
            characters: [],
            chat: []
        };
        let parentST = {};
        try { parentST = window.parent.SillyTavern || {}; } catch(e){}

        Object.defineProperty(window, 'SillyTavern', {
            get: () => (parentST.getContext ? parentST.getContext() : mockContext.getContext())
        });

        // Mock global functions likely used by scripts
        window.getChatMessages = window.getChatMessages || (() => []);
        window.getCurrentMessageId = window.getCurrentMessageId || (() => 0);
        window.this_chid = window.this_chid || 0; 
        
        // Mock STscript (often used in complex cards)
        window.STscript = window.STscript || {};

        const iframeId = window.frameElement?.id || window.name;
        if (iframeId) {
            window.__TH_IFRAME_ID = iframeId;
            if (!window.name) window.name = iframeId;
        }
    } catch(e){ console.error("Predefine Error", e); }
})();
`;

// Section 6.5: Adjust Viewport
const adjust_viewport_script = `
(function(){
    try {
        $('html').css('--TH-viewport-height', \`\${window.innerHeight}px\`);
        window.addEventListener('resize', function(){
             $('html').css('--TH-viewport-height', \`\${window.innerHeight}px\`);
        });
    } catch(e){}
})();
`;

// Section 6.6: Adjust Iframe Height
// Note: We need lodash (_) for throttle. 
// If jquery/lodash not loaded, this fails. Jquery is loaded in third_party. Lodash?
// ST assumes lodash is available. I should probably add lodash to third_party if not present?
// Or assume predefine gets it.
const adjust_height_script = `
(function () {
  // Ensure we wait for libraries
  const init = () => {
      if(!window.$ || !window._) { setTimeout(init, 100); return; }
      
      const IS_BLOB_MODE = window.location.protocol === 'blob:';
      let scheduled = false;

      function measureAndPost() {
        scheduled = false;
        try {
          const doc = window.document;
          const body = doc.body;
          const html = doc.documentElement;
          if (!body || !html) return;

          let height = 0;
          if (IS_BLOB_MODE) {
            const children = Array.from(body.children || []);
            if (children.length > 0) {
              const body_rect = body.getBoundingClientRect();
              const body_style = window.getComputedStyle(body);
              const padding_top = parseFloat(body_style.paddingTop) || 0;
              const padding_bottom = parseFloat(body_style.paddingBottom) || 0;

              let max_top = Infinity;
              let max_bottom = -Infinity;

              for (const el of children) {
                if (!(el instanceof HTMLElement)) continue;
                const rect = el.getBoundingClientRect();
                const style = window.getComputedStyle(el);
                const position = style.position;
                if (position === 'absolute' || position === 'fixed') continue;

                const margin_top = parseFloat(style.marginTop) || 0;
                const margin_bottom = parseFloat(style.marginBottom) || 0;
                const top_with_margin = rect.top - margin_top - body_rect.top;
                const bottom_with_margin = rect.bottom + margin_bottom - body_rect.top;

                if (Number.isFinite(top_with_margin) && top_with_margin < max_top) max_top = top_with_margin;
                if (Number.isFinite(bottom_with_margin) && bottom_with_margin > max_bottom) max_bottom = bottom_with_margin;
              }

              if (Number.isFinite(max_top) && Number.isFinite(max_bottom) && max_bottom > max_top) {
                const content_height = max_bottom - max_top;
                const total_height = content_height + padding_top + padding_bottom;
                if (Number.isFinite(total_height) && total_height > 0) height = total_height;
              }
            }
            if (!Number.isFinite(height) || height <= 0) height = body.scrollHeight;
          } else {
            height = body.scrollHeight;
          }

          if (!Number.isFinite(height) || height <= 0) return;
          
          window.parent.postMessage({ type: 'TH_ADJUST_IFRAME_HEIGHT', iframe_name: window.name, height: height }, '*');
        } catch (e) { console.error(e); }
      }

      const throttledMeasureAndPost = _.throttle(measureAndPost, 500);

      function postIframeHeight() {
        if (scheduled) return;
        scheduled = true;
        if (typeof window.requestAnimationFrame === 'function') window.requestAnimationFrame(measureAndPost);
        else throttledMeasureAndPost();
      }

      function observeHeightChange() {
        const body = document.body;
        if (!body) return;
        const resize_observer = new ResizeObserver(() => postIframeHeight());
        resize_observer.observe(body);

        if (IS_BLOB_MODE) {
          const mutation_observer = new MutationObserver(() => {
            resize_observer.disconnect();
            for (const element of body.children) resize_observer.observe(element);
            resize_observer.observe(body);
            postIframeHeight();
          });
          mutation_observer.observe(body, { childList: true, subtree: true, attributes: true });
        }
      }

      $(() => {
        postIframeHeight();
        observeHeightChange();
        // Also observe clicks which might expand content
        $(document).on('click', () => setTimeout(postIframeHeight, 100));
      });
  };
  init();
})();
`;

// Helper for avatars (placeholders)
function getUserAvatarPath() { return ''; }
function getCharAvatarPath() { return ''; }

// Dark Mode Sync
const dark_mode_sync_script = `
(function(){
    try {
        const syncDark = () => {
           try {
               // Check parent 'dark' class
               const isDark = window.parent.document.documentElement.classList.contains('dark');
               if(isDark) document.documentElement.classList.add('dark');
               else document.documentElement.classList.remove('dark');
           } catch(e) {}
        };
        syncDark();
        // Observe parent for changes (might throw SecurityError)
        try {
            const observer = new MutationObserver(syncDark);
            observer.observe(window.parent.document.documentElement, { attributes: true, attributeFilter: ['class'] });
        } catch(e){}
    } catch(e){}
})();
`;

// Dynamic Update Listener
const dynamic_update_script = `
(function(){
    try {
        window.addEventListener('message', function(event) {
            if (event.data && event.data.type === 'TH_UPDATE_CONTENT') {
                if (event.data.content !== undefined) {
                    document.body.innerHTML = event.data.content;
                }
                if (event.data.isDark !== undefined) {
                    if (event.data.isDark) document.documentElement.classList.add('dark');
                    else document.documentElement.classList.remove('dark');
                }
            }
        });
    } catch(e) { console.error("Dynamic Update Error", e); }
})();
`;

// Section 6.2: Create Src Content
// Section 6.2: Create Src Content
const prevent_nav_script = `
(function(){
    document.addEventListener('click', function(e) {
        const a = e.target.closest('a');
        if (a) {
            const href = a.getAttribute('href');
            // Prevent navigation for placeholder links which cause iframe to load parent page
            if (!href || href === '#' || href.startsWith('#')) {
                e.preventDefault();
            } else {
                 // For external links, force new tab to avoid breaking iframe
                 if (href.startsWith('http')) {
                     a.target = '_blank';
                 }
            }
        }
    });
})();
`;

export function createSrcContent(content: string, useBlobUrl: boolean = false, isDark: boolean = false): string {
    content = replaceVhInContent(content);

    // Use passed isDark parameter to set the class directly
    const initialDarkClass = isDark ? ' class="dark"' : '';

    return `<!DOCTYPE html>
<html${initialDarkClass}>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<base href="about:blank">
<style>
:root { color-scheme: light dark; }
*,*::before,*::after{box-sizing:border-box;}
html{background-color:transparent !important;}
body{margin:0!important;padding:0;overflow:hidden!important;max-width:100%!important;background-color:transparent!important;}
.user_avatar,.user-avatar{background-image:url('${getUserAvatarPath()}')}
.char_avatar,.char-avatar{background-image:url('${getCharAvatarPath()}')}
</style>
${third_party}
<!-- Inject Scripts Inline to avoid URL issues -->
<script>${predefine_script}</script>
<script>${prevent_nav_script}</script>
<script src="https://testingcf.jsdelivr.net/npm/lodash@4.17.21/lodash.min.js"></script> 
<!-- Added Lodash explicit load just in case -->
<script>${adjust_viewport_script}</script>
<script>${adjust_height_script}</script>
<script>
    // Forward Navigation Events to Parent (SillyReader Style Logic)
    (function(){
        try {
            // Keyboard: Forward specific keys
            window.addEventListener('keydown', function(e) {
                const target = e.target;
                if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return;

                if(e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key.toLowerCase() === 'a' || e.key.toLowerCase() === 'd') {
                    try { window.parent.postMessage({ type: 'TH_NAVIGATE', key: e.key }, '*'); } catch(e){}
                }
            });

            // Touch Swipe
            let tStartX = 0;
            let tStartY = 0;
            let tStartTime = 0;
            
            window.addEventListener('touchstart', function(e) {
                tStartX = e.changedTouches[0].clientX;
                tStartY = e.changedTouches[0].clientY;
                tStartTime = Date.now();
            }, { passive: true });

            window.addEventListener('touchend', function(e) {
                const tEndX = e.changedTouches[0].clientX;
                const tEndY = e.changedTouches[0].clientY;
                const diffX = tEndX - tStartX;
                const diffY = tEndY - tStartY;
                const duration = Date.now() - tStartTime;

                if (duration < 300 && Math.abs(diffX) > 50 && Math.abs(diffX) > Math.abs(diffY) * 1.5) {
                    try { window.parent.postMessage({ type: 'TH_NAVIGATE', swipe: diffX > 0 ? 'right' : 'left' }, '*'); } catch(e){}
                }
            }, { passive: true });
        } catch (e) { console.error("Nav Script Error", e); }
    })();
</script>
<script>${dark_mode_sync_script}</script>
<script>${dynamic_update_script}</script>
</head>
<body>
${content}
</body>
</html>
    `;
}

export function unwrapFrontendCodeBlocks(htmlContent: string): string {
    // Improved Regex to handle attributes and whitespace in pre/code tags
    return htmlContent.replace(/<pre[^>]*>\s*<code[^>]*>([\s\S]*?)<\/code>\s*<\/pre>/gi, (match, code) => {
        // Decode common entities produced by marked
        const decoded = code
            .replace(/&lt;/g, '<')
            .replace(/&gt;/g, '>')
            .replace(/&quot;/g, '"')
            .replace(/&#39;/g, "'")
            .replace(/&amp;/g, '&');

        if (isFrontend(decoded)) {
            // Keep a wrapper for potential styling, or just return raw decoded
            return `<div class="TH-render">${decoded}</div>`;
        }
        return match;
    });
}
