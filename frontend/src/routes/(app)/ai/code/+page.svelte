<script lang="ts">
    import { onMount } from 'svelte';
    import { breadcrumbs } from '$lib/stores/breadcrumb';
    import { Button } from '$lib/components/ui/button';
    import { Textarea } from '$lib/components/ui/textarea';
    import { Input } from '$lib/components/ui/input';
    import * as Tabs from '$lib/components/ui/tabs';
    import * as Sheet from '$lib/components/ui/sheet';
    import * as Dialog from '$lib/components/ui/dialog';
    import { toast } from 'svelte-sonner';
    import { AiService } from '$lib/ai/service';
    import Sparkles from '@lucide/svelte/icons/sparkles';
    import Eye from '@lucide/svelte/icons/eye';
    import Code from '@lucide/svelte/icons/code';
    import Wand2 from '@lucide/svelte/icons/wand-2';
    import Plus from '@lucide/svelte/icons/plus';
    import Save from '@lucide/svelte/icons/save';
    import Library from '@lucide/svelte/icons/library';
    import Send from '@lucide/svelte/icons/send';
    import Copy from '@lucide/svelte/icons/copy';
    import Layers from '@lucide/svelte/icons/layers';
    import Trash2 from '@lucide/svelte/icons/trash-2';
    import MousePointer from '@lucide/svelte/icons/mouse-pointer';
    import Wrench from '@lucide/svelte/icons/wrench';
    import Loader2 from '@lucide/svelte/icons/loader-2';
    import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
    import X from '@lucide/svelte/icons/x';
    import IdCard from '@lucide/svelte/icons/id-card';
    import { formatHtml } from '$lib/utils/renderUtils';
    import InsertToCardDialog from '$lib/components/ai/InsertToCardDialog.svelte';
    import { API_BASE } from '$lib/api';

    // 设置面包屑导航
    onMount(() => {
        breadcrumbs.set([
            { label: '皮皮美化工作台' }
        ]);
    });

    // ==================== 状态 ====================
    
    // 原始文本
    let originalText = $state('');
    
    // AI 生成的内容
    let regexPattern = $state('');
    let htmlCode = $state('');
    let worldinfoKey = $state('');
    let worldinfoContent = $state('');
    
    // 对话状态
    let chatInput = $state('');
    // ChatMessage 支持 selectedTag 字段用于显示 #tag 标签
    let chatHistory = $state<{ role: 'user' | 'assistant'; content: string; selectedTag?: string }[]>([]);
    let isGenerating = $state(false);
    let isFirstGeneration = $state(true);
    
    // UI 状态
    let activeTab = $state('preview');
    let editMode = $state(false);
    let selectedElement = $state('');
    let previewIframe: HTMLIFrameElement | null = $state(null);
    let renderMode = $state<'code' | 'full'>('code'); // 渲染模式：仅代码 / 完整内容
    
    // 样式库
    let libraryOpen = $state(false);
    let styleLibrary = $state<{ id: string; name: string; updated_at: string }[]>([]);
    
    // 保存对话框
    let saveDialogOpen = $state(false);
    let saveName = $state('');
    let currentStyleId = $state<string | null>(null);
    
    // ==================== 计算属性 ====================
    
    // 预览 iframe 内容（带交互式编辑支持）
    let previewSrcDoc = $derived.by(() => {
        if (!htmlCode.trim()) return '';
        
        // 获取主题色（从 CSS 变量）
        const getPrimaryColor = () => {
            const style = getComputedStyle(document.documentElement);
            const h = style.getPropertyValue('--primary').trim();
            // HSL 格式转换为可用的颜色
            return h ? `hsl(${h})` : '#3b82f6'; // 默认蓝色
        };
        const primaryColor = editMode ? getPrimaryColor() : '';
        const primaryColorHex = editMode ? (getComputedStyle(document.documentElement).getPropertyValue('--primary').trim() ? primaryColor : '#3b82f6') : '';
        
        // 交互式编辑模式脚本 - 使用主题色
        const editModeScript = editMode ? `
            <style>
                .piney-edit-badge {
                    position: fixed;
                    background: ${primaryColor};
                    color: white;
                    padding: 4px 10px;
                    border-radius: 4px;
                    font-size: 12px;
                    font-weight: 500;
                    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
                    cursor: pointer;
                    z-index: 99999;
                    display: none;
                    white-space: nowrap;
                }
                .piney-edit-badge:hover {
                    filter: brightness(0.9);
                }
            </style>
            <div class="piney-edit-badge" id="piney-badge">添加到对话框</div>
            <script>
                const PRIMARY_COLOR = '${primaryColor}';
                const PRIMARY_COLOR_LIGHT = '${primaryColor}80'; // 带透明度
                
                let selectedEl = null;
                let hoverEl = null;
                const badge = document.getElementById('piney-badge');
                
                function showBadge(el) {
                    const rect = el.getBoundingClientRect();
                    badge.style.left = rect.left + 'px';
                    badge.style.top = Math.max(4, rect.top - 28) + 'px';
                    badge.style.display = 'block';
                }
                
                document.body.addEventListener('click', (e) => {
                    if (e.target === badge) return;
                    e.preventDefault();
                    e.stopPropagation();
                    
                    // 清除之前选中
                    if (selectedEl) {
                        selectedEl.style.outline = '';
                        selectedEl.style.outlineOffset = '';
                    }
                    
                    selectedEl = e.target;
                    selectedEl.style.outline = '2px solid ' + PRIMARY_COLOR;
                    selectedEl.style.outlineOffset = '2px';
                    
                    showBadge(selectedEl);
                });
                
                badge.addEventListener('click', (e) => {
                    e.stopPropagation();
                    if (!selectedEl) return;
                    
                    // 发送选中元素信息给父窗口
                    const info = {
                        tagName: selectedEl.tagName,
                        className: selectedEl.className,
                        id: selectedEl.id,
                        textContent: selectedEl.textContent?.slice(0, 200),
                        outerHTML: selectedEl.outerHTML
                    };
                    window.parent.postMessage({ type: 'elementSelected', data: info }, '*');
                    badge.style.display = 'none';
                });
                
                // 悬停高亮（使用主题色虚线）
                document.body.addEventListener('mouseover', (e) => {
                    if (e.target !== selectedEl && e.target !== badge && !badge.contains(e.target)) {
                        hoverEl = e.target;
                        hoverEl.style.outline = '1px dashed ' + PRIMARY_COLOR_LIGHT;
                    }
                });
                document.body.addEventListener('mouseout', (e) => {
                    if (e.target !== selectedEl && e.target === hoverEl) {
                        e.target.style.outline = '';
                    }
                });
            <\/script>
        ` : '';

        // 自动高度调整脚本
        const resizeScript = `
            <script>
                const resizeObserver = new ResizeObserver(() => {
                    // 发送高度信息（加一点 buffer 防止闪烁）
                    const height = document.documentElement.scrollHeight;
                    window.parent.postMessage({ type: 'resize', height: height }, '*');
                });
                resizeObserver.observe(document.body);
            <\/script>
        `;
        
        // 编辑模式使用十字光标指针
        const editModeStyle = editMode ? `
            * { cursor: crosshair !important; }
        ` : '';
        
        return `<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * { box-sizing: border-box; }
        body { 
            margin: 0; 
            padding: 16px; 
            font-family: system-ui, sans-serif;
            overflow-y: hidden; /* 防止 iframe 内部双重滚动 */
        }
        
        /* 强制修复 details/summary 交互 */
        /* 强制修复 details/summary 交互 (Aggressive Reset) */
        details, summary { 
            display: block; 
            pointer-events: auto !important; 
        }
        details > summary { 
            cursor: pointer !important; 
            list-style: none;
        }
        details > summary::-webkit-details-marker {
            display: none;
        }
        /* Re-add a visible marker if AI doesn't provide one, or rely on AI providing one? 
           Let's just ensure it's clickable. If AI hides marker, that's fine. 
           But we must ensure pointer-events: auto works even inside disabled containers. 
        */
        *:where(button, a, input, select, textarea, details, summary) {
            pointer-events: auto !important;
        }
        
        details > summary:hover { opacity: 0.8; }
        
        ${editModeStyle}
    </style>
</head>
<body>${htmlCode}${editModeScript}${resizeScript}</body>
</html>`;
    });
    
    // 完整内容渲染（应用正则替换）
    let fullContentHtml = $derived.by(() => {
        if (!htmlCode.trim() || !regexPattern.trim() || !originalText.trim()) {
            return htmlCode; // 回退到纯代码
        }
        
        try {
            const regex = new RegExp(regexPattern, 'gms');
            // 直接尝试匹配
            const replaced = originalText.replace(regex, htmlCode);
            
            // 如果替换后和原文相同，说明没有匹配到
            if (replaced === originalText) {
                return 'REGEX_MISMATCH_ERROR';
            }
            
            return replaced;
        } catch (e) {
            console.error('正则替换失败:', e);
            return 'REGEX_SYNTAX_ERROR';
        }
    });
    
    // 根据渲染模式选择内容
    let renderedContent = $derived.by(() => {
        if (renderMode === 'code') return htmlCode;
        
        if (fullContentHtml === 'REGEX_MISMATCH_ERROR') {
             return `<div style="display:flex;height:100%;align-items:center;justify-content:center;color:#ef4444;background:#fef2f2;font-family:system-ui,sans-serif;text-align:center;padding:20px;">
               <div>
                 <div style="font-size:24px;margin-bottom:12px;">⚠️</div>
                 <div style="font-weight:bold;margin-bottom:8px;font-size:16px;">正则匹配失败</div>
                 <div style="font-size:13px;opacity:0.8;line-height:1.5;">
                    原始内容与正则规则不兼容。<br>
                    请点击下方的"修复"按钮，让 AI 自动修正。
                 </div>
               </div>
            </div>`;
        }
        
        if (fullContentHtml === 'REGEX_SYNTAX_ERROR') {
             return `<div style="padding:20px;color:red;">正则语法错误</div>`;
        }
        
        return fullContentHtml;
    });
    
    // 渲染最终 HTML
    let finalPreviewSrcDoc = $derived.by(() => {
        if (!renderedContent.trim()) return '';
        
        // 获取主题色（从 CSS 变量）
        const getPrimaryColor = () => {
            const style = getComputedStyle(document.documentElement);
            const h = style.getPropertyValue('--primary').trim();
            return h ? `hsl(${h})` : '#3b82f6';
        };
        const primaryColor = editMode ? getPrimaryColor() : '';
        
        // 编辑模式脚本（仅在代码模式下启用）
        const editModeScript = (editMode && renderMode === 'code') ? `
            <style>
                .piney-edit-badge {
                    position: fixed;
                    background: ${primaryColor};
                    color: white;
                    padding: 4px 10px;
                    border-radius: 4px;
                    font-size: 12px;
                    font-weight: 500;
                    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
                    cursor: pointer;
                    z-index: 99999;
                    display: none;
                    white-space: nowrap;
                }
                .piney-edit-badge:hover {
                    filter: brightness(0.9);
                }
            </style>
            <div class="piney-edit-badge" id="piney-badge">添加到对话框</div>
            <script>
                const PRIMARY_COLOR = '${primaryColor}';
                const PRIMARY_COLOR_LIGHT = '${primaryColor}80';

                let selectedEl = null;
                let hoverEl = null;
                const badge = document.getElementById('piney-badge');

                function showBadge(el) {
                    const rect = el.getBoundingClientRect();
                    badge.style.left = rect.left + 'px';
                    badge.style.top = Math.max(4, rect.top - 28) + 'px';
                    badge.style.display = 'block';
                }

                document.body.addEventListener('click', (e) => {
                    if (e.target === badge) return;
                    e.preventDefault();
                    e.stopPropagation();

                    if (selectedEl) {
                        selectedEl.style.outline = '';
                        selectedEl.style.outlineOffset = '';
                    }

                    selectedEl = e.target;
                    selectedEl.style.outline = '2px solid ' + PRIMARY_COLOR;
                    selectedEl.style.outlineOffset = '2px';

                    showBadge(selectedEl);
                });

                badge.addEventListener('click', (e) => {
                    e.stopPropagation();
                    if (!selectedEl) return;

                    const info = {
                        tagName: selectedEl.tagName,
                        className: selectedEl.className,
                        id: selectedEl.id,
                        textContent: selectedEl.textContent?.slice(0, 200),
                        outerHTML: selectedEl.outerHTML
                    };
                    window.parent.postMessage({ type: 'elementSelected', data: info }, '*');
                    badge.style.display = 'none';
                });

                document.body.addEventListener('mouseover', (e) => {
                    if (e.target !== selectedEl && e.target !== badge && !badge.contains(e.target)) {
                        hoverEl = e.target;
                        hoverEl.style.outline = '1px dashed ' + PRIMARY_COLOR_LIGHT;
                    }
                });
                document.body.addEventListener('mouseout', (e) => {
                    if (e.target !== selectedEl && e.target === hoverEl) {
                        e.target.style.outline = '';
                    }
                });
            <\/script>
        ` : '';
        
        // 自动高度调整脚本
        const resizeScript = `
            <script>
                const resizeObserver = new ResizeObserver(() => {
                    const height = document.documentElement.scrollHeight;
                    window.parent.postMessage({ type: 'resize', height: height }, '*');
                });
                resizeObserver.observe(document.body);
            <\/script>
        `;
        
        const editModeStyle = (editMode && renderMode === 'code') ? `* { cursor: crosshair !important; }` : '';
        
        return `<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * { box-sizing: border-box; }
        body { 
            margin: 0; 
            padding: 16px; 
            font-family: system-ui, sans-serif;
            overflow-y: hidden;
        }
        
        details, summary { display: block; pointer-events: auto !important; }
        details > summary { cursor: pointer !important; list-style: none; }
        details > summary::-webkit-details-marker { display: none; }
        *:where(button, a, input, select, textarea, details, summary) { pointer-events: auto !important; }
        details > summary:hover { opacity: 0.8; }
        
        ${editModeStyle}
    </style>
</head>
<body>${renderedContent}${editModeScript}${resizeScript}</body>
</html>`;
    });
    
    // ==================== API 调用 ====================
    
    // 获取认证头
    function getAuthHeaders(): Record<string, string> {
        const token = localStorage.getItem('auth_token');
        return token ? { 'Authorization': `Bearer ${token}` } : {};
    }
    
    async function loadStyleLibrary() {
        try {
            const res = await fetch('/api/frontend-styles', {
                headers: getAuthHeaders()
            });
            if (res.ok) {
                styleLibrary = await res.json();
            }
        } catch (e) {
            console.error('加载样式库失败', e);
        }
    }
    
    async function loadStyle(id: string) {
        try {
            const res = await fetch(`/api/frontend-styles/${id}`, {
                headers: getAuthHeaders()
            });
            if (res.ok) {
                const data = await res.json();
                currentStyleId = data.id;
                saveName = data.name;
                originalText = data.original_text;
                regexPattern = data.regex_pattern;
                htmlCode = data.html_code;
                worldinfoKey = data.worldinfo_key;
                worldinfoContent = data.worldinfo_content;
                libraryOpen = false;
                isFirstGeneration = false; // 加载后视为已有内容
                toast.success(`已加载样式: ${data.name}`);
            }
        } catch (e) {
            toast.error('加载样式失败');
        }
    }
    
    async function saveStyle() {
        if (!saveName.trim()) {
            toast.error('请输入样式名称');
            return;
        }
        
        try {
            const payload = {
                name: saveName,
                original_text: originalText,
                regex_pattern: regexPattern,
                html_code: htmlCode,
                worldinfo_key: worldinfoKey,
                worldinfo_content: worldinfoContent,
            };
            
            let res;
            if (currentStyleId) {
                res = await fetch(`/api/frontend-styles/${currentStyleId}`, {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json', ...getAuthHeaders() },
                    body: JSON.stringify(payload),
                });
            } else {
                res = await fetch('/api/frontend-styles', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json', ...getAuthHeaders() },
                    body: JSON.stringify(payload),
                });
            }
            
            if (res.ok) {
                const data = await res.json();
                currentStyleId = data.id;
                saveDialogOpen = false;
                toast.success('保存成功');
                loadStyleLibrary();
            } else {
                const err = await res.json();
                toast.error(err.error || '保存失败');
            }
        } catch (e) {
            toast.error('保存失败');
        }
    }
    
    async function deleteStyle(id: string) {
        try {
            const res = await fetch(`/api/frontend-styles/${id}`, { 
                method: 'DELETE',
                headers: getAuthHeaders()
            });
            if (res.ok) {
                toast.success('删除成功');
                loadStyleLibrary();
                if (currentStyleId === id) {
                    handleNew();
                }
            }
        } catch (e) {
            toast.error('删除失败');
        }
    }
    
    // ==================== AI 生成 ====================
    
    // 生成进度消息
    let progressMessage = $state('');
    
    async function handleSendMessage() {
        if (!chatInput.trim() || isGenerating) return;
        
        let userMessage = chatInput.trim();
        const currentTag = selectedTagName; // 保存当前选中的标签
        chatInput = '';
        
        // 如果消息以 #tag 开头，从内容中移除（避免重复显示）
        if (currentTag && userMessage.startsWith(`#${currentTag}`)) {
            userMessage = userMessage.slice(`#${currentTag}`.length).trim();
        }
        
        // 添加用户消息（包含选中的标签信息）
        chatHistory = [...chatHistory, { 
            role: 'user', 
            content: userMessage,
            selectedTag: currentTag || undefined
        }];
        
        isGenerating = true;
        
        // 显示进度消息
        const showProgress = async () => {
            // 判断是否为仅修改代码模式（首轮但有选中元素）
            const isCodeOnlyMode = isFirstGeneration && selectedElement;
            
            if (isFirstGeneration && !isCodeOnlyMode) {
                // 完整生成模式
                progressMessage = '正在创建世界书条目规则...';
                await new Promise(r => setTimeout(r, 1500));
                if (!isGenerating) return;
                progressMessage = '正在创建正则表达式...';
                await new Promise(r => setTimeout(r, 1500));
                if (!isGenerating) return;
                progressMessage = '正在创建前端样式代码...';
            } else {
                // 修改模式（包括 code-only 和后续修改）
                progressMessage = '正在修改代码...';
            }
        };
        
        // 启动进度显示
        showProgress();
        
        try {
            // 判断是否为仅修改代码模式（首轮但有选中元素）
            const isCodeOnlyMode = isFirstGeneration && selectedElement;
            
            const result = await AiService.generateFrontendStyle({
                originalText: originalText,
                userRequest: userMessage,
                // 关键修复：code-only 模式下也需要传递 htmlCode
                currentHtml: (isFirstGeneration && !isCodeOnlyMode) ? undefined : htmlCode,
                currentRegex: isFirstGeneration ? undefined : regexPattern,
                currentWorldinfoKey: isFirstGeneration ? undefined : worldinfoKey,
                currentWorldinfoContent: isFirstGeneration ? undefined : worldinfoContent,
                selectedElement: selectedElement || undefined,
                isFirstRound: isFirstGeneration
            });
            
            // 更新生成内容
            if (result.worldinfo) {
                worldinfoKey = result.worldinfo.key || worldinfoKey;
                worldinfoContent = result.worldinfo?.content || '';
            }
            regexPattern = result.regex || '';
            htmlCode = formatHtml(result.html || '');
            
            // 如果首轮返回了 original_text 且用户未提供，则填充
            if (result.original_text && !originalText.trim()) {
                originalText = result.original_text;
            }

            // 如果 AI 返回了格式化后的原始文本（适配正则），覆盖当前的原始文本
            if (result.formatted_original_text) {
                originalText = result.formatted_original_text;
                toast.info('已自动格式化原始文本以匹配正则规则');
            }
            
            isFirstGeneration = false;
            
            // 添加助手消息
            chatHistory = [...chatHistory, { 
                role: 'assistant', 
                content: '已生成/更新样式！请查看右侧预览和 AI 输出选项卡。' 
            }];
            
            // 清除选中元素
            selectedElement = '';
            selectedTagName = '';
            
            // 自动切换到预览
            activeTab = 'preview';
            
            toast.success('生成成功');
            
        } catch (e: any) {
            console.error('AI 生成失败', e);
            chatHistory = [...chatHistory, { 
                role: 'assistant', 
                content: `生成失败: ${e.message || '未知错误'}` 
            }];
            toast.error(e.message || '生成失败');
        } finally {
            isGenerating = false;
            progressMessage = '';
        }
    }

    // 修复正则（当正则与原始文本不匹配时调用）
    async function fixRegex() {
        if (isGenerating) return;
        isGenerating = true;
        progressMessage = '正在分析格式并修复正则...';
        
        try {
            const result = await AiService.generateFrontendStyle({
                originalText: originalText,
                userRequest: "修复正则匹配问题", // 这个参数在 fixMode 下不会被主要使用，但需要传递
                currentHtml: htmlCode,
                currentRegex: regexPattern,
                currentWorldinfoKey: worldinfoKey,
                currentWorldinfoContent: worldinfoContent,
                selectedElement: undefined,
                isFirstRound: false,
                isFixMode: true // 开启修复模式
            });
            
            // 更新所有内容
            if (result.worldinfo) {
                worldinfoKey = result.worldinfo.key || worldinfoKey;
                worldinfoContent = result.worldinfo.content || worldinfoContent;
            }
            regexPattern = result.regex || regexPattern;
            htmlCode = result.html ? formatHtml(result.html) : htmlCode;
            
            // 关键：更新格式化后的原始文本
            if (result.formatted_original_text) {
                originalText = result.formatted_original_text;
                toast.success('正则与格式已修复！');
            } else {
                toast.success('正则已更新');
            }
            
            // 添加系统消息
            chatHistory = [...chatHistory, { 
                role: 'assistant', 
                content: '已根据世界书格式修复了正则表达式和原始文本。现在应该能正确匹配了。' 
            }];
            
        } catch (e) {
            console.error('修复失败', e);
            toast.error('修复失败，请稍后再试');
        } finally {
            isGenerating = false;
        }
    }
    
    // ==================== 操作函数 ====================
    
    function handleNew() {
        currentStyleId = null;
        saveName = '';
        originalText = '';
        regexPattern = '';
        htmlCode = '';
        worldinfoKey = '';
        worldinfoContent = '';
        chatHistory = [];
        selectedElement = '';
        selectedTagName = '';
        editMode = false;
        isFirstGeneration = true;
    }
    
    function copyToClipboard(text: string, label: string) {
        navigator.clipboard.writeText(text);
        toast.success(`${label}已复制`);
    }
    
    function toggleEditMode() {
        editMode = !editMode;
        if (!editMode) {
            selectedElement = '';
            selectedTagName = '';
        }
    }

    // ==================== 插入到角色卡逻辑 ====================
    let insertDialogOpen = $state(false);
    
    async function handleInsertToCard(cardTarget: any) {
        if (!regexPattern && !htmlCode) {
             toast.error("无可用的生成内容");
             return;
        }

        const toastId = toast.loading("正在插入到角色卡...");
        try {
            const token = localStorage.getItem("auth_token");
            
            // 1. Fetch full card details
            const cardRes = await fetch(`${API_BASE}/api/cards/${cardTarget.id}`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {} 
            });
            if (!cardRes.ok) throw new Error("获取角色卡详情失败");
            const fullCard = await cardRes.json();
            
            // Parse data
            let cardData = fullCard.data;
            if (typeof cardData === 'string') {
                 try { cardData = JSON.parse(cardData); } catch {}
            }
            if (!cardData) cardData = {};
            
            // V2 structure check
            const v2Data = cardData.data || cardData;
            
            // --- 2. Prepare World Info ---
            let characterBook = v2Data.character_book || cardData.character_book;
            const currentExtensions = v2Data.extensions || {};
            let newExtensions = { ...currentExtensions };
            
            // Check if we need to create a new book
            if (!characterBook) {
                const bookName = `${fullCard.name}_世界书`;
                characterBook = { 
                    entries: [], 
                    name: bookName 
                };
                // CRITICAL: Set the World Name in extensions for V2 compatibility
                newExtensions.world = bookName;
            }
            
            // Normalize entries
            let entries: any[] = [];
            let isMap = false;
            if (Array.isArray(characterBook.entries)) {
                entries = [...characterBook.entries];
            } else if (characterBook.entries) {
                entries = Object.values(characterBook.entries);
                isMap = true;
            }
            
            const maxId = entries.reduce((max: number, e: any) => Math.max(max, Number(e.id || e.uid || 0)), 0);
            const newId = maxId + 1;
            
            // FULL SCHEMA conforming to WorldInfoTab.svelte defaults
            const newEntry = {
                id: newId,
                keys: [worldinfoKey],
                secondary_keys: [], // REQUIRED
                comment: worldinfoKey || "AI生成条目",
                content: worldinfoContent || "",
                constant: true, // User requirement
                selective: true, // Default
                insertion_order: 100,
                enabled: true,
                position: "before_char",
                use_regex: true,
                extensions: { 
                    position: 0,
                    exclude_recursion: false,
                    display_index: entries.length,
                    probability: 100,
                    useProbability: true,
                    depth: 4,
                    selectiveLogic: 0,
                    outlet_name: "",
                    group: "",
                    group_override: false,
                    group_weight: 100,
                    prevent_recursion: false,
                    delay_until_recursion: false,
                    scan_depth: null,
                    match_whole_words: null,
                    use_group_scoring: false,
                    case_sensitive: null,
                    automation_id: "",
                    role: 0, 
                    vectorized: false,
                    sticky: 0,
                    cooldown: 0,
                    delay: 0,
                    match_persona_description: false,
                    match_character_description: false,
                    match_character_personality: false,
                    match_character_depth_prompt: false,
                    match_scenario: false,
                    match_creator_notes: false,
                    triggers: [],
                    ignore_budget: false,
                }
            };
            
            if (Array.isArray(characterBook.entries)) {
                characterBook.entries.push(newEntry);
            } else {
                 characterBook.entries[String(newId)] = newEntry;
            }

            // --- 3. Prepare Regex Scripts ---
            // Note: we update `newExtensions.regex_scripts` AND send it.
            // But we must handle partial update logic carefully.
            // If we send `extensions` payload, it replaces EVERYTHING in DB extensions.
            // So we must include ALL existing extensions + new world name + new regex scripts.
            
            const existingScripts = Array.isArray(newExtensions.regex_scripts) ? [...newExtensions.regex_scripts] : [];
            
            const newScript = {
                id: crypto.randomUUID(),
                scriptName: worldinfoKey || "AI正则",
                findRegex: regexPattern,
                replaceString: htmlCode,
                trimStrings: [],
                placement: [2],
                disabled: false,
                markdownOnly: true,
                promptOnly: false,
                runOnEdit: true,
                substituteRegex: 0,
                minDepth: null,
                maxDepth: null
            };
            
            const updatedScripts = [...existingScripts, newScript];
            newExtensions.regex_scripts = updatedScripts;

            // --- 4. Send Payload ---
            // We use `extensions` key to update everything safely (World Name + Regex + Others)
            // We also send `character_book`
            
            const payload = {
                extensions: newExtensions, // Sends updated world name & regex scripts together
                character_book: characterBook
            };

            const updateRes = await fetch(`${API_BASE}/api/cards/${cardTarget.id}`, {
                 method: 'PATCH',
                 headers: {
                     'Content-Type': 'application/json',
                     ...(token ? { Authorization: `Bearer ${token}` } : {})
                 },
                 body: JSON.stringify(payload)
            });
             
            if (!updateRes.ok) throw new Error("更新角色卡失败");
             
            toast.success("已插入到角色卡");
            insertDialogOpen = false;

        } catch (e: any) {
            console.error(e);
            toast.error("插入失败: " + e.message);
        } finally {
            toast.dismiss(toastId);
        }
    }
    
    // 选中的元素标签名（用于显示 #tag）
    let selectedTagName = $state('');
    
    // 监听 iframe 消息（交互式编辑）
    function handleIframeMessage(event: MessageEvent) {
        if (event.data?.type === 'elementSelected') {
            const info = event.data.data;
            const tagLower = info.tagName.toLowerCase();
            selectedTagName = tagLower;
            
            // 构建更详细的元素描述供 AI 使用
            selectedElement = `元素类型: <${tagLower}>
属性: ${info.className ? `class="${info.className}"` : ''}${info.id ? ` id="${info.id}"` : ''}
完整 HTML:
${info.outerHTML}`;
            
            toast.success(`已选中 #${tagLower}`);
        } else if (event.data?.type === 'resize') {
            iframeHeight = Math.max(400, event.data.height + 20); // 最小高度 400，并添加缓冲
        }
    }
    
    // iframe 高度状态
    let iframeHeight = $state(600);
    
    // 初始化
    onMount(() => {
        if (localStorage.getItem('auth_token')) {
            loadStyleLibrary();
        }
        window.addEventListener('message', handleIframeMessage);
        return () => window.removeEventListener('message', handleIframeMessage);
    });
    function formatDate(dateStr: string) {
        if (!dateStr) return '';
        let d: Date;
        if (!dateStr.endsWith('Z') && !dateStr.includes('+')) {
            d = new Date(dateStr + 'Z');
        } else {
            d = new Date(dateStr);
        }
        
        if (isNaN(d.getTime())) return dateStr;
        return d.toLocaleString('zh-CN', {
            year: 'numeric', 
            month: '2-digit', 
            day: '2-digit', 
            hour: '2-digit', 
            minute: '2-digit', 
            second: '2-digit',
            hour12: false 
        });
    }
</script>

<div class="flex flex-col h-full">
    <!-- 顶栏 -->
    <!-- 顶栏 -->
    <div class="flex flex-col gap-3 px-4 py-4 border-b bg-card sm:flex-row sm:items-center sm:justify-between sm:px-6">
        <div>
            <h1 class="text-xl font-bold sm:text-2xl">皮皮美化工作台</h1>
            <p class="text-sm text-muted-foreground hidden sm:block">AI 驱动的前端样式生成系统，一次性生成样式、正则和世界书</p>
        </div>
        <div class="flex items-center gap-2 w-full sm:w-auto justify-between sm:justify-end">
            <Button variant="outline" size="sm" onclick={handleNew}>
                <Plus class="w-4 h-4 mr-1" />
                新建
            </Button>
            <Button variant="outline" size="sm" onclick={() => saveDialogOpen = true}>
                <Save class="w-4 h-4 mr-1" />
                保存
            </Button>
            <Button variant="outline" size="sm" onclick={() => { libraryOpen = true; loadStyleLibrary(); }}>
                <Library class="w-4 h-4 mr-1" />
                样式库
            </Button>
        </div>
    </div>
    
    <!-- 主内容区：两栏布局 -->
    <div class="flex flex-col lg:flex-row flex-1 overflow-y-auto lg:overflow-hidden">
        <!-- 左侧栏：控制台 -->
        <div class="w-full lg:w-[400px] min-h-[60vh] lg:h-full border-b lg:border-b-0 lg:border-r flex flex-col bg-muted/30 shrink-0">
            <!-- 标题 -->
            <div class="px-4 py-3 border-b flex items-center gap-2">
                <Sparkles class="w-4 h-4 text-primary" />
                <span class="font-medium">控制台</span>
                {#if isGenerating}
                    <Loader2 class="w-4 h-4 ml-auto animate-spin text-muted-foreground" />
                {/if}
            </div>
            
            <!-- 原始文本 -->
            <div class="p-4 border-b">
                <label for="original-text" class="text-sm font-medium mb-2 block">原始文本（可选）</label>
                <Textarea 
                    id="original-text"
                    bind:value={originalText}
                    placeholder="粘贴示例文本，如人物状态栏格式..."
                    class="min-h-[100px] resize-y text-sm font-mono"
                />
                <p class="text-xs text-muted-foreground mt-1">AI 会根据此文本设计正则和世界书格式</p>
                <p class="text-xs text-muted-foreground mt-1">注意，不会传递"聊天记录"，每次都是独立的生成或修改</p>
            </div>
            
            <!-- 对话区域 -->
            <div class="flex-1 flex flex-col overflow-hidden">
                <!-- 对话历史 -->
                <div class="flex-1 overflow-y-auto p-4 space-y-3">
                    {#if chatHistory.length === 0}
                        <div class="text-center text-muted-foreground text-sm py-8 space-y-2">
                            <Wand2 class="w-8 h-8 mx-auto opacity-50" />
                            <p>描述你想要的样式效果...</p>
                            <p class="text-xs">例如：赛博朋克风格的人物状态面板</p>
                        </div>
                    {:else}
                        {#each chatHistory as msg}
                            <div class={msg.role === 'user' ? 'text-right' : 'text-left'}>
                                <div class={`inline-block px-3 py-2 rounded-lg max-w-[90%] text-sm ${
                                    msg.role === 'user' 
                                        ? 'bg-primary/90 text-primary-foreground' 
                                        : 'bg-muted'
                                }`}>
                                    {#if msg.selectedTag}
                                        <code class="inline-block px-1.5 py-0.5 mr-1 rounded bg-primary-foreground/20 text-primary-foreground/80 font-mono text-xs">#{msg.selectedTag}</code>
                                    {/if}
                                    {msg.content}
                                </div>
                            </div>
                        {/each}
                    {/if}
                    
                    <!-- 生成进度消息 -->
                    {#if progressMessage}
                        <div class="text-left">
                            <div class="inline-flex items-center gap-2 px-3 py-2 rounded-lg bg-muted text-sm text-muted-foreground">
                                <Loader2 class="w-3 h-3 animate-spin" />
                                {progressMessage}
                            </div>
                        </div>
                    {/if}
                </div>
                
                <!-- 输入框 -->
                <div class="p-4 border-t">
                    <!-- 选中元素 badge -->
                    {#if selectedTagName}
                        <div class="mb-2 flex items-center gap-1">
                            <button 
                                class="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/80 transition-colors"
                                onclick={() => { selectedElement = ''; selectedTagName = ''; chatInput = chatInput.replace(new RegExp(`^#${selectedTagName}\\s*`), ''); }}
                                title="点击移除"
                            >
                                #{selectedTagName}
                                <X class="w-3 h-3" />
                            </button>
                            <span class="text-xs text-muted-foreground">针对此元素修改</span>
                        </div>
                    {/if}
                    <div class="flex gap-2 items-end">
                        <Textarea 
                            bind:value={chatInput}
                            placeholder={selectedTagName ? `描述对 #${selectedTagName} 的修改...` : "描述样式需求..."}
                            onkeydown={(e) => e.key === 'Enter' && !e.shiftKey && (e.preventDefault(), handleSendMessage())}
                            disabled={isGenerating}
                            rows={2}
                            class="flex-1 min-h-[52px] max-h-[200px] resize-none field-sizing-content"
                        />
                        <Button 
                            size="icon" 
                            onclick={handleSendMessage}
                            disabled={isGenerating || !chatInput.trim()}
                            class="h-10 w-10 shrink-0"
                        >
                            {#if isGenerating}
                                <Loader2 class="w-4 h-4 animate-spin" />
                            {:else}
                                <Send class="w-4 h-4" />
                            {/if}
                        </Button>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- 右侧栏：预览/AI输出 -->
        <div class="flex-1 w-full lg:w-auto h-auto lg:h-full shrink-0 flex flex-col overflow-visible lg:overflow-hidden">
            <Tabs.Root bind:value={activeTab} class="flex-1 flex flex-col">
                <!-- Tab 头部 -->
                <div class="flex items-center justify-between px-4 py-2 border-b">
                    <Tabs.List>
                        <Tabs.Trigger value="preview" class="flex items-center gap-1">
                            <Eye class="w-4 h-4" />
                            预览
                        </Tabs.Trigger>
                        <Tabs.Trigger value="output" class="flex items-center gap-1">
                            <Code class="w-4 h-4" />
                            AI输出
                        </Tabs.Trigger>
                    </Tabs.List>
                    
                    <div class="flex items-center gap-2">
                        {#if activeTab === 'preview' && htmlCode.trim()}
                            <Button 
                                variant={editMode ? "default" : "outline"} 
                                size="sm"
                                onclick={toggleEditMode}
                            >
                                <MousePointer class="w-4 h-4 mr-1" />
                                {editMode ? '退出修改' : '点击修改'}
                            </Button>
                        {/if}
                    </div>
                </div>
                
                <!-- Tab 内容 -->
                <Tabs.Content value="preview" class="flex-1 overflow-visible lg:overflow-hidden m-0 p-0 relative min-h-[500px]">
                    {#if htmlCode.trim()}
                        <!-- 渲染模式切换浮窗 -->
                        <div class="absolute top-3 right-3 z-10 backdrop-blur-md bg-background/30 border rounded-lg shadow-lg p-1 flex gap-1">
                            <button 
                                class={`px-2.5 py-1 text-xs rounded transition-colors ${renderMode === 'code' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'}`}
                                onclick={() => renderMode = 'code'}
                            >
                                仅预览
                            </button>
                            <button 
                                class={`px-2.5 py-1 text-xs rounded transition-colors ${renderMode === 'full' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'}`}
                                onclick={() => renderMode = 'full'}
                                disabled={!originalText.trim() || !regexPattern.trim()}
                                title={!originalText.trim() || !regexPattern.trim() ? '需要原始文本和正则表达式' : '应用正则替换'}
                            >
                                实际应用
                            </button>
                        </div>
                        
                        {#if editMode && renderMode === 'code'}
                            <div class="absolute bottom-3 right-3 z-10 bg-primary/90 text-primary-foreground text-xs px-2.5 py-1 rounded shadow-lg pointer-events-none opacity-80">
                                🖱️ 点击选中元素
                            </div>
                        {/if}
                        
                        {#if renderMode === 'full' && fullContentHtml === 'REGEX_MISMATCH_ERROR' && !isGenerating}
                            <div class="absolute bottom-6 left-1/2 -translate-x-1/2 z-30">
                                <Button onclick={fixRegex} variant="destructive" class="shadow-lg animate-bounce">
                                    <Wrench class="w-4 h-4 mr-2" />
                                    一键修复正则与格式
                                </Button>
                            </div>
                        {/if}
                        <iframe 
                            bind:this={previewIframe}
                            srcdoc={finalPreviewSrcDoc}
                            class="w-full lg:!h-full border-0 transition-all duration-300"
                            style="height: {iframeHeight}px"
                            title="预览"
                            sandbox="allow-scripts"
                        ></iframe>
                    {:else}
                        <div class="flex flex-col items-center justify-center h-full text-muted-foreground">
                            <Layers class="w-12 h-12 mb-4 opacity-50" />
                            <p>暂无生成内容</p>
                            <p class="text-sm">在左侧输入需求开始生成</p>
                        </div>
                    {/if}
                </Tabs.Content>
                
                <Tabs.Content value="output" class="flex-1 overflow-y-auto m-0 p-4 space-y-4">

                    <!-- 正则 -->
                    <div>
                        <div class="flex items-center justify-between mb-2">
                            <label for="regex-pattern" class="text-sm font-medium">正则表达式</label>
                            <Button variant="ghost" size="icon" class="h-6 w-6" onclick={() => copyToClipboard(regexPattern, '正则')}>
                                <Copy class="w-3 h-3" />
                            </Button>
                        </div>
                        <Textarea 
                            id="regex-pattern"
                            bind:value={regexPattern}
                            placeholder="正则表达式..."
                            class="min-h-[80px] font-mono text-sm"
                        />
                    </div>
                    
                    <!-- 样式代码 -->
                    <div>
                        <div class="flex items-center justify-between mb-2">
                            <label for="html-code" class="text-sm font-medium">替换为（HTML/CSS/JS）</label>
                            <Button variant="ghost" size="icon" class="h-6 w-6" onclick={() => copyToClipboard(htmlCode, '样式代码')}>
                                <Copy class="w-3 h-3" />
                            </Button>
                        </div>
                        <Textarea 
                            id="html-code"
                            bind:value={htmlCode}
                            placeholder="HTML/CSS/JS 代码..."
                            class="min-h-[200px] font-mono text-sm"
                        />
                    </div>
                    
                    <!-- 世界书条目 -->
                    <div>
                        <div class="flex items-center justify-between mb-2">
                            <label for="worldinfo-key" class="text-sm font-medium">世界书条目</label>
                            <Button variant="ghost" size="icon" class="h-6 w-6" onclick={() => copyToClipboard(`${worldinfoKey}\n\n${worldinfoContent}`, '世界书条目')}>
                                <Copy class="w-3 h-3" />
                            </Button>
                        </div>
                        <Input 
                            id="worldinfo-key"
                            bind:value={worldinfoKey}
                            placeholder="触发关键词 / 条目名称"
                            class="mb-2"
                        />
                        <Textarea 
                            bind:value={worldinfoContent}
                            placeholder="条目内容（AI 输出格式指令）..."
                            class="min-h-[150px]"
                        />
                    </div>

                    <div class="flex justify-end mt-4 pt-4 border-t">
                         <Button variant="default" size="sm" class="gap-2" onclick={() => insertDialogOpen = true} disabled={!regexPattern && !htmlCode}>
                            <IdCard class="w-4 h-4" />
                            插入到角色卡...
                        </Button>
                    </div>
                </Tabs.Content>
            </Tabs.Root>
        </div>
    </div>
</div>

<!-- 保存对话框 -->
<Dialog.Root bind:open={saveDialogOpen}>
    <Dialog.Content>
        <Dialog.Header>
            <Dialog.Title>保存样式</Dialog.Title>
            <Dialog.Description>为当前样式命名以便后续使用</Dialog.Description>
        </Dialog.Header>
        <div class="py-4">
            <Input 
                bind:value={saveName}
                placeholder="输入样式名称..."
                onkeydown={(e) => e.key === 'Enter' && saveStyle()}
            />
        </div>
        <Dialog.Footer>
            <Button variant="outline" onclick={() => saveDialogOpen = false}>取消</Button>
            <Button onclick={saveStyle}>保存</Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>

<InsertToCardDialog 
    bind:open={insertDialogOpen}
    onConfirm={handleInsertToCard}
/>


    <!-- 样式库 Sheet -->
    <Sheet.Root bind:open={libraryOpen}>
        <Sheet.Content side="right" class="w-[70%] sm:w-[400px] flex flex-col p-0 gap-0">
            <Sheet.Header class="px-6 py-4 border-b">
                <Sheet.Title>样式库</Sheet.Title>
                <Sheet.Description>管理您保存的样式预设</Sheet.Description>
            </Sheet.Header>
            
            <div class="flex-1 overflow-y-auto px-6 py-6">
                {#if styleLibrary.length === 0}
                    <div class="flex flex-col items-center justify-center h-40 text-muted-foreground border-2 border-dashed rounded-lg">
                        <Library class="w-8 h-8 mb-2 opacity-20" />
                        <span>暂无保存的样式</span>
                    </div>
                {:else}
                    <div class="space-y-3">
                    {#each styleLibrary as style}
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <div class="relative flex flex-col p-4 rounded-xl border bg-card text-card-foreground shadow-sm transition-all hover:shadow-md hover:border-primary/50 group cursor-pointer"
                             onclick={() => loadStyle(style.id)}
                             role="button"
                             tabindex="0">
                            
                            <div class="flex items-start justify-between mb-2">
                                <div class="flex items-center gap-2">
                                    <div class="p-1.5 rounded-md bg-primary/10 text-primary">
                                        <Sparkles class="w-4 h-4" />
                                    </div>
                                    <span class="font-semibold">{style.name}</span>
                                </div>
                                <Button 
                                    variant="ghost" 
                                    size="icon" 
                                    class="h-7 w-7 text-muted-foreground hover:text-destructive hover:bg-destructive/10 -mr-2 -mt-2 opacity-0 group-hover:opacity-100 transition-opacity"
                                    onclick={(e) => { e.stopPropagation(); deleteStyle(style.id); }}
                                >
                                    <Trash2 class="w-4 h-4" />
                                </Button>
                            </div>
                            
                            <div class="flex items-center text-xs text-muted-foreground mt-1">
                                <span class="bg-muted px-1.5 py-0.5 rounded text-[10px] mr-2">更新于</span>
                                {formatDate(style.updated_at)}
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    </Sheet.Content>
</Sheet.Root>

<style>
    /* 确保 iframe 在编辑模式下可交互 */
    iframe {
        pointer-events: auto;
    }
</style>