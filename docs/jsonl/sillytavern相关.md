# 结论总览
- 展示链路的“纯文本 -> 可插入 DOM 的 HTML 字符串”的核心函数是 `messageFormatting()`，最终通过 `.html(...)`/`.append(...)`/`innerHTML = ...` 写入 `.mes_text`。
- 展示侧默认会走 Markdown（Showdown：`converter.makeHtml`），但 `power_user.encode_tags` 打开时会先把 `<`/`>` 转义成 `&lt;`/`&gt;`，从而把“原始 HTML”降级为纯文本。
- 展示侧 HTML 安全边界主要由 `DOMPurify.sanitize(mes, { MESSAGE_SANITIZE:true, ADD_TAGS:['custom-style'], ... })` + `addDOMPurifyHooks()` 提供；其中对 `class` 做 `custom-` 前缀化，对 `target` 强制 `rel=noopener`。
- `<style>` 并非直接放行：先 `encodeStyleTags()` 把 `<style>...</style>` 变成 `<custom-style>...`（URL 编码），sanitize 后再 `decodeStyleTags(..., { prefix: '.mes_text ' })` 解码并对选择器做前缀化、过滤 `@import`，从而避免全局 CSS 污染。
- 展示侧 Regex 扩展通过 `getRegexedString(..., { isMarkdown:true, depth })` 在 Markdown 解析前执行；Prompt 构建侧通过 `getRegexedString(..., { isPrompt:true, depth })` 执行，二者由 `script.markdownOnly/promptOnly` 分流。
- Prompt 构建链路不会走 Markdown 渲染（不会产出 HTML），而是对 `chatItem.mes` 等纯文本做清洗（`cleanUpMessage`）、regex、宏替换（用户输入/偏置等），再组装成发送给模型的字符串/消息结构。
- 代码块 DOM 结构由 Showdown 生成的 `<pre><code>...</code></pre>` 提供；渲染后会执行 `hljs.highlightElement(code)` 包装 `<span>`，并往 `code` 里 `appendChild(copyButton)` 注入复制按钮，这会改变 `textContent`。
- 流式输出用 `StreamingProcessor`，每个 token 先 `messageFormatting(processedText, ...)`，再 `this.messageTextDom.innerHTML = formattedText`（或 `applyStreamFadeIn` + `morphdom`）；结束时补齐 `addCopyToCodeBlocks`/reasoning/media。
- 编辑/重排会触发重渲染：`messageEditCancel/messageEditDone` 会清空 `.mes_text` 后重新插入 `messageFormatting(...)` 结果并再跑后处理；`messageEditMove` 直接 `insertAfter/insertBefore` 重排 DOM 并交换 `mesid`。
- 另一个会影响展示结果的 DOM 后处理来自 `MutationObserver`：对 `.mes_text` 内的 `<math>` 把子 TextNode `textContent=''`（用于公式渲染兼容）。
- 未发现“把任意用户/模型输入绕过 DOMPurify 直接写入 `.mes_text` 的路径”；已检索 `.mes_text .html/append/innerHTML`，唯一不经 sanitize 的写入是常量占位 `'.mes_text'.html('...')`。

# 调用链总图

display（写入聊天界面）

```
chat[]/message.extra.display_text
  -> addOneMessage(...) / updateMessageBlock(...) / StreamingProcessor.onProgressStreaming(...)
    -> messageFormatting(mes, ch_name, isSystem, isUser, messageId, sanitizerOverrides, isReasoning)
      -> (display regex) getRegexedString(..., { isMarkdown:true, depth })
      -> (markdown) converter.makeHtml(...)
      -> (style sandbox) encodeStyleTags -> DOMPurify.sanitize(config{MESSAGE_SANITIZE,ADD_TAGS:['custom-style'],...}) -> decodeStyleTags(prefix:'.mes_text ')
      -> return HTML string
    -> 写 DOM: .mes_text.html(...) / .mes_text.append(...) / mes_text.innerHTML = ...
    -> DOM 后处理: updateReasoningUI -> appendMediaToMessage -> addCopyToCodeBlocks(hljs+copyButton) -> (可选) applyStreamFadeIn(morphdom)
```

prompt（发送给模型）

```
Generate(...)
  -> cleanUpMessage(getMessage, ...)  // 停止词裁剪 + regex(AI_OUTPUT/USER_INPUT) + auto_fix_generated_markdown 等
  -> (prompt regex) coreChat.map(...) -> getRegexedString(chatItem.mes, placement, { isPrompt:true, depth })
  -> (可选) appendFileContent(...) / reasoning 注入(getRegexedString(..., {isPrompt:true, depth}))
  -> 组装为最终 prompt 文本/消息数组
  -> 发送到后端 API
```

# STEP 1：定位“消息渲染写入 DOM”的入口（必须找全）

## 证据列表（路径 + 行号范围 + 原文摘录）

### 1) 初次加载/切换聊天：`reloadCurrentChat()`（清空 DOM -> `printMessages()`）

证据：`public/script.js:1572-1592`

```js
export async function reloadCurrentChat() {
    preserveNeutralChat();
    await clearChat();
    chat.length = 0;

    if (selected_group) {
        await getGroupChat(selected_group, true);
    }
    else if (this_chid !== undefined) {
        await getChat();
    }
    else {
        resetChatState();
        restoreNeutralChat();
        await getCharacters();
        await printMessages();
        await eventSource.emit(event_types.CHAT_CHANGED, getCurrentChatId());
    }

    refreshSwipeButtons();
}
```

### 2) 清空聊天 DOM：`clearChat()`（删除所有子节点，包括历史加载按钮）

证据：`public/script.js:1485-1502`

```js
export async function clearChat() {
    cancelDebouncedChatSave();
    cancelDebouncedMetadataSave();
    closeMessageEditor();
    extension_prompts = {};
    if (is_delete_mode) {
        $('#dialogue_del_mes_cancel').trigger('click');
    }
    //This will also remove non '.mes' elements, e.g. '<div id="show_more_messages">Show more messages</div>'.
    chatElement.children().remove();
    if ($('.zoomed_avatar[forChar]').length) {
        console.debug('saw avatars to remove');
        $('.zoomed_avatar[forChar]').remove();
    } else { console.debug('saw no avatars'); }

    await saveItemizedPrompts(getCurrentChatId());
    itemizedPrompts.length = 0;
}
```

### 3) 初次整屏渲染：`printMessages()`（循环 `addOneMessage`，并可能插入“show more”占位 DOM）

证据：`public/script.js:1416-1436`

```js
export async function printMessages() {
    let startIndex = 0;
    let count = power_user.chat_truncation || Number.MAX_SAFE_INTEGER;

    if (chat.length > count) {
        startIndex = chat.length - count;
        chatElement.append('<div id="show_more_messages">Show more messages</div>');
    }

    for (let i = startIndex; i < chat.length; i++) {
        const item = chat[i];
        addOneMessage(item, { scroll: false, forceId: i, showSwipes: false });
    }

    chatElement.find('.mes').removeClass('last_mes');
    chatElement.find('.mes').last().addClass('last_mes');
    refreshSwipeButtons(false, false);
    applyStylePins();
    scrollChatToBottom({ waitForFrame: true });
    delay(debounce_timeout.short).then(() => scrollOnMediaLoad());
}
```

### 4) 历史向上加载：`showMoreMessages()`（`insertBefore` 插入并复用 `addOneMessage`）

证据：`public/script.js:1380-1414`

```js
while (messageId > 0 && count > 0) {
    let newMessageId = messageId - 1;
    addOneMessage(chat[newMessageId], { insertBefore: messageId >= chat.length ? null : messageId, scroll: false, forceId: newMessageId, showSwipes: false });
    count--;
    messageId--;
}
```

### 5) 新增单条消息（核心入口）：`addOneMessage()`

证据：`public/script.js:2398-2562`

```js
// if mes.extra.uses_system_ui is true, set an override on the sanitizer options
const sanitizerOverrides = mes.extra?.uses_system_ui ? { MESSAGE_ALLOW_SYSTEM_UI: true } : {};

messageText = messageFormatting(
    messageText,
    mes.name,
    isSystem,
    mes.is_user,
    chat.indexOf(mes),
    sanitizerOverrides,
    false,
);
const bias = messageFormatting(mes.extra?.bias ?? '', '', false, false, -1, {}, false);

const renderedMessage = getMessageFromTemplate(params);

if (type !== 'swipe') {
    if (!insertAfter && !insertBefore) {
        chatElement.append(renderedMessage);
    }
    else if (insertAfter) {
        const target = chatElement.find(`.mes[mesid="${insertAfter}"]`);
        $(renderedMessage).insertAfter(target);
    } else {
        const target = chatElement.find(`.mes[mesid="${insertBefore}"]`);
        $(renderedMessage).insertBefore(target);
    }
}

// Callers push the new message to chat before calling addOneMessage
const newMessageId = typeof forceId == 'number' ? forceId : chat.length - 1;
const newMessage = chatElement.find(`[mesid="${newMessageId}"]`);

if (type === 'swipe') {
    const swipeMessage = chatElement.find(`[mesid="${newMessageId}"]`);
    swipeMessage.attr('swipeid', params.swipeId);
    swipeMessage.find('.mes_text').html(messageText).attr('title', title);
    swipeMessage.find('.timestamp').text(timestamp).attr('title', `${params.extra.api} - ${params.extra.model}`);
    updateReasoningUI(swipeMessage);
    appendMediaToMessage(mes, swipeMessage, scroll ? SCROLL_BEHAVIOR.ADJUST : SCROLL_BEHAVIOR.NONE);
    if (power_user.timestamp_model_icon && params.extra?.api) {
        insertSVGIcon(swipeMessage, params.extra);
    }
    // ...
} else {
    chatElement.find(`[mesid="${newMessageId}"] .mes_text`).append(messageText);
    appendMediaToMessage(mes, newMessage, scroll ? SCROLL_BEHAVIOR.ADJUST : SCROLL_BEHAVIOR.NONE);
}

addCopyToCodeBlocks(newMessage);
```

### 6) 单条消息更新/重渲染：`updateMessageBlock()`（直接写 `.mes_text`.html(...)）

证据：`public/script.js:1920-1931`

```js
export function updateMessageBlock(messageId, message, { rerenderMessage = true } = {}) {
    const messageElement = chatElement.find(`[mesid="${messageId}"]`);
    if (rerenderMessage) {
        const text = message?.extra?.display_text ?? message.mes;
        messageElement.find('.mes_text').html(messageFormatting(text, message.name, message.is_system, message.is_user, messageId, {}, false));
    }

    updateReasoningUI(messageElement);

    addCopyToCodeBlocks(messageElement);
    appendMediaToMessage(message, messageElement);
}
```

### 7) 流式输出：`StreamingProcessor.onProgressStreaming()`（`innerHTML = formattedText`）

证据：`public/script.js:3409-3414`（缓存 `.mes_text` 原生节点）

```js
this.messageDom = document.querySelector(`#chat .mes[mesid="${messageId}"]`);
this.messageTextDom = this.messageDom?.querySelector('.mes_text');
```

证据：`public/script.js:3531-3545`（每 token 写 DOM）

```js
const formattedText = messageFormatting(
    processedText,
    chat[messageId].name,
    chat[messageId].is_system,
    chat[messageId].is_user,
    messageId,
    {},
    false,
);
if (this.messageTextDom instanceof HTMLElement) {
    if (power_user.stream_fade_in) {
        applyStreamFadeIn(this.messageTextDom, formattedText);
    } else {
        this.messageTextDom.innerHTML = formattedText;
    }
}
```

证据：`public/script.js:3562-3569`（结束时补齐后处理）

```js
async onFinishStreaming(messageId, text) {
    await this.onProgressStreaming(messageId, text, true);
    const messageElement = chatElement.find(`.mes[mesid="${messageId}"]`);
    const message = chat[messageId];
    addCopyToCodeBlocks(messageElement);

    await this.reasoningHandler.finish(messageId);
    // ...
}
```

### 8) 流式 fade-in 的 DOM 写入：`applyStreamFadeIn()`（clone + `innerHTML` + `morphdom`）

证据：`public/scripts/util/stream-fadein.js:17-19,65-69`

```js
export function segmentTextInElement(htmlElement, htmlContent, granularity = 'word') {
    htmlElement.innerHTML = htmlContent;
    // ... skip pre/code ...
}

export function applyStreamFadeIn(messageTextElement, htmlContent) {
    const targetElement = /** @type {HTMLElement} */ (messageTextElement.cloneNode());
    segmentTextInElement(targetElement, htmlContent);
    morphdom(messageTextElement, targetElement);
}
```

### 9) 编辑取消：清空 `.mes_text` 后重新插入格式化 HTML

证据：`public/script.js:7992-8017`

```js
thisMesBlock.find('.mes_text').empty();
thisMesBlock.find('.mes_text')
    .append(messageFormatting(
        text,
        this_edit_mes_chname,
        chat[messageId].is_system,
        chat[messageId].is_user,
        messageId,
        {},
        false,
    ));
appendMediaToMessage(chat[messageId], thisMesDiv);
addCopyToCodeBlocks(thisMesDiv);
```

### 10) 编辑确认：清空 `.mes_text` 后重新插入；并重渲染 `.mes_bias`

证据：`public/script.js:8098-8116`

```js
mesBlock.find('.mes_text').empty();
mesBlock.find('.mes_text').append(
    messageFormatting(
        text,
        this_edit_mes_chname,
        mes.is_system,
        mes.is_user,
        this_edit_mes_id,
        {},
        false,
    ),
);
mesBlock.find('.mes_bias').empty();
mesBlock.find('.mes_bias').append(messageFormatting(bias, '', false, false, -1, {}, false));
appendMediaToMessage(mes, div.closest('.mes'));
addCopyToCodeBlocks(div.closest('.mes'));
```

### 11) swipe “生成中占位”：不经 `messageFormatting` 的常量写入

证据：`public/script.js:9947-9956`

```js
if (run_generate) {
    thisMesDiv.find('.mes_text').html('...');
    thisMesDiv.find('.mes_timer').html('');
    updateReasoningUI(thisMesDiv, { reset: true });
    // ...
}
```

### 12) 模板阶段也会写 DOM（但不是 `.mes_text`）：`getMessageFromTemplate()` 写 `.mes_bias`，并触发 reasoning UI

证据：`public/script.js:1881-1910`

```js
mes.find('.avatar img').attr('src', avatarImg);
mes.find('.ch_name .name_text').text(characterName);
mes.find('.mes_bias').html(bias);
mes.find('.timestamp').text(timestamp).attr('title', `${extra?.api ? extra.api + ' - ' : ''}${extra?.model ?? ''}`);
// ...
updateReasoningUI(mes);
```

## 解释

- SillyTavern 的“消息写 DOM”有两类：
  - 写入消息容器/顺序：`chatElement.append(renderedMessage)` / `insertBefore/insertAfter` / `insertAfter/insertBefore`（编辑移动）。
  - 写入消息内容：`.mes_text` 的 `.append` / `.html` 或 `innerHTML = ...`。
- 展示侧内容写入几乎都来自 `messageFormatting()` 的返回值；唯一例外是 swipe 过程中的常量占位 `'...'`。
- 不同入口（初次加载/加载更多/追加/更新/流式/编辑/Swipe）最终都落在同一套“格式化 + sanitize + 写入 + 后处理”模式上，只是写入方式不同（jQuery vs 原生 `innerHTML`）。

## 复刻要点

- 必须把“写 DOM 的入口”拆成：初次渲染（批量 addOneMessage）、单条更新（覆盖 `.mes_text`）、流式更新（增量覆盖 `.mes_text`）、编辑重渲染（清空后重插入）。
- 必须复刻 `.mes_text` 的三种写入方式语义：append（累加）、html/innerHTML（覆盖）。
- 必须在每次“写入最终 HTML”后跑同一组后处理（至少：reasoning UI、媒体注入、代码高亮与复制按钮）。

# STEP 2：还原 messageFormatting（或等价函数）的完整行为

## 证据列表（路径 + 行号范围 + 原文摘录）

### 1) 函数签名与主要分支

证据：`public/script.js:1644-1803`

```js
export function messageFormatting(mes, ch_name, isSystem, isUser, messageId, sanitizerOverrides = {}, isReasoning = false) {
    if (!mes) {
        return '';
    }

    if (Number(messageId) === 0 && !isSystem && !isUser && !isReasoning) {
        const mesBeforeReplace = mes;
        const chatMessage = chat[messageId];
        mes = substituteParams(mes, undefined, ch_name);
        if (chatMessage && chatMessage.mes === mesBeforeReplace && chatMessage.extra?.display_text !== mesBeforeReplace) {
            chatMessage.mes = mes;
        }
    }

    mesForShowdownParse = mes;

    // Force isSystem = false on comment messages so they get formatted properly
    if (ch_name === COMMENT_NAME_DEFAULT && isSystem && !isUser) {
        isSystem = false;
    }

    // Let hidden messages have markdown
    if (isSystem && ch_name !== systemUserName) {
        isSystem = false;
    }

    // Prompt bias replacement should be applied on the raw message
    const replacedPromptBias = power_user.user_prompt_bias && substituteParams(power_user.user_prompt_bias);
    if (!power_user.show_user_prompt_bias && ch_name && !isUser && !isSystem && replacedPromptBias && mes.startsWith(replacedPromptBias)) {
        mes = mes.slice(replacedPromptBias.length);
    }

    if (!isSystem) {
        function getRegexPlacement() {
            try {
                if (isReasoning) {
                    return regex_placement.REASONING;
                }
                if (isUser) {
                    return regex_placement.USER_INPUT;
                } else if (chat[messageId]?.extra?.type === 'narrator') {
                    return regex_placement.SLASH_COMMAND;
                } else {
                    return regex_placement.AI_OUTPUT;
                }
            } catch {
                return regex_placement.AI_OUTPUT;
            }
        }

        const regexPlacement = getRegexPlacement();
        const usableMessages = chat.map((x, index) => ({ message: x, index: index })).filter(x => !x.message.is_system);
        const indexOf = usableMessages.findIndex(x => x.index === Number(messageId));
        const depth = messageId >= 0 && indexOf !== -1 ? (usableMessages.length - indexOf - 1) : undefined;

        // Always override the character name
        mes = getRegexedString(mes, regexPlacement, {
            characterOverride: ch_name,
            isMarkdown: true,
            depth: depth,
        });
    }

    if (power_user.auto_fix_generated_markdown) {
        mes = fixMarkdown(mes, true);
    }

    if (!isSystem && power_user.encode_tags) {
        mes = canUseNegativeLookbehind()
            ? mes.replaceAll('<', '&lt;').replace(new RegExp('(?<!^|\\n\\s*)>', 'g'), '&gt;')
            : mes.replaceAll('<', '&lt;').replaceAll('>', '&gt;');
    }
    // ...（后续：引号包装/markdown/sanitize/return）
}
```

### 2) Markdown 引擎（Showdown）在哪里初始化、有哪些配置项/扩展

证据：`public/script.js:489-507`

```js
export function reloadMarkdownProcessor() {
    converter = new showdown.Converter({
        emoji: true,
        literalMidWordUnderscores: true,
        parseImgDimensions: true,
        tables: true,
        underline: true,
        simpleLineBreaks: true,
        strikethrough: true,
        disableForced4SpacesIndentedSublists: true,
        extensions: [markdownUnderscoreExt()],
    });

    // Inject the dinkus extension after creating the converter
    // Maybe move this into power_user init?
    converter.addExtension(markdownExclusionExt(), 'exclusion');

    return converter;
}
```

证据：`public/script.js:662-705`（初始化顺序：patch + hooks + reloadMarkdownProcessor）

```js
addShowdownPatch(showdown);
addDOMPurifyHooks();
reloadMarkdownProcessor();
// ...
initMacros();
```

### 3) Markdown 扩展：underscore / exclusion

证据：`public/scripts/showdown-underscore.js:9-23`

```js
regex: new RegExp('(<code(?:\\s+[^>]*)?>[\\s\\S]*?<\\/code>|<style(?:\\s+[^>]*)?>[\\s\\S]*?<\\/style>)|\\b(?<!_)_(?!_)(.*?)(?<!_)_(?!_)\\b', 'gi'),
replace: function(match, tagContent, italicContent) {
    if (tagContent) {
        return match;
    } else if (italicContent) {
        return '<em>' + italicContent + '</em>';
    }
    return match;
},
```

证据：`public/scripts/showdown-exclusion.js:14-38`

```js
if (!power_user.markdown_escape_strings) {
    return [];
}
return [{
    type: 'lang',
    filter: (text) => {
        const escapedExclusions = substituteParams(power_user.markdown_escape_strings)
            .split(',')
            .filter((element) => element.length > 0)
            .map((element) => `(${element.split('').map((char) => `\\${char}`).join('')})`);
        if (escapedExclusions.length === 0) {
            return text;
        }
        const replaceRegex = new RegExp(`^(${escapedExclusions.join('|')})\\n`, 'gm');
        return text.replace(replaceRegex, ((match) => match.replace(replaceRegex, `\\u0000${match} \\n`)));
    },
}];
```

### 4) HTML / encode_tags 的处理（允许原始 HTML vs 转义为文本）

证据：`public/script.js:1711-1715`

```js
if (!isSystem && power_user.encode_tags) {
    mes = canUseNegativeLookbehind()
        ? mes.replaceAll('<', '&lt;').replace(new RegExp('(?<!^|\\n\\s*)>', 'g'), '&gt;')
        : mes.replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}
```

证据：`public/script.js:1730-1734,1764-1767`（当允许 HTML 时，先“保护标签内引号”，后恢复）

```js
if (!power_user.encode_tags) {
    mes = mes.replace(/<([^>]+)>/g, function (_, contents) {
        return '<' + contents.replace(/"/g, '\ufffe') + '>';
    });
}
// ...
if (!power_user.encode_tags) {
    mes = mes.replace(/\ufffe/g, '"');
}
```

### 5) Markdown 解析、code block newline 修正

证据：`public/script.js:1769-1782`

```js
mes = mes.replaceAll('\\begin{align*}', '$$');
mes = mes.replaceAll('\\end{align*}', '$$');
mes = converter.makeHtml(mes);

mes = mes.replace(/<code(.*)>[\s\S]*?<\/code>/g, function (match) {
    // Firefox creates extra newlines from <br>s in code blocks, so we replace them before converting newlines to <br>s.
    return match.replace(/\n/gm, '\u0000');
});
mes = mes.replace(/\u0000/g, '\n'); // Restore converted newlines
mes = mes.trim();

mes = mes.replace(/<code(.*)>[\s\S]*?<\/code>/g, function (match) {
    return match.replace(/&amp;/g, '&');
});
```

### 6) DOMPurify sanitize + custom-style（允许标签/配置/覆盖项）

证据：`public/script.js:1789-1801`

```js
/** @type {import('dompurify').Config & { RETURN_DOM_FRAGMENT: false; RETURN_DOM: false }} */
const config = {
    RETURN_DOM: false,
    RETURN_DOM_FRAGMENT: false,
    RETURN_TRUSTED_TYPE: false,
    MESSAGE_SANITIZE: true,
    ADD_TAGS: ['custom-style'],
    ...sanitizerOverrides,
};
mes = encodeStyleTags(mes);
mes = DOMPurify.sanitize(mes, config);
mes = decodeStyleTags(mes, { prefix: '.mes_text ' });
```

证据：`public/script.js:2440-2441`（sanitizerOverrides 的来源：`uses_system_ui`）

```js
const sanitizerOverrides = mes.extra?.uses_system_ui ? { MESSAGE_ALLOW_SYSTEM_UI: true } : {};
```

### 7) DOMPurify hooks（为什么能影响消息渲染）

证据：`public/scripts/chats.js:1904-2055`

```js
export function addDOMPurifyHooks() {
    // Allow target="_blank" in links
    DOMPurify.addHook('afterSanitizeAttributes', function (node) {
        if ('target' in node) {
            node.setAttribute('target', '_blank');
            node.setAttribute('rel', 'noopener');
        }
    });

    DOMPurify.addHook('uponSanitizeAttribute', (node, data, config) => {
        if (!config['MESSAGE_SANITIZE']) {
            return;
        }

        /* Retain the classes on UI elements of messages that interact with the main UI */
        const permittedNodeTypes = ['BUTTON', 'DIV'];
        if (config['MESSAGE_ALLOW_SYSTEM_UI'] && node.classList.contains('menu_button') && permittedNodeTypes.includes(node.nodeName)) {
            return;
        }

        switch (data.attrName) {
            case 'class': {
                if (data.attrValue) {
                    data.attrValue = data.attrValue.split(' ').map((v) => {
                        if (v.startsWith('fa-') || v.startsWith('note-') || v === 'monospace') {
                            return v;
                        }

                        return 'custom-' + v;
                    }).join(' ');
                }
                break;
            }
        }
    });

    DOMPurify.addHook('uponSanitizeElement', (node, _, config) => {
        if (!config['MESSAGE_SANITIZE']) {
            return;
        }

        // Replace line breaks with <br> in unknown elements
        if (node instanceof HTMLUnknownElement) {
            node.innerHTML = node.innerHTML.trim();
            // ... TreeWalker: \n -> <br>, skip <pre> ...
        }

        const isMediaAllowed = isExternalMediaAllowed();
        if (isMediaAllowed) {
            return;
        }
        // ... block external src/srcset/data for IMG/VIDEO/AUDIO/etc, node.remove() ...
    });
}
```

### 8) custom-style 的编码/解码与“选择器沙箱化”

证据：`public/scripts/chats.js:536-626`

```js
export function encodeStyleTags(text) {
    const styleRegex = /<style>(.+?)<\/style>/gims;
    return text.replaceAll(styleRegex, (_, match) => {
        return `<custom-style>${encodeURIComponent(match)}</custom-style>`;
    });
}

export function decodeStyleTags(text, { prefix } = { prefix: '.mes_text ' }) {
    const styleDecodeRegex = /<custom-style>(.+?)<\/custom-style>/gms;
    const mediaAllowed = isExternalMediaAllowed();

    function sanitizeRule(rule) {
        if (Array.isArray(rule.selectors)) {
            for (let i = 0; i < rule.selectors.length; i++) {
                const selector = rule.selectors[i];
                if (selector) {
                    rule.selectors[i] = prefix + sanitizeSelector(selector);
                }
            }
        }
        if (!mediaAllowed && Array.isArray(rule.declarations) && rule.declarations.length > 0) {
            rule.declarations = rule.declarations.filter(declaration => !declaration.value.includes('://'));
        }
    }

    function sanitizeSimpleSelector(selector) {
        // Split by spaces but preserve complex selectors
        return selector.split(/\s+/).map((part) => {
            // Handle class selectors, but preserve pseudo-classes and other complex parts
            return part.replace(/\.([\w-]+)/g, (match, className) => {
                // Don't modify if it's already prefixed with 'custom-'
                if (className.startsWith('custom-')) {
                    return match;
                }
                return `.custom-${className}`;
            });
        }).join(' ');
    }

    function sanitizeRuleSet(ruleSet) {
        if (Array.isArray(ruleSet.selectors) || Array.isArray(ruleSet.declarations)) {
            sanitizeRule(ruleSet);
        }

        if (Array.isArray(ruleSet.rules)) {
            ruleSet.rules = ruleSet.rules.filter(rule => rule.type !== 'import');

            for (const mediaRule of ruleSet.rules) {
                sanitizeRuleSet(mediaRule);
            }
        }
    }

    return text.replaceAll(styleDecodeRegex, (_, style) => {
        try {
            let styleCleaned = decodeURIComponent(style).replaceAll(/<br\/>/g, '');
            const ast = css.parse(styleCleaned);
            const sheet = ast?.stylesheet;
            if (sheet) {
                sanitizeRuleSet(ast.stylesheet);
            }
            return `<style>${css.stringify(ast)}</style>`;
        } catch (error) {
            return `CSS ERROR: ${error}`;
        }
    });
}
```

## 解释（完整流程图：输入 -> 变换 -> 输出）

```
输入：mes(原始文本), ch_name, isSystem/isUser/messageId, sanitizerOverrides, isReasoning
  -> (只对首条 bot 消息且非 system/user/reasoning) substituteParams(mes, undefined, ch_name)
  -> 修正 isSystem（评论消息/隐藏消息）
  -> (可选) 从 mes 头部去掉 user_prompt_bias（show_user_prompt_bias=false 时）
  -> 若 !isSystem：
      -> 计算 regexPlacement(REASONING/USER_INPUT/SLASH_COMMAND/AI_OUTPUT)
      -> 计算 display depth（排除 system 消息后的“从当前消息往后数”）
      -> getRegexedString(mes, placement, { characterOverride, isMarkdown:true, depth })
  -> (可选) fixMarkdown(mes, true)
  -> 若 !isSystem && encode_tags=true：转义 < 与 >
  -> 强制把 reasoning prefix/suffix 自身 escapeHtml（仅替换首个出现）
  -> 若 !isSystem：
      -> 若 encode_tags=false：保护标签内双引号（\ufffe）
      -> 把多种引号内容包裹为 <q>...</q>（跳过 style/代码块/inline code）
      -> 还原标签内引号
      -> \begin/\end{align*} => $$
      -> converter.makeHtml(mes)  // showdown
      -> 修正 code 内换行与 &amp;
  -> 若 !allow_name2_display && 非 user/system：去掉行首 “ch_name:”
  -> 安全：encodeStyleTags -> DOMPurify.sanitize(config{MESSAGE_SANITIZE,ADD_TAGS:['custom-style'],...}) -> decodeStyleTags(prefix '.mes_text ')
输出：可插入 DOM 的 HTML 字符串
```

## 复刻要点

- 必须按顺序实现：display regex -> markdown -> DOMPurify sanitize -> style 解码沙箱化；否则结果与安全边界都不等价。
- `encode_tags` 的语义不是“是否使用 markdown”，而是“是否把原始 `<...>` 当作 HTML 语法保留”；开启后必须在 markdown 前转义。
- `sanitizerOverrides` 是展示链路的扩展点：消息 `extra.uses_system_ui` 时打开 `MESSAGE_ALLOW_SYSTEM_UI`，以配合 DOMPurify hook 放行 `menu_button` 类名。

# STEP 3：解释“markdown vs html”的判定与分流（必须非常明确）

## 证据列表

### 1) SillyTavern 什么时候把文本当 markdown 渲染？是否总是 markdown？

证据：`public/script.js:1728-1783`（只有 `!isSystem` 分支才执行 `converter.makeHtml`）

```js
if (!isSystem) {
    // ...（quote 包装/align* 替换）
    mes = converter.makeHtml(mes);
    // ...（code newline 修正）
}
```

补充证据：`public/script.js:1660-1668`（“隐藏系统消息”被强制 `isSystem=false` 从而进入 markdown）

```js
// Let hidden messages have markdown
if (isSystem && ch_name !== systemUserName) {
    isSystem = false;
}
```

解释：展示链路并非“所有消息都当 markdown”。代码逻辑是：`isSystem` 为真时默认跳过 markdown，但对两类情况会强制 `isSystem=false`：评论消息、以及“非 systemUserName 的 system 消息”（隐藏消息）。

### 2) 是否存在 markdownOnly / promptOnly / encode_tags / 禁止 HTML 等开关？

证据：`public/script.js:1711-1715`（`power_user.encode_tags`）

```js
if (!isSystem && power_user.encode_tags) {
    mes = ...replaceAll('<', '&lt;')...
}
```

证据：`public/scripts/extensions/regex/engine.js:346-377`（`markdownOnly`/`promptOnly` 的脚本筛选）

```js
if (
    (script.markdownOnly && isMarkdown) ||
    (script.promptOnly && isPrompt) ||
    (!script.markdownOnly && !script.promptOnly && !isMarkdown && !isPrompt)
) {
    if (isEdit && !script.runOnEdit) {
        return;
    }
    if (script.placement.includes(placement)) {
        finalString = runRegexScript(script, finalString, { characterOverride });
    }
}
```

证据：`public/script.js:2440-2441`（system UI 放行开关：`MESSAGE_ALLOW_SYSTEM_UI`）

```js
const sanitizerOverrides = mes.extra?.uses_system_ui ? { MESSAGE_ALLOW_SYSTEM_UI: true } : {};
```

### 3) 哪些情况下 HTML 会被转义成 &lt; &gt;？哪些情况下会被保留后再 sanitize？

证据：`public/script.js:1711-1715`（encode_tags=true：转义）

```js
mes.replaceAll('<', '&lt;') ... replaceAll('>', '&gt;')
```

证据：`public/script.js:1730-1734,1798-1800`（encode_tags=false：保留 HTML，但最终仍经 DOMPurify）

```js
// Save double quotes in tags as a special character to prevent them from being encoded
if (!power_user.encode_tags) {
    mes = mes.replace(/<([^>]+)>/g, function (_, contents) {
        return '<' + contents.replace(/"/g, '\ufffe') + '>';
    });
}
// ...
mes = encodeStyleTags(mes);
mes = DOMPurify.sanitize(mes, config);
```

结论：
- `encode_tags=true`：`<...>` 先转义，后续 markdown/sanitize 都只能把它当普通文本。
- `encode_tags=false`：原始 HTML 会进入 markdown 输出 HTML，再进入 `DOMPurify.sanitize`，最终才写入 DOM。

### 4) 代码围栏 ``` 是如何变成 DOM 的（pre/code 结构在哪里产生/哪里假定存在）？

证据：`public/script.js:1769-1772`（围栏由 Showdown 解析生成 HTML）

```js
mes = converter.makeHtml(mes);
```

证据：`public/script.js:2366-2368`（后处理明确假定存在 `pre code` 结构）

```js
const codeBlocks = $(messageElement).find('pre code');
for (let i = 0; i < codeBlocks.length; i++) {
    hljs.highlightElement(codeBlocks.get(i));
    // ...
}
```

### 5) 高亮/复制按钮/媒体注入是否会改变 code 的 textContent？

证据：`public/script.js:2366-2381`（复制按钮被 append 到 `code`，复制时读 `textContent`）

```js
hljs.highlightElement(codeBlocks.get(i));
const copyButton = document.createElement('i');
copyButton.classList.add('fa-solid', 'fa-copy', 'code-copy', 'interactable');
codeBlocks.get(i).appendChild(copyButton);
copyButton.addEventListener('pointerup', async function () {
    const text = codeBlocks.get(i).textContent;
    await copyText(text);
});
```

结论：
- `hljs.highlightElement` 会把 `code` 内文本拆成带 `<span>` 的高亮结构。
- 复制按钮是一个额外 DOM 节点，被放在 `code` 内；因此 `code.textContent` 将包含该按钮节点的文本（若有）。

## 复刻要点

- 复刻时必须把“围栏 -> `<pre><code>`”交给同等能力的 Markdown 引擎，且后处理必须基于同样的 DOM 结构选择器（`pre code`）。
- 如果你在复制逻辑里使用 `textContent`，要接受它可能与原始模型输出不完全一致（因为 DOM 后处理注入了节点）；要保持等价，就按同样方式注入与读取。

# STEP 4：正则扩展系统（Regex extension）对 display 与 prompt 的差异

## 证据列表

### 1) getRegexedString 实现、参数、脚本筛选条件（markdownOnly/promptOnly/placement/depth/isEdit）

证据：`public/scripts/extensions/regex/engine.js:281-466`

```js
export function getRegexedString(rawString, placement, { characterOverride, isMarkdown, isPrompt, isEdit, depth } = {}) {
    // WTF have you passed me?
    if (typeof rawString !== 'string') {
        console.warn('getRegexedString: rawString is not a string. Returning empty string.');
        return '';
    }

    let finalString = rawString;
    if (extension_settings.disabledExtensions.includes('regex') || !rawString || placement === undefined) {
        return finalString;
    }

    const allRegex = getRegexScripts({ allowedOnly: true });
    allRegex.forEach((script) => {
        if (
            (script.markdownOnly && isMarkdown) ||
            (script.promptOnly && isPrompt) ||
            (!script.markdownOnly && !script.promptOnly && !isMarkdown && !isPrompt)
        ) {
            if (isEdit && !script.runOnEdit) {
                console.debug(`getRegexedString: Skipping script ${script.scriptName} because it does not run on edit`);
                return;
            }

            if (typeof depth === 'number') {
                if (!isNaN(script.minDepth) && script.minDepth !== null && script.minDepth >= -1 && depth < script.minDepth) {
                    return;
                }
                if (!isNaN(script.maxDepth) && script.maxDepth !== null && script.maxDepth >= 0 && depth > script.maxDepth) {
                    return;
                }
            }

            if (script.placement.includes(placement)) {
                finalString = runRegexScript(script, finalString, { characterOverride });
            }
        }
    });

    return finalString;
}
```

以及：`public/scripts/extensions/regex/engine.js:281-292`（placement 枚举）

```js
export const regex_placement = {
    MD_DISPLAY: 0,
    USER_INPUT: 1,
    AI_OUTPUT: 2,
    SLASH_COMMAND: 3,
    // 4 - sendAs (legacy)
    WORLD_INFO: 5,
    REASONING: 6,
};
```

### 2) display 渲染时如何调用（isMarkdown/isPrompt/placement/depth）

证据：`public/script.js:1676-1704`（messageFormatting：isMarkdown=true）

```js
mes = getRegexedString(mes, regexPlacement, {
    characterOverride: ch_name,
    isMarkdown: true,
    depth: depth,
});
```

证据：`public/script.js:1694-1698`（display depth 计算：排除 system 消息）

```js
const usableMessages = chat.map((x, index) => ({ message: x, index: index })).filter(x => !x.message.is_system);
const indexOf = usableMessages.findIndex(x => x.index === Number(messageId));
const depth = messageId >= 0 && indexOf !== -1 ? (usableMessages.length - indexOf - 1) : undefined;
```

### 3) prompt 构建时如何调用（isPrompt/isMarkdown/placement/depth）

证据：`public/script.js:4276-4303`（coreChat.map：isPrompt=true）

```js
coreChat = await Promise.all(coreChat.map(async (/** @type {ChatMessage} */ chatItem, index) => {
    let message = chatItem.mes;
    let regexType = chatItem.is_user ? regex_placement.USER_INPUT : regex_placement.AI_OUTPUT;
    let options = { isPrompt: true, depth: (coreChat.length - index - (isContinue ? 2 : 1)) };

    let regexedMessage = getRegexedString(message, regexType, options);
    // ...
    return {
        ...chatItem,
        mes: regexedMessage,
        index,
    };
}));
```

证据：`public/script.js:4307-4318`（prompt reasoning 也跑 regex：isPrompt=true, depth=depth）

```js
mes: promptReasoning.addToMessage(
    coreChat[i].mes,
    getRegexedString(
        String(coreChat[i].extra?.reasoning ?? ''),
        regex_placement.REASONING,
        { isPrompt: true, depth: depth },
    ),
    isPrefix,
    coreChat[i].extra?.reasoning_duration,
),
```

证据：`public/scripts/world-info.js:4956-4959`（World Info：isMarkdown=false, isPrompt=true）

```js
const regexDepth = entry.position === world_info_position.atDepth ? (entry.depth ?? DEFAULT_DEPTH) : null;
const content = getRegexedString(entry.content, regex_placement.WORLD_INFO, { depth: regexDepth, isMarkdown: false, isPrompt: true });
```

### 4) edit 场景（runOnEdit + isEdit=true）

证据：`public/script.js:7838-7855`（updateMessage：isEdit=true）

```js
text = getRegexedString(
    text,
    regexPlacement,
    {
        characterOverride: mes.extra?.type === 'narrator' ? undefined : mes.name,
        isEdit: true,
    },
);
```

## 解释

- display 与 prompt 的根本差异是：
  - display：`isMarkdown:true`，placement 来源于消息类型（AI/USER/SLASH/REASONING），depth 从“展示消息序列（排除 system）”反向计数。
  - prompt：`isPrompt:true`，placement 来源于 `chatItem.is_user`/`AI_OUTPUT`，depth 从 `coreChat.length - index - 偏移` 计算（continue 时偏移 2，否则 1）。
- `markdownOnly`/`promptOnly` 不是全局开关，而是“每条 regex script 的执行条件”；同一 script 可以只作用 display 或只作用 prompt。

## 复刻要点

- 必须把 regex 脚本的筛选维度做全：`placement` + `depth` + `markdownOnly/promptOnly` + `runOnEdit`。
- 必须复刻 display 的 depth 定义（排除 system）与 prompt 的 depth 定义（基于 coreChat/continue 偏移），否则脚本命中会不同。

# STEP 5：宏系统（如果存在多套，全部梳理）

## 证据列表

### 1) legacy 宏：`evaluateMacros()` + `initMacros()`（以及内置宏的执行顺序）

证据：`public/scripts/macros.js:609-741`

```js
export function evaluateMacros(content, env, postProcessFn) {
    // ...
    const preEnvMacros = [
        // Legacy non-curly macros
        { regex: /<USER>/gi, replace: () => typeof env.user === 'function' ? env.user() : env.user },
        { regex: /<BOT>/gi, replace: () => typeof env.char === 'function' ? env.char() : env.char },
        { regex: /<CHAR>/gi, replace: () => typeof env.char === 'function' ? env.char() : env.char },
        { regex: /<CHARIFNOTGROUP>/gi, replace: () => typeof env.group === 'function' ? env.group() : env.group },
        { regex: /<GROUP>/gi, replace: () => typeof env.group === 'function' ? env.group() : env.group },
        getDiceRollMacro(),
        ...getInstructMacros(env),
        ...getVariableMacros(),
        { regex: /{{newline}}/gi, replace: () => '\n' },
        { regex: /(?:\r?\n)*{{trim}}(?:\r?\n)*/gi, replace: () => '' },
        { regex: /{{noop}}/gi, replace: () => '' },
        { regex: /{{input}}/gi, replace: () => String($('#send_textarea').val()) },
    ];

    const postEnvMacros = [
        { regex: /{{maxPrompt}}/gi, replace: () => String(getMaxContextSize()) },
        { regex: /{{lastMessage}}/gi, replace: () => getLastMessage() },
        // ...
        { regex: /{{outlet::(.+?)}}/gi, replace: (_, key) => getOutletPrompt(key.trim()) || '' },
        getTimeDiffMacro(),
        getBannedWordsMacro(),
        getRandomReplaceMacro(),
        getPickReplaceMacro(rawContent),
    ];

    MacrosParser.populateEnv(env);
    // ... build envMacros by iterating env ...
    const macros = [...preEnvMacros, ...envMacros, ...postEnvMacros];
    for (const macro of macros) {
        if (!macro.regex.source.startsWith('<') && !content.includes('{{')) {
            break;
        }
        content = content.replace(macro.regex, (...args) => postProcessFn(macro.replace(...args)));
    }
    return content;
}

export function initMacros() {
    if (!power_user.experimental_macro_engine) {
        // ... register legacy runtime macros into MacrosParser ...
    }
    initRegisterMacros();
}
```

### 2) 新宏引擎入口：`initRegisterMacros()`（注册顺序）

证据：`public/scripts/macros/macro-system.js:64-83`

```js
export function initRegisterMacros() {
    // Core utilities and generic helpers
    registerCoreMacros();
    // Env / character / system / extras
    registerEnvMacros();
    // Runtime state tracking (eventSource etc.)
    registerStateMacros();
    // Chat/history inspection macros
    registerChatMacros();
    // Time / date / durations
    registerTimeMacros();
    // Variable and instruct macros
    registerVariableMacros();
    registerInstructMacros();
}
```

### 3) 新宏语法与解析（Chevrotain CST）

证据：`public/scripts/macros/engine/MacroParser.js:34-55`

```js
$.macro = $.RULE('macro', () => {
    $.CONSUME(Tokens.Macro.Start);
    $.OR([
        { ALT: () => $.CONSUME(Tokens.Macro.DoubleSlash, { LABEL: 'Macro.identifier' }) },
        { ALT: () => $.CONSUME(Tokens.Macro.Identifier, { LABEL: 'Macro.identifier' }) },
    ]);
    $.OPTION(() => $.SUBRULE($.arguments));
    $.CONSUME(Tokens.Macro.End);
});
```

### 4) 宏在哪些阶段展开（展示前/存储前/发送前）

证据：`public/script.js:5646-5692`（用户输入：入库前执行 `substituteParams`，再 `addOneMessage`）

```js
export async function sendMessageAsUser(messageText, messageBias, insertAt = null, compact = false, name = name1, avatar = user_avatar) {
    messageText = getRegexedString(messageText, regex_placement.USER_INPUT);

    const message = {
        // ...
        mes: substituteParams(messageText),
        extra: {
            isSmallSys: compact,
        },
    };
    // ...
    chat.push(message);
    const chat_id = (chat.length - 1);
    await eventSource.emit(event_types.MESSAGE_SENT, chat_id);
    addOneMessage(message);
    // ...
}
```

证据：`public/script.js:7838-7866`（编辑保存：先 regex(isEdit=true)，再 `substituteParams(text)` 入库）

```js
text = getRegexedString(
    text,
    regexPlacement,
    {
        characterOverride: mes.extra?.type === 'narrator' ? undefined : mes.name,
        isEdit: true,
    },
);
// ...
const bias = substituteParams(extractMessageBias(text));
text = substituteParams(text);
if (bias) {
    text = removeMacros(text);
}
mes['mes'] = text;
```

## 解释

- 宏系统确实存在“多套”：
  - legacy：`evaluateMacros` + `MacrosParser`（仍在项目中，且 `initMacros()` 在新引擎关闭时会注册 legacy runtime 宏）。
  - 新引擎：`MacroEngine/MacroRegistry`（由 `initRegisterMacros()` 注册宏集合）。
- “宏在哪展开”要分链路：
  - 用户输入：在存储进 `chat[]` 前就 `substituteParams(messageText)`，因此展示时 `messageFormatting` 不需要再次展开。
  - 编辑保存：同样先 `substituteParams(text)` 再写回 `chat[]`。
  - 展示链路：`messageFormatting` 只在“首条 bot 消息（messageId==0）”做了一次 `substituteParams(mes, undefined, ch_name)`，其余展示不做通用宏展开（见 STEP2 证据）。

### 宏是否会在 fenced code 内展开？

仅基于上述证据，能严格确认的是：
- 用户输入与编辑保存对整段字符串做 `substituteParams(...)`，没有“跳过 fenced code”的分支；因此如果用户在代码围栏里写 `{{...}}`，它会在“存储前”被展开。

证据：同上（`sendMessageAsUser` 与 `updateMessage`）这两处对 `text`/`messageText` 没有 fenced code 判断。

### 宏与 regex 的先后顺序（display 与 prompt 是否不同）

- 存储侧（用户发送）：先 regex（`getRegexedString(messageText, USER_INPUT)`），后宏（`substituteParams(messageText)`）。
  - 证据：`public/script.js:5646-5655`。
- 编辑保存：先 regex（isEdit），后宏（`substituteParams(text)`）。
  - 证据：`public/script.js:7847-7866`。
- 展示侧（messageFormatting）：先 regex（display），后 markdown + sanitize；宏仅对首条 bot 消息做一次（且在 regex 之前）。
  - 证据：`public/script.js:1649-1655` 与 `public/script.js:1699-1704`。

## 最小可复刻宏子集（变量插值/头像路径/角色名）

如果只想复刻 `{{user}}/{{char}}/{{group}}/{{model}}` 这类“环境变量宏”，最小依赖是：
- `substituteParams()`（或等价）：把输入字符串送入宏引擎
  - 证据：`public/script.js:2823-2852`（见 STEP8 会再集中摘录）。
- env 构建：至少构造 `names.user/names.char/names.group` 与 `system.model`
  - 证据：`public/scripts/macros/definitions/env-macros.js:16-28`（`user`/`char` 宏）

```js
MacroRegistry.registerMacro('user', { handler: ({ env }) => env.names.user });
MacroRegistry.registerMacro('char', { handler: ({ env }) => env.names.char });
```

# STEP 6：DOM 后处理（必须列出所有会影响渲染结果的后处理）

## 证据列表

### 1) 代码高亮（hljs）与复制按钮注入（影响 code DOM 结构）

证据：`public/script.js:2366-2383`

```js
hljs.highlightElement(codeBlocks.get(i));
const copyButton = document.createElement('i');
copyButton.classList.add('fa-solid', 'fa-copy', 'code-copy', 'interactable');
codeBlocks.get(i).appendChild(copyButton);
// ... copy reads textContent ...
```

触发点（新增/更新/流式结束/编辑）：

证据：`public/script.js:2541,1929,3566,8017,8115`

```js
addCopyToCodeBlocks(newMessage);
addCopyToCodeBlocks(messageElement);
addCopyToCodeBlocks(thisMesDiv);
addCopyToCodeBlocks(div.closest('.mes'));
```

### 2) 媒体/文件注入（会隐藏 `.mes_text` 或重建媒体 DOM）

证据：`public/script.js:2537-2539`（新增消息后处理）

```js
chatElement.find(`[mesid="${newMessageId}"] .mes_text`).append(messageText);
appendMediaToMessage(mes, newMessage, scroll ? SCROLL_BEHAVIOR.ADJUST : SCROLL_BEHAVIOR.NONE);
```

证据：`public/script.js:1929-1930`（更新后处理）

```js
addCopyToCodeBlocks(messageElement);
appendMediaToMessage(message, messageElement);
```

证据：`public/script.js:2138-2140`（appendMediaToMessage 内部会切换 `.mes_text` 显示）

```js
messageElement.find('.mes_text').toggleClass('displayNone', hideMessageText);
```

### 3) reasoning UI（会改写/新增 reasoning DOM 区域，且可能被流式更新）

证据：`public/script.js:1904-1905`（模板阶段触发）

```js
updateReasoningUI(mes);
```

证据：`public/script.js:1927`（更新阶段触发）

```js
updateReasoningUI(messageElement);
```

证据：`public/scripts/reasoning.js:539-558`（reasoning 内容容器写 `innerHTML`）

```js
const displayReasoning = messageFormatting(reasoning, '', false, false, messageId, {}, true);
this.messageReasoningContentDom.innerHTML = displayReasoning;
```

### 4) 流式 fade-in（morphdom，且跳过 pre/code 的文本拆分）

证据：`public/scripts/util/stream-fadein.js:17-35`

```js
// Skip ancestors of code/pre
if (textNode.parentElement && textNode.parentElement.closest('pre, code')) {
    continue;
}
```

### 5) MutationObserver：对 `.mes_text` 内 `<math>` 清空 text（影响最终渲染）

证据：`public/scripts/RossAscends-mods.js:69-87`

```js
} else if (mutation.target.classList.contains('mes_text')) {
    for (const element of mutation.target.getElementsByTagName('math')) {
        element.childNodes.forEach(function (child) {
            if (child.nodeType === Node.TEXT_NODE) {
                child.textContent = '';
            }
        });
    }
}
```

### 6) 复制行为修复（不改 DOM，但改变复制结果）

证据：`public/scripts/browser-fixes.js:5-53`

```js
if (!selection.anchorNode?.parentElement.closest('.mes_text')) {
    return;
}
if (node.nodeType === Node.ELEMENT_NODE && node.nodeName.toLowerCase() === 'q') {
    const span = document.createElement('span');
    // ...
    return span;
}
event.preventDefault();
event.clipboardData.setData('text/plain', newRange.toString());
```

## 解释

- “DOM 后处理”不只是在消息写入后做视觉增强，它会改变 DOM 结构（hljs + copyButton + 媒体 wrapper + reasoning 区块），从而影响：复制文本、选择文本、`textContent`、以及某些插件/脚本依赖的 DOM 查询。
- 这些后处理会在多个入口被重复调用（新增/更新/编辑/流式结束）。当前实现并没有在 `addCopyToCodeBlocks` 内部做“去重检查”，因此复刻时要按同样策略：要么保持同样的可能重复行为，要么在复刻清单里把“幂等化”作为可选增强（见 STEP8）。

## 复刻要点

- 复刻最小集合：`updateReasoningUI`、`appendMediaToMessage`、`addCopyToCodeBlocks`、（可选）`applyStreamFadeIn`。
- 必须按入口触发同样的后处理：
  - 新增（addOneMessage）
  - 单条更新（updateMessageBlock）
  - 流式结束（onFinishStreaming）
  - 编辑保存/取消（messageEditDone/messageEditCancel）

# STEP 7：安全模型总结（必须用证据支撑）

## 证据列表

### 1) XSS 防护点（按执行顺序）

1) `encode_tags`：把 `<`/`>` 变成文本（禁止原始 HTML）

证据：`public/script.js:1711-1715`

```js
mes.replaceAll('<', '&lt;')
```

2) `DOMPurify.sanitize`：消息最终 HTML sanitize（MESSAGE_SANITIZE=true）

证据：`public/script.js:1789-1801`

```js
mes = DOMPurify.sanitize(mes, config);
```

3) DOMPurify hooks：
- 强制 `rel=noopener`（防 tabnabbing）
- class 前缀化（防 CSS/JS hook 依赖 class 注入）
- unknown element `\n -> <br>`（控制 DOM 结构）
- 外链媒体禁止（移除 node）

证据：`public/scripts/chats.js:1904-2055`

```js
node.setAttribute('rel', 'noopener');
data.attrValue = 'custom-' + v;
node.remove();
```

4) `<style>` 沙箱化：encode -> sanitize -> decode 时 prefix selector、过滤 @import、禁止外链 URL

证据：`public/scripts/chats.js:536-626`

```js
return `<custom-style>${encodeURIComponent(match)}</custom-style>`;
ruleSet.rules = ruleSet.rules.filter(rule => rule.type !== 'import');
rule.selectors[i] = prefix + sanitizeSelector(selector);
rule.declarations = rule.declarations.filter(declaration => !declaration.value.includes('://'));
```

### 2) DOMPurify 配置中的关键开关

证据：`public/script.js:1789-1797`

```js
MESSAGE_SANITIZE: true,
ADD_TAGS: ['custom-style'],
...sanitizerOverrides,
```

证据：`public/script.js:2440-2441`（覆盖项：`MESSAGE_ALLOW_SYSTEM_UI`）

```js
const sanitizerOverrides = mes.extra?.uses_system_ui ? { MESSAGE_ALLOW_SYSTEM_UI: true } : {};
```

### 3) 可能绕过点（如存在）

已确认存在的“未走 sanitize 的写入”只有常量 `'...'`：

证据：`public/script.js:9947-9956`

```js
thisMesDiv.find('.mes_text').html('...');
```

除此之外，写入 `.mes_text` 的路径都来自 `messageFormatting()` 的输出，且该输出统一走 `DOMPurify.sanitize`：

证据：`public/script.js:2537-2538`（append messageText）

```js
chatElement.find(`[mesid="${newMessageId}"] .mes_text`).append(messageText);
```

其中 `messageText` 来源：

证据：`public/script.js:2443-2451`

```js
messageText = messageFormatting(
    messageText,
    mes.name,
    isSystem,
    mes.is_user,
    chat.indexOf(mes),
    sanitizerOverrides,
    false,
);
```

结论（仅基于源码检索结果）：未发现“把任意用户/模型输入绕过 sanitize 直接写入 `.mes_text`”的路径。

## 复刻要点（安全边界）

- 必须把 “encode_tags（可选）+ DOMPurify + hooks + style sandbox”作为一个整体等价实现；缺任何一环都会改变安全边界。
- `MESSAGE_ALLOW_SYSTEM_UI` 是一个“刻意的放行口”，必须只在受控消息上开启（本仓库条件是 `mes.extra?.uses_system_ui`）。
- 外链媒体策略是 sanitize hook 里做的（不是仅靠 DOMPurify 默认配置）；复刻时必须把“外链识别规则 + remove + warning”一起做。

# STEP 8：复刻规格说明（Implementation Spec）

## 输入：消息对象/字段（展示侧）

证据：`public/script.js:2398-2472`（addOneMessage 使用的字段）

```js
let messageText = mes['mes'];
if (mes?.extra?.display_text) {
    messageText = mes.extra.display_text;
}
const isSystem = mes.is_system;
const title = mes.title;
const sanitizerOverrides = mes.extra?.uses_system_ui ? { MESSAGE_ALLOW_SYSTEM_UI: true } : {};
// ...
let params = {
  mesId: forceId ?? chat.length - 1,
  swipeId: mes.swipe_id ?? 0,
  characterName: mes.name,
  isUser: mes.is_user,
  bias: bias,
  isSystem: isSystem,
  title: title,
  extra: mes.extra,
  tokenCount: mes.extra?.token_count ?? 0,
  type: mes.extra?.type ?? '',
  // timer fields...
};
```

可复刻的最小消息结构（展示侧需要）：

```ts
type ChatMessage = {
  name: string;
  mes: string;
  is_user: boolean;
  is_system: boolean;
  send_date?: any;
  gen_started?: any;
  gen_finished?: any;
  title?: string;
  swipe_id?: number;
  swipes?: string[];
  swipe_info?: any[];
  force_avatar?: string;
  extra?: {
    display_text?: string;
    bias?: string;
    type?: string; // narrator/tool etc
    uses_system_ui?: boolean;
    tool_invocations?: any[];
    token_count?: number;
    reasoning?: string;
    reasoning_duration?: number|null;
    reasoning_signature?: string|null;
    media?: any[];
    files?: any[];
    media_display?: any;
    isSmallSys?: boolean;
    api?: string;
    model?: string;
  };
};
```

## 处理顺序（display 链路）

必须等价实现（严格顺序）：

1) 取展示文本：`text = message.extra.display_text ?? message.mes`

证据：`public/script.js:1922-1924`

```js
const text = message?.extra?.display_text ?? message.mes;
```

2) display regex（placement + depth + isMarkdown）

证据：`public/script.js:1694-1704`

```js
const depth = messageId >= 0 && indexOf !== -1 ? (usableMessages.length - indexOf - 1) : undefined;
mes = getRegexedString(mes, regexPlacement, { characterOverride: ch_name, isMarkdown: true, depth: depth });
```

3) markdown（Showdown）

证据：`public/script.js:1771`

```js
mes = converter.makeHtml(mes);
```

4) sanitize（含 custom-style 支持）

证据：`public/script.js:1798-1800`

```js
mes = encodeStyleTags(mes);
mes = DOMPurify.sanitize(mes, config);
mes = decodeStyleTags(mes, { prefix: '.mes_text ' });
```

5) 写 DOM（`.html`/`.append`/`innerHTML`）

证据：
- `public/script.js:2521` swipe 覆盖
- `public/script.js:2537` 普通追加
- `public/script.js:3544` 流式覆盖

```js
swipeMessage.find('.mes_text').html(messageText)
chatElement.find(`[mesid="${newMessageId}"] .mes_text`).append(messageText)
this.messageTextDom.innerHTML = formattedText
```

6) DOM 后处理（hljs/copy/media/reasoning/stream fade-in）

证据：`public/script.js:2523-2524,2538-2541,3566-3569`

```js
updateReasoningUI(swipeMessage);
appendMediaToMessage(mes, swipeMessage, ...);
addCopyToCodeBlocks(newMessage);
await this.reasoningHandler.finish(messageId);
```

## 处理顺序（prompt 链路）

必须等价实现（至少这些节点）：

1) cleanUpMessage：停止词裁剪 + regex（AI_OUTPUT/USER_INPUT）+ auto_fix_generated_markdown

证据：`public/script.js:6160-6210`

```js
// Add the prompt bias before anything else
getMessage = substituteParams(power_user.user_prompt_bias) + getMessage;
// ... stoppingStrings trimming ...
getMessage = getRegexedString(getMessage, isImpersonate ? regex_placement.USER_INPUT : regex_placement.AI_OUTPUT);
// ...
if (power_user.auto_fix_generated_markdown) {
    getMessage = fixMarkdown(getMessage, false);
}
```

2) prompt regex：对 coreChat 的每条消息跑 `getRegexedString(..., { isPrompt:true, depth })`

证据：`public/script.js:4278-4282`

```js
let options = { isPrompt: true, depth: (coreChat.length - index - (isContinue ? 2 : 1)) };
let regexedMessage = getRegexedString(message, regexType, options);
```

3) World Info：`isMarkdown:false, isPrompt:true, depth: entry.depth`

证据：`public/scripts/world-info.js:4957-4959`

```js
const content = getRegexedString(entry.content, regex_placement.WORLD_INFO, { depth: regexDepth, isMarkdown: false, isPrompt: true });
```

## 关键设置项（默认值、作用范围、证据）

证据：`public/scripts/power-user.js:122-230`（默认值节选）

```js
user_prompt_bias: '',
show_user_prompt_bias: true,
stream_fade_in: false,
auto_fix_generated_markdown: true,
allow_name2_display: false,
```

证据：`public/scripts/power-user.js:304-347`（默认值节选：encode_tags/forbid_external_media/experimental_macro_engine）

```js
encode_tags: false,
experimental_macro_engine: false,
forbid_external_media: true,
external_media_allowed_overrides: [],
external_media_forbidden_overrides: [],
```

## 最小复刻版本（MVP）

必须等价实现：
- `reloadMarkdownProcessor()`：Showdown 初始化 + 两个扩展（underscore/exclusion）。
- `messageFormatting()`：display regex + markdown + encode_tags 分支 + DOMPurify sanitize + custom-style 编解码。
- DOM 写入入口：`addOneMessage`、`updateMessageBlock`、流式 `onProgressStreaming` 的 `innerHTML` 更新。
- 后处理：`addCopyToCodeBlocks`（hljs + copyButton）、`appendMediaToMessage`（至少不破坏 `.mes_text` 显示逻辑）、`updateReasoningUI`（可先 stub，但要保持 DOM 写入点一致）。

可选增强：
- 增量渲染：`applyStreamFadeIn`（morphdom + Intl.Segmenter，且跳过 pre/code）。
- 幂等化：对 `addCopyToCodeBlocks` 增加“已注入则跳过”逻辑（本仓库目前未体现为必需）。
- 可观测性：对每个入口记录“messageId -> formatted HTML -> postprocess”耗时。

# 关键代码摘录集（便于复制）

## messageFormatting（展示侧主转换）

`public/script.js:1644-1803`

```js
export function messageFormatting(mes, ch_name, isSystem, isUser, messageId, sanitizerOverrides = {}, isReasoning = false) {
    if (!mes) {
        return '';
    }
    if (Number(messageId) === 0 && !isSystem && !isUser && !isReasoning) {
        const mesBeforeReplace = mes;
        const chatMessage = chat[messageId];
        mes = substituteParams(mes, undefined, ch_name);
        if (chatMessage && chatMessage.mes === mesBeforeReplace && chatMessage.extra?.display_text !== mesBeforeReplace) {
            chatMessage.mes = mes;
        }
    }
    mesForShowdownParse = mes;
    if (ch_name === COMMENT_NAME_DEFAULT && isSystem && !isUser) {
        isSystem = false;
    }
    if (isSystem && ch_name !== systemUserName) {
        isSystem = false;
    }
    const replacedPromptBias = power_user.user_prompt_bias && substituteParams(power_user.user_prompt_bias);
    if (!power_user.show_user_prompt_bias && ch_name && !isUser && !isSystem && replacedPromptBias && mes.startsWith(replacedPromptBias)) {
        mes = mes.slice(replacedPromptBias.length);
    }
    if (!isSystem) {
        function getRegexPlacement() {
            try {
                if (isReasoning) {
                    return regex_placement.REASONING;
                }
                if (isUser) {
                    return regex_placement.USER_INPUT;
                } else if (chat[messageId]?.extra?.type === 'narrator') {
                    return regex_placement.SLASH_COMMAND;
                } else {
                    return regex_placement.AI_OUTPUT;
                }
            } catch {
                return regex_placement.AI_OUTPUT;
            }
        }

        const regexPlacement = getRegexPlacement();
        const usableMessages = chat.map((x, index) => ({ message: x, index: index })).filter(x => !x.message.is_system);
        const indexOf = usableMessages.findIndex(x => x.index === Number(messageId));
        const depth = messageId >= 0 && indexOf !== -1 ? (usableMessages.length - indexOf - 1) : undefined;

        mes = getRegexedString(mes, regexPlacement, {
            characterOverride: ch_name,
            isMarkdown: true,
            depth: depth,
        });
    }
    if (power_user.auto_fix_generated_markdown) {
        mes = fixMarkdown(mes, true);
    }
    if (!isSystem && power_user.encode_tags) {
        mes = canUseNegativeLookbehind()
            ? mes.replaceAll('<', '&lt;').replace(new RegExp('(?<!^|\\n\\s*)>', 'g'), '&gt;')
            : mes.replaceAll('<', '&lt;').replaceAll('>', '&gt;');
    }
    [power_user.reasoning.prefix, power_user.reasoning.suffix].forEach((reasoningString) => {
        if (!reasoningString || !reasoningString.trim().length) {
            return;
        }
        if (mes.includes(reasoningString)) {
            mes = mes.replace(reasoningString, escapeHtml(reasoningString));
        }
    });
    if (!isSystem) {
        if (!power_user.encode_tags) {
            mes = mes.replace(/<([^>]+)>/g, function (_, contents) {
                return '<' + contents.replace(/"/g, '\ufffe') + '>';
            });
        }
        mes = mes.replace(
            /<style>[\s\S]*?<\/style>|```[\s\S]*?```|~~~[\s\S]*?~~~|``[\s\S]*?``|`[\s\S]*?`|(".*?")|(\u201C.*?\u201D)|(\u00AB.*?\u00BB)|(\u300C.*?\u300D)|(\u300E.*?\u300F)|(\uFF02.*?\uFF02)/gim,
            function (match, p1, p2, p3, p4, p5, p6) {
                if (p1) {
                    return `<q>"${p1.slice(1, -1)}"</q>`;
                } else if (p2) {
                    return `<q>“${p2.slice(1, -1)}”</q>`;
                } else if (p3) {
                    return `<q>«${p3.slice(1, -1)}»</q>`;
                } else if (p4) {
                    return `<q>「${p4.slice(1, -1)}」</q>`;
                } else if (p5) {
                    return `<q>『${p5.slice(1, -1)}』</q>`;
                } else if (p6) {
                    return `<q>＂${p6.slice(1, -1)}＂</q>`;
                } else {
                    return match;
                }
            },
        );
        if (!power_user.encode_tags) {
            mes = mes.replace(/\ufffe/g, '"');
        }
        mes = mes.replaceAll('\\begin{align*}', '$$');
        mes = mes.replaceAll('\\end{align*}', '$$');
        mes = converter.makeHtml(mes);
        mes = mes.replace(/<code(.*)>[\s\S]*?<\/code>/g, function (match) {
            return match.replace(/\n/gm, '\u0000');
        });
        mes = mes.replace(/\u0000/g, '\n');
        mes = mes.trim();
        mes = mes.replace(/<code(.*)>[\s\S]*?<\/code>/g, function (match) {
            return match.replace(/&amp;/g, '&');
        });
    }
    if (!power_user.allow_name2_display && ch_name && !isUser && !isSystem) {
        mes = mes.replace(new RegExp(`(^|\\n)${escapeRegex(ch_name)}:`, 'g'), '$1');
    }
    const config = {
        RETURN_DOM: false,
        RETURN_DOM_FRAGMENT: false,
        RETURN_TRUSTED_TYPE: false,
        MESSAGE_SANITIZE: true,
        ADD_TAGS: ['custom-style'],
        ...sanitizerOverrides,
    };
    mes = encodeStyleTags(mes);
    mes = DOMPurify.sanitize(mes, config);
    mes = decodeStyleTags(mes, { prefix: '.mes_text ' });
    return mes;
}
```

## getRegexedString（regex 扩展入口）

`public/scripts/extensions/regex/engine.js:281-466`

```js
export function getRegexedString(rawString, placement, { characterOverride, isMarkdown, isPrompt, isEdit, depth } = {}) {
    // ...
}
```

## addDOMPurifyHooks（sanitize hook）

`public/scripts/chats.js:1904-2055`

```js
export function addDOMPurifyHooks() {
    // ...
}
```

## addOneMessage（写入聊天 DOM 的核心）

`public/script.js:2398-2562`

```js
export function addOneMessage(mes, { type = 'normal', insertAfter = null, scroll = true, insertBefore = null, forceId = null, showSwipes = true } = {}) {
    // ...
}
```

# 检索关键词命中情况（按要求列出）

说明：我在仓库内对以下关键词进行了全局检索（grep），并在命中文件中读取原文片段用于证据。

- messageFormatting：命中（`public/script.js:1644`）
- reloadMarkdownProcessor：命中（`public/script.js:489`）
- showdown：命中（`public/script.js` / `public/lib.js` 等）
- DOMPurify：命中（`public/script.js` / `public/scripts/chats.js` 等）
- sanitize：命中（多处 `DOMPurify.sanitize`）
- encode_tags：命中（`public/script.js:1711` 与 `public/scripts/power-user.js`）
- addCopyToCodeBlocks：命中（`public/script.js:2366`）
- hljs：命中（`public/script.js` / `public/lib.js`）
- getRegexedString：命中（`public/scripts/extensions/regex/engine.js:334`）
- regex：命中（大量）
- placement：命中（`regex_placement` 与 UI 相关）
- promptOnly：命中（regex engine + regex UI）
- markdownOnly：命中（regex engine + regex UI）
- MacrosParser：命中（`public/scripts/macros.js`）
- custom-style：命中（`ADD_TAGS:['custom-style']` + style 编解码）
- encodeStyleTags：命中（`public/scripts/chats.js:536`）
- decodeStyleTags：命中（`public/scripts/chats.js:551`）
- mes_text：命中（`public/script.js` / `public/scripts/RossAscends-mods.js` 等）
- pre code：命中（`$(messageElement).find('pre code')`）
- innerHTML：命中（流式 `this.messageTextDom.innerHTML = formattedText`）

额外说明（可复核的“未命中”示例）：
- showdown.setOption：未命中（本仓库使用 `new showdown.Converter({...})` 而非 setOption 风格）
