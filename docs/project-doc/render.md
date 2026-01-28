# 酒馆助手 HTML/CSS/JS 渲染实现详解

## 一、核心概念

### 1.1 什么是前端代码渲染

将 AI 输出的 HTML/CSS/JavaScript 代码在 SillyTavern 聊天界面中以**可交互的 iframe** 形式渲染，而不是作为纯文本显示。

### 1.2 整体架构

```
聊天消息渲染完成
       ↓
message.ts:render$mes() 查找符合条件的前端代码块
       ↓
用 <div class="TH-render"> 包装代码块
       ↓
Render.vue 使用 Teleport 将 Iframe 组件渲染到包装元素内
       ↓
Iframe.vue 创建 iframe 并加载内容
       ↓
iframe 内脚本调整高度并 postMessage 通知父窗口
```

---

## 二、渲染配置

**文件**: `src/type/settings.ts`

```typescript
render: z
  .object({
    enabled: z.boolean().default(true),           // 是否启用渲染器
    collapse_code_block: CollapseCodeBlock.default('frontend_only'), // 代码折叠模式
    use_blob_url: z.boolean().default(false),     // 使用 Blob URL 渲染
    depth: z.number().default(0),                 // 渲染深度 (0 = 全部)
  })
```

| 配置项 | 可选值 | 说明 |
|--------|--------|------|
| `enabled` | `true` / `false` | 是否启用渲染器 |
| `collapse_code_block` | `none` / `frontend_only` / `all` | 代码折叠模式 |
| `use_blob_url` | `true` / `false` | 使用 Blob URL 而非 srcdoc |
| `depth` | 数字 | 渲染深度，0 表示全部可见消息 |

---

## 三、前端代码检测规则

### 3.1 检测函数

**文件**: `src/util/is_frontend.ts:1-3`

```typescript
export function isFrontend(content: string): boolean {
  return ['html>', '<head>', '<body'].some(tag => content.includes(tag));
}
```

### 3.2 检测条件（满足任一即视为前端代码）

| 检测项 | 说明 |
|--------|------|
| 包含 `html>` | HTML 标签闭合 |
| 包含 `<head>` | HEAD 标签 |
| 包含 `<body>` | BODY 标签 |

### 3.3 检测位置

此检测应用于 `<pre>` 标签内的代码块文本：
```typescript
$mes.find('pre')
  .filter((_index, pre) => isFrontend($(pre).text()))
```

**重要**: 只有 `<pre>` 标签内的代码块才会被检测和渲染。

---

## 四、文本预处理规则

### 4.1 Tavern 正则处理

**文件**: `src/function/tavern_regex.ts:22-68`

```typescript
export function formatAsTavernRegexedString(
  text: string,
  source: 'user_input' | 'ai_output' | 'slash_command' | 'world_info' | 'reasoning',
  destination: 'display' | 'prompt',
  { depth, character_name }: FormatAsTavernRegexedStringOption = {},
)
```

**处理流程**:
1. 调用 `getRegexedString()` 应用全局/角色正则脚本
2. 调用 `substituteParams()` 替换参数
3. 应用宏替换 (`macros.forEach`)

**正则来源**:
- 全局正则: `extension_settings.regex`
- 角色正则: `characters.at(chid).data.extensions.regex_scripts`

### 4.2 宏替换

**文件**: `src/function/macro_like.ts:16-68`

| 宏模式 | 用途 | 示例 |
|--------|------|------|
| `{{get_message_variable::路径}}` | 获取消息变量 | `{{get_message_variable::name}}` |
| `{{get_chat_variable::路径}}` | 获取聊天变量 | `{{get_chat_variable::title}}` |
| `{{get_character_variable::路径}}` | 获取角色变量 | `{{get_character_variable::description}}` |
| `{{get_preset_variable::路径}}` | 获取预设变量 | `{{get_preset_variable::temp}}` |
| `{{get_global_variable::路径}}` | 获取全局变量 | `{{get_global_variable::theme}}` |
| `{{format_xxx_variable::路径}}` | 格式化变量（带前缀 YAML 输出） | `{{format_chat_variable::notes}}` |

**宏处理事件**:
- `GENERATE_AFTER_DATA` / `CHAT_COMPLETION_SETTINGS_READY` - 提示词生成时
- `chatLoaded` - 聊天加载时
- `CHARACTER_MESSAGE_RENDERED` / `USER_MESSAGE_RENDERED` - 消息渲染时
- `MESSAGE_UPDATED` / `MESSAGE_SWIPED` - 消息更新时

### 4.3 VH 单位转换

**文件**: `src/panel/render/iframe.ts:5-75`

```typescript
function replaceVhInContent(content: string): string
```

将 `vh` 单位转换为 `var(--TH-viewport-height)` CSS 变量，解决 iframe 内 `100vh` 计算问题。

**转换场景**:

| 场景 | 模式 | 示例 |
|------|------|------|
| CSS 声明块 | `min-height: ...vh` | `.el { min-height: 100vh; }` |
| 行内 style | `style="min-height: ...vh"` | `<div style="min-height: 100vh">` |
| JavaScript 赋值 | `.style.minHeight = "...vh"` | `el.style.minHeight = '100vh'` |
| JavaScript API | `.setProperty('min-height', "...vh")` | `el.style.setProperty('min-height', '100vh')` |

**转换公式**:
```javascript
100vh → var(--TH-viewport-height)
Nvh   → calc(var(--TH-viewport-height) * N/100)
```

---

## 五、消息代码块处理

### 5.1 核心渲染函数

**文件**: `src/store/iframe_runtimes/message.ts:8-28`

```typescript
function render$mes($mes: JQuery<HTMLElement>, reload_memo: string): Runtime[] {
  return _($mes.toArray())
    .map(div => {
      const message_id = Number($(div).attr('mesid'));
      const $element = $(div)
        .find('pre')
        .filter((_index, pre) => isFrontend($(pre).text()))  // 检测是否前端代码
        .map((_index, pre) => {
          const $pre = $(pre);
          const $possible_div = $pre.parent('div.TH-render');
          if ($possible_div.length > 0) {
            return $possible_div[0];
          }
          $pre.wrap('<div class="TH-render">');  // 用 div 包装
          return $pre.parent('div.TH-render')[0];
        });
      return { message_id, reload_memo, elements: $element.toArray() };
    })
    .filter(({ elements }) => elements.length > 0)
    .value();
}
```

### 5.2 处理步骤

1. 查找消息元素 `#chat > .mes[mesid]`
2. 查找其中的 `<pre>` 标签
3. 过滤出被 `isFrontend()` 判定为前端的代码块
4. 用 `<div class="TH-render">` 包装代码块
5. 返回需要渲染的 Runtime 列表

### 5.3 渲染深度计算

**文件**: `src/store/iframe_runtimes/message.ts:35-41`

```typescript
function calcToRender(depth: number): number[] {
  const min_showed_message_id = Number($('#chat > .mes').first().attr('mesid'));
  return _.range(
    depth === 0 ? min_showed_message_id : Math.max(min_showed_message_id, chat.length - depth),
    chat.length,
  );
}
```

| depth 值 | 渲染范围 |
|----------|----------|
| `0` | 所有可见消息（从 `min_showed_message_id` 开始） |
| `>0` | 最近 N 条消息 |

### 5.4 运行时审核

**文件**: `src/store/iframe_runtimes/message.ts:43-50`

```typescript
function auditRuntimes(runtimes: Runtime[], depth: number): Runtime[] {
  const rendered = _.map(runtimes, runtime => runtime.message_id);
  const to_render = calcToRender(depth);
  return _.concat(
    _.filter(runtimes, runtime => _.includes(to_render, runtime.message_id)),
    renderMessages(_.difference(to_render, rendered), uuidv4()),
  );
}
```

---

## 六、忽略规则

### 6.1 语法高亮忽略

**文件**: `src/panel/render/optimize_hljs.ts:5-11`

```typescript
hljs.highlightElement = (element: HTMLElement) => {
  if (isFrontend($(element).text())) {
    return;  // 前端代码不进行语法高亮
  }
  originalHighlightElement(element);
};
```

**规则**: 被判定为前端的代码块不进行 highlight.js 语法高亮，避免破坏代码。

### 6.2 代码折叠忽略

**文件**: `src/panel/render/use_collapse_code_block.ts:12-15`

```typescript
const is_frontend = isFrontend($pre.text());
if (collapse_code_block === 'frontend_only' && !is_frontend) {
  return;  // 非前端代码块不折叠
}
```

**规则**:
| collapse_code_block 值 | 前端代码 | 非前端代码 |
|------------------------|----------|------------|
| `none` | 不折叠 | 不折叠 |
| `frontend_only` | 折叠 | 不折叠 |
| `all` | 折叠 | 折叠 |

### 6.3 宏处理预检查

**文件**: `src/panel/render/macro_like.ts:40-48`

```typescript
if (
  $mes_text.length === 0 ||
  !macros.some(macro => {
    macro.regex.lastIndex = 0;
    return macro.regex.test($mes_text.text());
  })
) {
  return;  // 无宏时跳过处理
}
```

**规则**: `.mes_text` 内无宏匹配时跳过宏处理。

### 6.4 iframe 渲染跳过

**文件**: `src/panel/render/macro_like.ts:60-61`

```typescript
// 因未知原因, 一些设备上在初次进入角色卡时会重复渲染，因此需要移除额外渲染的 iframe
$mes_text.find('.TH-render > iframe').remove();
```

**规则**: 宏处理前先移除已存在的 iframe，避免重复渲染。

---

## 七、代码折叠功能

### 7.1 折叠按钮创建

**文件**: `src/panel/render/use_collapse_code_block.ts:22-34`

```typescript
const $button = $('<div class="TH-collapse-code-block-button">')
  .text(is_frontend ? '显示前端代码块' : '显示代码块')
  .on('click', function () {
    const is_visible = $pre.is(':visible');
    if (is_visible) {
      $pre.addClass('hidden!');
      $(this).text(is_frontend ? '显示前端代码块' : '显示代码块');
    } else {
      $pre.removeClass('hidden!');
      $(this).text(is_frontend ? '隐藏前端代码块' : '隐藏代码块');
    }
  })
  .prependTo($div);
```

### 7.2 折叠触发事件

```typescript
// 流式输出时
eventSource.on(event_types.STREAM_TOKEN_RECEIVED, ...)

// 消息渲染时
eventSource.on(event_types.CHARACTER_MESSAGE_RENDERED, ...)
eventSource.on(event_types.USER_MESSAGE_RENDERED, ...)

// 消息更新时
eventSource.on(event_types.MESSAGE_UPDATED, ...)
eventSource.on(event_types.MESSAGE_SWIPED, ...)

// 聊天加载时
eventSource.on('chatLoaded', ...)
```

### 7.3 折叠状态同步

```typescript
if ($div.children('iframe').length > 0) {
  $button.addClass('hidden!');  // 有 iframe 时隐藏折叠按钮
}
```

**规则**: 当代码块已被 iframe 渲染时，隐藏折叠按钮。

---

## 八、iframe 组件实现

### 8.1 Iframe.vue 模板

**文件**: `src/panel/render/Iframe.vue`

```vue
<template>
  <iframe
    :id="prefixed_id"
    :name="prefixed_id"
    ref="iframe_ref"
    loading="lazy"
    v-bind="src_prop"
    class="w-full"
    frameborder="0"
    @load="onLoad"
  />
</template>
```

### 8.2 内容加载

**文件**: `src/panel/render/Iframe.vue`

```typescript
// 从 <pre><code> 中提取文本内容
const content = createSrcContent($pre.find('code').text(), props.useBlobUrl);

// 两种模式：srcdoc 或 Blob URL
if (!props.useBlobUrl) {
  return { srcdoc: content };
}
return { src: URL.createObjectURL(new Blob([content], { type: 'text/html' })) };
```

### 8.3 高度调整监听

```typescript
useEventListener('message', event => {
  if (event?.data?.type === 'TH_ADJUST_IFRAME_HEIGHT' && event?.data?.iframe_name === iframe_ref.value?.id) {
    iframe_ref.value!.style.height = `${event.data.height}px`;
  }
});
useEventListener(window, 'resize', () => {
  iframe_ref.value?.contentWindow?.postMessage({ type: 'TH_UPDATE_VIEWPORT_HEIGHT' }, '*');
});
```

---

## 九、iframe 内容构建

### 9.1 完整 HTML 结构

**文件**: `src/panel/render/iframe.ts:78-104`

```typescript
export function createSrcContent(content: string, use_blob_url: boolean) {
  content = replaceVhInContent(content);  // VH 单位转换

  return `
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
${use_blob_url ? `<base href="${window.location.origin}"/>` : ''}
<style>
*,*::before,*::after{box-sizing:border-box;}
html,body{margin:0!important;padding:0;overflow:hidden!important;max-width:100%!important;}
.user_avatar,.user-avatar{background-image:url('${getUserAvatarPath()}')}
.char_avatar,.char-avatar{background-image:url('${getCharAvatarPath()}')}
</style>
${third_party}  <!-- 注入第三方库 -->
<script src="${predefine_url}"></script>
<script src="https://testingcf.jsdelivr.net/npm/vue/dist/vue.runtime.global.prod.min.js"></script>
<script src="${adjust_viewport_url}"></script>
<script src="${adjust_iframe_height_url}"></script>
</head>
<body>
${content}  <!-- 用户代码放在 body 内 -->
</body>
</html>
`;
}
```

### 9.2 注入的第三方库

**文件**: `src/iframe/third_party_message.html`

```html
<link rel="stylesheet" href="https://testingcf.jsdelivr.net/npm/@fortawesome/fontawesome-free/css/all.min.css" />
<script src="/scripts/extensions/third-party/JS-Slash-Runner/lib/tailwindcss.min.js"></script>
<script src="https://testingcf.jsdelivr.net/npm/jquery/dist/jquery.min.js"></script>
<script src="https://testingcf.jsdelivr.net/npm/jquery-ui/dist/jquery-ui.min.js"></script>
<link rel="stylesheet" href="https://testingcf.jsdelivr.net/npm/jquery-ui/themes/base/theme.min.css" />
<script src="https://testingcf.jsdelivr.net/npm/jquery-ui-touch-punch"></script>
<script src="https://testingcf.jsdelivr.net/npm/vue/dist/vue.runtime.global.prod.min.js"></script>
<script src="https://testingcf.jsdelivr.net/npm/vue-router/dist/vue-router.global.prod.min.js"></script>
<script src="https://testingcf.jsdelivr.net/npm/pixi.js/dist/pixi.min.js"></script>
```

| 库 | 用途 |
|------|------|
| Font Awesome | 图标 |
| jQuery + jQuery UI | DOM 操作、拖拽 |
| Vue 3 Runtime | 响应式 UI |
| Vue Router | 路由 |
| Pixi.js | 2D 图形/游戏 |
| TailwindCSS | 样式框架 |

### 9.3 注入的全局变量

**文件**: `src/iframe/predefine.js`

```javascript
// 继承父窗口的 lodash
window._ = window.parent._;

// SillyTavern 上下文
Object.defineProperty(window, 'SillyTavern', {
  get: () => _.get(window.parent, 'SillyTavern').getContext(),
});

// 合并父窗口的全局对象
result = _(window);
result = result.merge(_.pick(window.parent, ['EjsTemplate', 'TavernHelper', 'YAML', 'showdown', 'toastr', 'z']));

// Mvu 状态管理
if (_.has(window.parent, 'Mvu')) {
  Object.defineProperty(window, 'Mvu', {
    get: () => _.get(window.parent, 'Mvu'),
    set: () => {},
  });
}
```

| 全局变量 | 来源 | 用途 |
|----------|------|------|
| `_` | parent | lodash 工具库 |
| `SillyTavern` | parent | SillyTavern 上下文 |
| `YAML` | parent | YAML 解析 |
| `showdown` | parent | Markdown 转换 |
| `toastr` | parent | 通知提示 |
| `Mvu` | parent | 状态管理 |

---

## 十、高度自动调整

### 10.1 高度测量

**文件**: `src/iframe/adjust_iframe_height.js`

```javascript
function measureAndPost() {
  const body = document.body;
  const html = document.documentElement;

  let height = 0;
  if (IS_BLOB_MODE) {
    // Blob 模式：计算子元素高度
    const children = Array.from(body.children || []);
    // 排除绝对/固定定位元素
    for (const el of children) {
      if (position === 'absolute' || position === 'fixed') continue;
      // 计算元素边界...
    }
    height = total_height;
  } else {
    // srcdoc 模式
    height = body.scrollHeight;
  }

  window.parent.postMessage({ type: 'TH_ADJUST_IFRAME_HEIGHT', iframe_name: getIframeName(), height }, '*');
}
```

### 10.2 变化监听

```javascript
const resize_observer = new ResizeObserver(() => postIframeHeight());
resize_observer.observe(body);

if (IS_BLOB_MODE) {
  const mutation_observer = new MutationObserver(() => {
    resize_observer.disconnect();
    for (const element of body.children) {
      resize_observer.observe(element);
    }
    postIframeHeight();
  });
  mutation_observer.observe(body, { childList: true, subtree: true, attributes: true });
}
```

**监听策略**:
- 所有模式：监听 `body` 的 `ResizeObserver`
- Blob 模式额外监听 `MutationObserver`

---

## 十一、视口高度同步

**文件**: `src/iframe/adjust_viewport.js`

```javascript
// 设置 CSS 变量
$('html').css('--TH-viewport-height', `${window.parent.innerHeight}px`);

// 监听父窗口 resize
window.addEventListener('message', function (event) {
  if (event.data?.type === 'TH_UPDATE_VIEWPORT_HEIGHT') {
    $('html').css('--TH-viewport-height', `${window.parent.innerHeight}px`);
  }
});
```

**用途**: 将父窗口的视口高度同步到 iframe 内，供 VH 转换使用。

---

## 十二、完整处理流程图

```
AI 输出文本
     ↓
[1] formatAsTavernRegexedString()
    ├─ getRegexedString()     → 应用 Tavern 正则
    ├─ substituteParams()     → 替换参数
    └─ macros.forEach()       → 宏替换
     ↓
[2] messageFormatting()       → 消息格式化
     ↓
[3] demacroOnRender()
    ├─ 检查宏是否存在
    ├─ 移除已存在的 iframe
    ├─ 替换 .mes_text HTML 中的宏
    └─ 替换 <code> 中的宏并重新高亮
     ↓
[4] SillyTavern 消息渲染到 DOM
     ↓
[5] render$mes() / useMessageIframeRuntimesStore
    ├─ 查找 <pre> 标签
    ├─ isFrontend() 检测
    ├─ 用 <div class="TH-render"> 包装
    └─ 根据 depth 计算渲染范围
     ↓
[6] useCollapseCodeBlock()
    ├─ collapse_code_block = 'none' → 不折叠
    ├─ collapse_code_block = 'frontend_only' → 只折叠前端代码
    └─ collapse_code_block = 'all' → 折叠所有
     ↓
[7] useOptimizeHljs()
    └─ 前端代码跳过 highlight.js 高亮
     ↓
[8] Iframe.vue 创建 iframe
    ├─ createSrcContent()
    │  ├─ replaceVhInContent()    → VH 转换
    │  ├─ 注入基础样式
    │  ├─ 注入第三方库
    │  └─ 注入全局变量
    └─ 加载内容 (srcdoc 或 Blob URL)
     ↓
[9] adjust_iframe_height.js
    ├─ ResizeObserver 监听
    ├─ MutationObserver 监听
    └─ postMessage 通知父窗口调整高度
     ↓
渲染完成
```

---

## 十三、关键文件索引

| 文件 | 用途 |
|------|------|
| `src/util/is_frontend.ts` | 前端代码检测 |
| `src/function/tavern_regex.ts` | Tavern 正则处理 |
| `src/function/macro_like.ts` | 宏定义与注册 |
| `src/panel/render/macro_like.ts` | 宏处理（渲染时） |
| `src/panel/render/iframe.ts` | iframe 内容构建 |
| `src/panel/render/Iframe.vue` | iframe 组件 |
| `src/panel/render/use_collapse_code_block.ts` | 代码折叠 |
| `src/panel/render/optimize_hljs.ts` | 语法高亮优化 |
| `src/store/iframe_runtimes/message.ts` | 消息渲染运行时 |
| `src/iframe/predefine.js` | 全局变量注入 |
| `src/iframe/adjust_iframe_height.js` | 高度调整 |
| `src/iframe/adjust_viewport.js` | 视口同步 |
| `src/iframe/third_party_message.html` | 第三方库 |

---

## 十四、注意事项

1. **isFrontend() 检测可能过于简单**：只检测三个标签，可能漏判很多前端代码
2. **仅处理 `<pre>` 标签**：其他位置的代码不会被渲染
3. **宏处理可能影响性能**：每次渲染都检查所有宏
4. **VH 转换只处理 `min-height`**：其他 vh 属性（如 `height`, `top`）不会被转换
5. **iframe 内无 SillyTavern DOM 访问权限**：需要通过 postMessage 通信
