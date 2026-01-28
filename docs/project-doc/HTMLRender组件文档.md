# HTMLRender 组件使用文档

## 简介

`HTMLRender` 是一个用于安全渲染用户生成的 HTML、CSS 和 JavaScript 内容的 Svelte 组件。它基于沙盒 Iframe 实现，提供了自动高度调整、视口同步以及常用前端库的预加载支持。

该组件的设计严格遵循 `docs/render.md` 规范，旨在兼容 SillyTavern 的前端代码块渲染逻辑。

## 引入方式

```svelte
<script>
    import HTMLRender from '$lib/components/render/HTMLRender.svelte';
</script>
```

## Props 参数

| 参数名 | 类型 | 默认值 | 说明 |
|Ref |Type |Default |Description |
|---|---|---|---|
| `content` | `string` | `''` | 需要渲染的原始 HTML/JS 代码字符串。 |
| `useBlobUrl` | `boolean` | `false` | 是否使用 Blob URL 模式渲染。默认为 `false` (使用 `srcdoc`)。Blob URL 模式在某些跨域场景下可能更受限，但在处理相对路径资源时可能有用。 |

## 内置支持

为了方便快速开发交互式内容，组件在 Iframe 中预注入了以下 CDN 库：

*   **样式库**:
    *   TailwindCSS (v3.4.1)
    *   Font Awesome (Free)
    *   jQuery UI Theme (Base)
*   **JS 库**:
    *   Vue 3 (Full Build, 支持模板编译)
    *   jQuery
    *   jQuery UI (+ Touch Punch)
    *   Pixi.js

此外，组件还内置了以下功能：
*   **Lodash**: `window._` (部分 Mock 或继承父窗口)
*   **SillyTavern Context**: `window.SillyTavern` (Mock 接口)
*   **VH 单位自动转换**: 自动将 `100vh` 转换为 Iframe 视口高度，避免撑破 Iframe。

## 使用示例

### 1. 基础 HTML & CSS

```svelte
<HTMLRender content={`
    <style>
        .box { padding: 20px; background: #f0f9ff; border: 1px solid #0ea5e9; border-radius: 8px; }
        h3 { color: #0284c7; margin-top: 0; }
    </style>
    <div class="box">
        <h3>Hello World</h3>
        <p>这是一个基础的 HTML 渲染示例。</p>
    </div>
`} />
```

### 2. 使用 TailwindCSS

无需额外配置，直接使用 Tailwind 类名即可。

```svelte
<HTMLRender content={`
    <div class="p-6 max-w-sm mx-auto bg-white rounded-xl shadow-lg flex items-center space-x-4">
        <div class="shrink-0">
            <div class="h-12 w-12 bg-blue-500 rounded-full flex items-center justify-center text-white font-bold">logo</div>
        </div>
        <div>
            <div class="text-xl font-medium text-black">ChitChat</div>
            <p class="text-slate-500">You have a new message!</p>
        </div>
    </div>
`} />
```

### 3. 使用 Vue 3 (交互式组件)

组件内置了 Vue 3 (完整版)，可以直接在 HTML 中编写 Vue 逻辑。

```svelte
<HTMLRender content={`
    <div id="app" class="p-4 border rounded bg-gray-50">
        <h3 class="font-bold mb-2">{{ message }}</h3>
        <button @click="count++" class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition">
            点击次数: {{ count }}
        </button>
    </div>

    <script>
        const { createApp, ref } = Vue;
        createApp({
            setup() {
                const count = ref(0);
                return { count, message: 'Vue 3 交互示例' }
            }
        }).mount('#app')
    <\/script>
`} />
```
*注意：在 JS 字符串中编写 `<script>` 标签时，建议转义为 `<\/script>` 以避免解析错误。*

## 注意事项

1.  **高度自适应**：
    *   组件会自动根据内容高度调整 Iframe 高度。
    *   **重要**：请避免在最外层容器设置固定的 `height` 或 `min-height: 100vh`，这可能导致高度计算错误（只能变大不能变小）。
    *   内容应尽量自然撑开文档流。

2.  **安全性**：
    *   Iframe 使用了 `sandbox` 属性，允许脚本执行 (`allow-scripts`) 但限制了部分高风险行为。
    *   请勿渲染不可信来源的恶意代码。

3.  **调试**：
    *   可以访问路由 `/debug/render` 查看组件的调试页面和更多示例。
