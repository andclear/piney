# JSONL 聊天记录渲染流程文档

本文档详细说明了 Piney 系统中 JSONL 格式聊天记录从导入、解析到前端渲染的完整流程。该流程旨在严格复刻 SillyTavern 及 SillyReader 的渲染逻辑。

## 1. 核心流程概览

整个流程分为 **后端解析 (Rust)** 和 **前端渲染 (Svelte/TS)** 两个主要阶段：

1.  **后端解析**: 读取 JSONL，提取内容与元数据。
2.  **前端流水线 (Pipeline)**: 这是一个严格顺序的处理链：
    *   `Regex (Display)` -> `Tag Filter` -> `Tag Newline` -> `Markdown` -> `Sanitize`。
3.  **动态组件挂载**: 渲染 HTML 后，检测 `<html>` 代码块并动态挂载沙箱 Iframe (TavernHelper 逻辑)。

---

## 2. 后端解析与数据获取

**文件路径**: `src/api/history.rs`

后端主要负责文件的物理读取、分页和初步的元数据提取。

### 2.1 JSONL 解析
后端使用 `serde_json` 逐行解析 JSONL 文件。这确保了标准的 JSON 结构被正确读取。

### 2.2 全局标签检测
为了支持前端的“阅读设置”菜单显示所有可用标签，后端会在读取内容时对**整个文件内容**进行一次正则扫描。
*   **正则**: `Regex::new(r"</?([a-zA-Z0-9_\-\.\u4e00-\u9fa5]+)(?:\s[^>]*)?>")`
*   **作用**: 扫描所有 XML 风格的标签，返回去重后的标签列表 (`detected_tags`)。

---

## 3. 前端渲染管线 (Rendering Pipeline)

**文件路径**: 
- Pipeline: `frontend/src/lib/utils/renderUtils.ts` (`processTextWithPipeline`)
- Orchestrator: `frontend/src/routes/(app)/characters/[id]/history/+page.svelte`

### 3.1 流水线步骤 (Sequential Steps)

#### 步骤 1: 正则脚本应用 (Regex Scripts)
*   **核心**: `processContentWithScripts` (`lib/utils/regexProcessor.ts`)
*   **逻辑**: 这里只应用 `markdownOnly` (Display) 或 通用正则。`promptOnly` 的正则会被跳过。
*   **顺序**: 先应用 **Chat Regex**，后应用 **Character Regex**。

#### 步骤 2: 智能标签过滤 (Smart Tag Filtering)
*   **核心**: `smartFilterTags` (`lib/utils/tagFilter.ts`)
*   **输入**: 用户在“设置”中**取消勾选**的标签。
*   **逻辑**: 识别成对标签 (`<tag>...</tag>`) 和未闭合标签，移除其内容。

#### 步骤 3: 标签分行处理 (Newline Processing)
*   **核心**: `processTagNewlines` (`lib/utils/tagFilter.ts`)
*   **输入**: 用户**勾选**了“分行显示”的标签。
*   **逻辑**: 将被选中标签内容里的 `\n` 转换为 `<br>`，未选中的转换为 ` ` (空格)。此步骤包含对代码块的保护，防止破坏代码格式。

#### 步骤 4: Markdown 渲染
*   **引擎**: `marked` (npm: marked)
*   **配置**: `{ breaks: true, async: false }`。复刻 SillyTavern 的 GitHub Flavored Markdown 渲染行为。
*   **代码块**: Markdown 引擎会自动将 ` ``` ` 转换为 `<pre><code>...</code></pre>`。

#### 步骤 5: 安全清洗 (Strict Sanitization)
*   **核心**: `strictSanitize` (`lib/utils/renderUtils.ts`)
*   **目的**: 移除 `script`, `iframe`, `object`, `style`, `form` 以及所有 `on*` 事件和 `javascript:` 链接。
*   **例外**: 只有通过后续步骤 (3.2) 挂载的 Iframe 允许执行脚本，主文本区域严格禁止脚本。

---

## 3.2 动态 Iframe 挂载 (Frontend Code Execution)

**文件路径**: `frontend/src/lib/components/render/Iframe.svelte`

为了支持类似 TavernHelper 的“前端代码预览”功能，我们在主文本渲染后执行额外的挂载步骤：

1.  **Svelte Action**: `mountCodeBlockIframes` (在 `+page.svelte` 中定义)
2.  **检测**: 监听 DOM 变化，查找所有 `<pre><code>` 元素。
3.  **判断**: 检查代码块内容是否包含 `<html>` 等前端特征 (`isFrontend`)。
4.  **挂载**:
    *   如果判断为前端代码，创建一个容器 `div.th-iframe-wrapper` 替换原有的 `<pre>`。
    *   使用 Svelte 的 `mount()` API 动态挂载 `Iframe.svelte` 组件。
    *   **Iframe**: 使用 `srcdoc` 和 `sandbox="allow-scripts"` 隔离执行环境。
5.  **通信**: Iframe 内部包含 `resize` 脚本，通过 `postMessage` 向父级报告高度，实现自适应高度。

---

## 4. 关键代码文件索引

| 功能模组 | 文件路径 | 说明 |
| :--- | :--- | :--- |
| **页面入口** | `frontend/src/routes/(app)/characters/[id]/history/+page.svelte` | 协调渲染、挂载 Iframe |
| **渲染管线** | `frontend/src/lib/utils/renderUtils.ts` | 包含 Pipeline、Sanitizer、Iframe HTML 生成 |
| **Iframe组件** | `frontend/src/lib/components/render/Iframe.svelte` | 沙箱容器，处理通信 |
| **正则/标签** | `frontend/src/lib/utils/regexProcessor.ts` / `tagFilter.ts` | 文本预处理逻辑 |

## 5. CSS 样式支持
主界面使用 Tailwind CSS + 全局样式 (`app.css` 或 `style` 块) 适配 SillyReader 风格：
*   **深色模式**: 自动适配。
*   **Markdown元素**: 为 `code`, `pre`, `q`, `em`, `strong` 定义了特定的阅读样式。
