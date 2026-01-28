# JS-Slash-Runner 渲染机制完整实现 (独立可用版)

> 本文档包含完整的代码实现，可直接在其他项目中复用。

---

## 一、前端代码检测工具

### 1.1 检测函数 `isFrontend`

检测内容是否为前端代码（包含 HTML 结构）。

```typescript
/**
 * 检测字符串是否为前端代码
 * @param content - 要检测的内容
 * @returns true 表示包含前端代码
 */
export function isFrontend(content: string): boolean {
  return ['html>', '<head>', '<body'].some(tag => content.includes(tag));
}
```

**检测规则** (满足任一即返回 `true`):

| 检测项 | 匹配内容 | 说明 |
|--------|----------|------|
| `html>` | `</html>` 或 `html>` | HTML 闭合标签 |
| `<head>` | `<head>` | HEAD 开始标签 |
| `<body>` | `<body>` | BODY 开始标签 |

---

## 二、代码高亮处理

### 2.1 高亮函数 `highlight_code`

为代码块添加高亮效果和复制按钮。

```typescript
export function highlight_code(element: HTMLElement) {
  const $node = $(element);
  // 跳过已高亮的代码或前端代码
  if ($node.hasClass('hljs') || $node.text().includes('<body')) {
    return;
  }

  hljs.highlightElement(element);
  // 添加复制按钮
  $node.append(
    $(`<i class="fa-solid fa-copy code-copy interactable" title="Copy code"></i>`)
      .on('click', function (e) {
        e.stopPropagation();
      })
      .on('pointerup', async function () {
        navigator.clipboard.writeText($(element).text());
        toastr.info(t`已复制!`, '', { timeOut: 2000 });
      }),
  );
}
```

---

## 三、Tavern 正则处理

### 3.1 核心格式化函数 `formatAsTavernRegexedString`

处理 Tavern 正则脚本、参数替换和宏替换。

```typescript
type FormatAsTavernRegexedStringOption = {
  depth?: number;
  character_name?: string;
};

export function formatAsTavernRegexedString(
  text: string,
  source: 'user_input' | 'ai_output' | 'slash_command' | 'world_info' | 'reasoning',
  destination: 'display' | 'prompt',
  { depth, character_name }: FormatAsTavernRegexedStringOption = {},
) {
  // 1. 应用正则脚本
  let result = getRegexedString(
    text,
    (
      {
        user_input: regex_placement.USER_INPUT,
        ai_output: regex_placement.AI_OUTPUT,
        slash_command: regex_placement.SLASH_COMMAND,
        world_info: regex_placement.WORLD_INFO,
        reasoning: regex_placement.REASONING,
      } as const
    )[source],
    {
      characterOverride: character_name,
      isMarkdown: destination === 'display',
      isPrompt: destination === 'prompt',
      depth,
    },
  );

  // 2. 参数替换
  result = substituteParams(result, undefined, character_name, undefined, undefined);

  // 3. 宏替换
  macros.forEach(macro => {
    result = result.replace(macro.regex, (substring, ...args) =>
      macro.replace(
        {
          role: (
            {
              user_input: 'user',
              ai_output: 'assistant',
              slash_command: 'system',
              world_info: 'system',
              reasoning: 'system',
            } as const
          )[source],
          message_id: depth !== undefined ? chat.length - depth - 1 : undefined,
        },
        substring,
        ...args,
      ),
    );
  });
  return result;
}
```

---

## 四、宏定义系统

### 4.1 宏接口定义

```typescript
export interface MacroLike {
  regex: RegExp;
  replace: (context: MacroLikeContext, substring: string, ...args: any[]) => string;
}

export interface MacroLikeContext {
  message_id?: number;
  role?: 'user' | 'assistant' | 'system';
}

export const macros: MacroLike[] = [];
```

### 4.2 内置宏实现

```typescript
// 变量获取宏: {{get_variable::path}}
{
  regex: /\{\{get_(message|chat|character|preset|global)_variable::(.*?)\}\}/gi,
  replace: (
    context: MacroLikeContext,
    _substring: string,
    type: 'message' | 'chat' | 'character' | 'preset' | 'global',
    path: string,
  ) => {
    const variables = get_variables_without_clone(
      type !== 'message'
        ? { type }
        : {
            type,
            message_id:
              context.message_id ??
              chat.findLastIndex(message => _.isObject(message.variables?.[message.swipe_id ?? 0])),
          },
    );
    const value = omitDeepBy(_.get(variables, _.unescape(path), null), (_, key) => key.startsWith('$'));
    return typeof value === 'string' ? value : JSON.stringify(value);
  },
}

// 变量格式化宏: {{format_variable::path}}
{
  regex: /^(.*)\{\{format_(message|chat|character|preset|global)_variable::(.*?)\}\}/gim,
  replace: (
    context: MacroLikeContext,
    _substring: string,
    prefix: string,
    type: 'message' | 'chat' | 'character' | 'preset' | 'global',
    path: string,
  ) => {
    const variables = get_variables_without_clone(
      type !== 'message'
        ? { type }
        : {
            type,
            message_id:
              context.message_id ??
              chat.findLastIndex(message => _.isObject(message.variables?.[message.swipe_id ?? 0])),
          },
    );
    const value = omitDeepBy(_.get(variables, _.unescape(path), null), (_, key) => key.startsWith('$'));
    return (
      prefix +
      (typeof value === 'string' ? value : YAML.stringify(value, { blockQuote: 'literal' }).trimEnd()).replaceAll(
        '\n',
        '\n' + ' '.repeat(prefix.length),
      )
    );
  },
}
```

### 4.3 宏注册/注销

```typescript
export function registerMacroLike(
  regex: RegExp,
  replace: (context: MacroLikeContext, substring: string, ...args: any[]) => string,
): { unregister: () => void } {
  if (!macros.some(macro => macro.regex.source === regex.source)) {
    macros.push({ regex, replace });
  }
  return { unregister: () => unregisterMacroLike(regex) };
}

export function unregisterMacroLike(regex: RegExp) {
  const index = macros.findIndex(macro => macro.regex.source === regex.source);
  if (index !== -1) {
    macros.splice(index, 1);
  }
}
```

---

## 五、渲染时宏处理

### 5.1 渲染时宏替换函数 `demacroOnRender`

```typescript
function demacroOnRender($mes: JQuery<HTMLDivElement>) {
  const $mes_text = $mes.find('.mes_text');
  if (
    $mes_text.length === 0 ||
    !macros.some(macro => {
      macro.regex.lastIndex = 0;
      return macro.regex.test($mes_text.text());
    })
  ) {
    return;
  }

  const replace_html = (html: string) => {
    for (const macro of macros) {
      macro.regex.lastIndex = 0;
      html = html.replace(macro.regex, (substring: string, ...args: any[]) =>
        macro.replace({ role: $mes.attr('is_user') === 'true' ? 'user' : 'assistant' }, substring, ...args),
      );
    }
    return html;
  };

  // 移除额外渲染的 iframe
  $mes_text.find('.TH-render > iframe').remove();

  // 替换 HTML 中的宏
  $mes_text.html((_index, html) => replace_html(html));

  // 处理 <code> 中的宏
  $mes_text
    .find('code')
    .filter((_index, element) =>
      macros.some(macro => {
        macro.regex.lastIndex = 0;
        return macro.regex.test($(element).text());
      }),
    )
    .text((_index, text) => replace_html(text))
    .removeClass('hljs')
    .each((_index, element) => {
      highlight_code(element);
    });
}
```

### 5.2 事件监听设置

```typescript
export function useMacroLike(enabled: Readonly<Ref<boolean>>) {
  watch(enabled, (value, old_value) => {
    if (value !== old_value) {
      reloadAndRenderChatWithoutEvents();
    }
  });

  // 生成提示词后的宏处理
  eventSource.on(event_types.GENERATE_AFTER_DATA, (event_data: any, dry_run: boolean) => {
    if (enabled.value && !dry_run) {
      for (const message of event_data.prompt) {
        for (const macro of macros) {
          if (typeof message.content === 'string') {
            macro.regex.lastIndex = 0;
            message.content = message.content.replace(macro.regex, (substring: string, ...args: any[]) =>
              macro.replace({ role: message.role }, substring, ...args),
            );
          }
        }
      }
    }
  });

  // 聊天加载完成后处理所有消息
  eventSource.on('chatLoaded', () => {
    if (enabled.value) {
      $('#chat > .mes').each((_index, node) => {
        demacroOnRender($(node as HTMLDivElement));
      });
    }
  });

  // 消息渲染/更新时处理
  [
    event_types.CHARACTER_MESSAGE_RENDERED,
    event_types.USER_MESSAGE_RENDERED,
    event_types.MESSAGE_UPDATED,
    event_types.MESSAGE_SWIPED,
  ].forEach(event => {
    eventSource.on(event, (message_id: number | string) => {
      if (enabled.value) {
        demacroOnRender($(`#chat > .mes[mesid="${Number(message_id)}"]`));
      }
    });
  });
}
```

---

## 六、Iframe 内容构建

### 6.1 VH 单位转换函数 `replaceVhInContent`

将 `vh` 单位转换为 CSS 变量引用。

```typescript
function replaceVhInContent(content: string): string {
  const has_css_min_vh = /min-height\s*:\s*[^;{}]*\d+(?:\.\d+)?vh/gi.test(content);
  const has_inline_style_vh = /style\s*=\s*(["'])[\s\S]*?min-height\s*:\s*[^;]*?\d+(?:\.\d+)?vh[\s\S]*?\1/gi.test(
    content,
  );
  const has_js_vh =
    /(\.style\.minHeight\s*=\s*(["']))([\s\S]*?vh)(\2)/gi.test(content) ||
    /(setProperty\s*\(\s*(["'])min-height\2\s*,\s*(["']))([\s\S]*?vh)(\3\s*\))/gi.test(content);

  if (!has_css_min_vh && !has_inline_style_vh && !has_js_vh) {
    return content;
  }

  const convertVhToVariable = (value: string) =>
    value.replace(/(\d+(?:\.\d+)?)vh\b/gi, (match, value) => {
      const parsed = parseFloat(value);
      if (!isFinite(parsed)) {
        return match;
      }
      const VARIABLE_EXPRESSION = `var(--TH-viewport-height)`;
      if (parsed === 100) {
        return VARIABLE_EXPRESSION;
      }
      return `calc(${VARIABLE_EXPRESSION} * ${parsed / 100})`;
    });

  // CSS 声明块中的 min-height
  content = content.replace(
    /(min-height\s*:\s*)([^;]*?\d+(?:\.\d+)?vh)(?=\s*[;}])/gi,
    (_m, prefix: string, value: string) => {
      return `${prefix}${convertVhToVariable(value)}`;
    },
  );

  // 行内 style 属性中的 min-height
  content = content.replace(
    /(style\s*=\s*(["']))([^"'"]*?)(\2)/gi,
    (match, prefix: string, _quote: string, styleContent: string, suffix: string) => {
      if (!/min-height\s*:\s*[^;]*vh/i.test(styleContent)) return match;
      const replaced = styleContent.replace(
        /(min-height\s*:\s*)([^;]*?\d+(?:\.\d+)?vh)/gi,
        (_m, p1: string, p2: string) => {
          return `${p1}${convertVhToVariable(p2)}`;
        },
      );
      return `${prefix}${replaced}${suffix}`;
    },
  );

  // JavaScript 中的 .style.minHeight 赋值
  content = content.replace(
    /(\.style\.minHeight\s*=\s*(["']))([\s\S]*?)(\2)/gi,
    (match, prefix: string, _q: string, val: string, suffix: string) => {
      if (!/\b\d+(?:\.\d+)?vh\b/i.test(val)) return match;
      const converted = convertVhToVariable(val);
      return `${prefix}${converted}${suffix}`;
    },
  );

  // JavaScript 中的 setProperty 调用
  content = content.replace(
    /(setProperty\s*\(\s*(["'])min-height\2\s*,\s*(["']))([\s\S]*?)(\3\s*\))/gi,
    (match, prefix: string, _q1: string, _q2: string, val: string, suffix: string) => {
      if (!/\b\d+(?:\.\d+)?vh\b/i.test(val)) return match;
      const converted = convertVhToVariable(val);
      return `${prefix}${converted}${suffix}`;
    },
  );

  return content;
}
```

### 6.2 创建 iframe 内容 `createSrcContent`

```typescript
export function createSrcContent(content: string, use_blob_url: boolean) {
  content = replaceVhInContent(content);

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
${third_party}
<script src="${predefine_url}"></script>
<script src="https://testingcf.jsdelivr.net/gh/N0VI028/JS-Slash-Runner/src/iframe/node_modules/log.js"></script>
<script src="${adjust_viewport_url}"></script>
<script src="${adjust_iframe_height_url}"></script>
</head>
<body>
${content}
</body>
</html>
`;
}
```

**说明**:
- `third_party` - 第三方库注入 (见 6.3)
- `predefine_url` - 预定义脚本注入 (见 6.4)
- `adjust_viewport_url` - 视口同步脚本 (见 6.5)
- `adjust_iframe_height_url` - 高度调整脚本 (见 6.6)

### 6.3 第三方库注入 `third_party`

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

| 库 | 版本 | 用途 |
|------|------|------|
| Font Awesome | 6.x | 图标库 |
| TailwindCSS | 3.x | CSS 样式框架 |
| jQuery | 3.x | DOM 操作 |
| jQuery UI | 1.13.x | UI 组件、拖拽 |
| jQuery UI Touch Punch | - | 触摸支持 |
| Vue 3 Runtime | 3.x | 响应式 UI (无编译器) |
| Vue Router | 4.x | 路由 |
| Pixi.js | 7.x | 2D 图形/游戏引擎 |

### 6.4 预定义脚本 `predefine.js`

注入全局变量和 SillyTavern 上下文。

```javascript
// 继承父窗口的 lodash
window._ = window.parent._;

// SillyTavern 上下文
Object.defineProperty(window, 'SillyTavern', {
  get: () => _.get(window.parent, 'SillyTavern').getContext(),
});

// iframe ID 缓存
const iframeId = window.frameElement?.id || window.name;
if (iframeId) {
  window.__TH_IFRAME_ID = iframeId;
  if (!window.name) {
    window.name = iframeId;
  }
}

// 合并父窗口的全局对象
let result = _(window);
result = result.merge(_.pick(window.parent, ['EjsTemplate', 'TavernHelper', 'YAML', 'showdown', 'toastr', 'z']));
result = result.merge(_.omit(_.get(window.parent, 'TavernHelper'), '_bind'));
result = result.merge(
  ...Object.entries(_.get(window.parent, 'TavernHelper')._bind).map(([key, value]) => ({
    [key.replace('_', '')]: value.bind(window),
  })),
);
result.value();

// Mvu 状态管理
if (_.has(window.parent, 'Mvu')) {
  Object.defineProperty(window, 'Mvu', {
    get: () => _.get(window.parent, 'Mvu'),
    set: () => {},
    configurable: true,
  });
}

// 页面隐藏时清理事件监听
$(window).on('pagehide', () => {
  eventClearAll();
});
```

### 6.5 视口同步脚本 `adjust_viewport.js`

```javascript
$('html').css('--TH-viewport-height', `${window.parent.innerHeight}px`);
window.addEventListener('message', function (event) {
  if (event.data?.type === 'TH_UPDATE_VIEWPORT_HEIGHT') {
    $('html').css('--TH-viewport-height', `${window.parent.innerHeight}px`);
  }
});
```

### 6.6 高度调整脚本 `adjust_iframe_height.js`

```javascript
(function () {
  const IS_BLOB_MODE = window.location.protocol === 'blob:';

  let scheduled = false;

  function measureAndPost() {
    scheduled = false;
    try {
      const doc = window.document;
      const body = doc.body;
      const html = doc.documentElement;
      if (!body || !html) {
        return;
      }

      let height = 0;
      if (IS_BLOB_MODE) {
        // blob 模式: 使用子元素高度算法
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
            // 只计算正常文档流中占空间的元素
            if (position === 'absolute' || position === 'fixed') {
              continue;
            }

            const margin_top = parseFloat(style.marginTop) || 0;
            const margin_bottom = parseFloat(style.marginBottom) || 0;

            const top_with_margin = rect.top - margin_top - body_rect.top;
            const bottom_with_margin = rect.bottom + margin_bottom - body_rect.top;

            if (Number.isFinite(top_with_margin) && top_with_margin < max_top) {
              max_top = top_with_margin;
            }
            if (Number.isFinite(bottom_with_margin) && bottom_with_margin > max_bottom) {
              max_bottom = bottom_with_margin;
            }
          }

          if (Number.isFinite(max_top) && Number.isFinite(max_bottom) && max_bottom > max_top) {
            const content_height = max_bottom - max_top;
            const total_height = content_height + padding_top + padding_bottom;
            if (Number.isFinite(total_height) && total_height > 0) {
              height = total_height;
            }
          }
        }

        if (!Number.isFinite(height) || height <= 0) {
          height = body.scrollHeight;
        }
      } else {
        // srcdoc 模式
        height = body.scrollHeight;
      }

      if (!Number.isFinite(height) || height <= 0) {
        return;
      }

      window.parent.postMessage({ type: 'TH_ADJUST_IFRAME_HEIGHT', iframe_name: getIframeName(), height: height }, '*');
    } catch {
      //
    }
  }

  const throttledMeasureAndPost = _.throttle(measureAndPost, 500);

  function postIframeHeight() {
    if (scheduled) {
      return;
    }
    scheduled = true;

    if (typeof window.requestAnimationFrame === 'function') {
      window.requestAnimationFrame(measureAndPost);
    } else {
      throttledMeasureAndPost();
    }
  }

  function observeHeightChange() {
    const body = document.body;
    if (!body) {
      return;
    }

    const resize_observer = new ResizeObserver(entries => {
      postIframeHeight();
    });
    resize_observer.observe(body);

    if (IS_BLOB_MODE) {
      const mutation_observer = new MutationObserver(entries => {
        resize_observer.disconnect();

        for (const element of body.children) {
          resize_observer.observe(element);
        }
        resize_observer.observe(body);
        postIframeHeight();
      });
      mutation_observer.observe(body, { childList: true, subtree: true, attributes: true });
    }
  }

  $(() => {
    postIframeHeight();
    observeHeightChange();
  });
})();
```

---

## 七、消息渲染运行时

### 7.1 核心函数 `render$mes`

查找并包装前端代码块。

```typescript
type Runtime = { message_id: number; reload_memo: string; elements: HTMLElement[] };

function render$mes($mes: JQuery<HTMLElement>, reload_memo: string): Runtime[] {
  return _($mes.toArray())
    .map(div => {
      const message_id = Number($(div).attr('mesid'));
      const $element = $(div)
        .find('pre')
        .filter((_index, pre) => isFrontend($(pre).text()))
        .map((_index, pre) => {
          const $pre = $(pre);
          const $possible_div = $pre.parent('div.TH-render');
          if ($possible_div.length > 0) {
            return $possible_div[0];
          }
          $pre.wrap('<div class="TH-render">');
          return $pre.parent('div.TH-render')[0];
        });
      return { message_id, reload_memo, elements: $element.toArray() };
    })
    .filter(({ elements }) => elements.length > 0)
    .value();
}
```

### 7.2 事件驱动的运行时管理

```typescript
export const useMessageIframeRuntimesStore = defineStore('message_iframe_runtimes', () => {
  const global_settings = useGlobalSettingsStore();
  const runtimes = ref<Runtime[]>([]);

  // 监听设置变化
  watch(
    () => [global_settings.settings.render.enabled, global_settings.settings.render.depth] as const,
    ([new_enabled, new_depth]) => {
      if (new_enabled) {
        runtimes.value = auditRuntimes(runtimes.value, new_depth);
      } else {
        runtimes.value = [];
      }
    },
    { immediate: true },
  );

  // 聊天加载完成
  eventSource.on('chatLoaded', () => {
    if (global_settings.settings.render.enabled) {
      runtimes.value = renderMessages(calcToRender(global_settings.settings.render.depth), uuidv4());
    }
  });

  // 消息渲染/更新事件
  [
    event_types.CHARACTER_MESSAGE_RENDERED,
    event_types.USER_MESSAGE_RENDERED,
    event_types.MESSAGE_UPDATED,
    event_types.MESSAGE_SWIPED,
  ].forEach(event => {
    eventSource.on(event, (message_id: number | string) => {
      if (global_settings.settings.render.enabled) {
        const numbered_message_id = Number(message_id);
        runtimes.value = auditRuntimes(
          _.reject(runtimes.value, runtime => runtime.message_id === numbered_message_id),
          global_settings.settings.render.depth,
        );
      }
    });
  });

  // 消息删除
  eventSource.on(event_types.MESSAGE_DELETED, () => {
    if (global_settings.settings.render.enabled) {
      runtimes.value = auditRuntimes(runtimes.value, global_settings.settings.render.depth);
    }
  });

  // 重新加载所有
  const reloadAll = () => {
    const reload_memo = uuidv4();
    runtimes.value = runtimes.value.map(runtime => ({ ...runtime, reload_memo }));
  };

  return { runtimes, reloadAll };
});
```

---

## 八、完整执行流程图

```
┌─────────────────────────────────────────────────────────────────────┐
│ AI 原始输出文本                                                       │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [1] formatAsTavernRegexedString()                                   │
│     ├─ getRegexedString()     → 应用全局/角色正则脚本                │
│     ├─ substituteParams()     → 参数替换                            │
│     └─ macros.forEach()       → 宏替换                              │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [2] messageFormatting()  (SillyTavern 核心)                          │
│     ├─ Tavern 正则处理（第二轮）                                      │
│     ├─ 宏替换（第二轮）                                               │
│     └─ Markdown 渲染 (Showdown)                                      │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [3] DOM 注入                                                         │
│     $mes_html.find('.mes_text').append(messageFormatting(...))       │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [4] 代码高亮处理                                                     │
│     $mes_html.find('pre code').each(highlight_code)                  │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [5] CHARACTER_MESSAGE_RENDERED / USER_MESSAGE_RENDERED 事件触发      │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [6] demacroOnRender()                                               │
│     ├─ 移除已存在的 iframe                                           │
│     ├─ 替换 .mes_text 中的宏                                         │
│     └─ 替换 <code> 中的宏并重新高亮                                   │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [7] render$mes()                                                    │
│     ├─ 查找 <pre> 标签                                               │
│     ├─ isFrontend() 检测                                            │
│     └─ 用 <div class="TH-render"> 包装                              │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [8] Iframe 渲染                                                      │
│     ├─ createSrcContent()                                           │
│     │  ├─ replaceVhInContent()                                      │
│     │  ├─ 注入 CSS (Reset CSS + 头像)                               │
│     │  ├─ 注入第三方库                                               │
│     │  └─ 注入全局变量 (predefine.js)                               │
│     └─ 创建 iframe 加载内容                                          │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ [9] adjust_iframe_height.js                                         │
│     ├─ ResizeObserver 监听                                          │
│     ├─ MutationObserver 监听                                        │
│     └─ postMessage 通知父窗口调整高度                                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 九、已知限制与问题

1. **isFrontend() 检测过于简单**
   - 只检测 `html>`、`<head>`、`<body>`
   - 不检测 `<script>`、`<style>`、自定义标签等

2. **仅处理 `<pre>` 标签**
   - 其他位置的代码不会被检测
   - 行内代码无法被渲染

3. **宏处理可能影响性能**
   - 每次渲染都检查所有宏
   - 正则表达式的 `lastIndex` 需要手动重置

4. **VH 转换不完整**
   - 只处理 `min-height`
   - 不处理 `height`、`top`、`margin-top` 等

5. **iframe 内无 SillyTavern DOM 访问权限**
   - 需要通过 `postMessage` 通信
   - 无法直接操作父窗口 DOM
