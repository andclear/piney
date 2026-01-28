# AI 对话配置及功能实现文档

> 基于 Cherry Studio 项目的 OpenAI 兼容部分实现分析

## 目录

1. [核心架构概览](#核心架构概览)
2. [模型列表获取](#模型列表获取)
3. [模型可用性检测](#模型可用性检测)
4. [聊天对话补全](#聊天对话补全)
5. [消息格式转换](#消息格式转换)
6. [类型定义参考](#类型定义参考)

---

## 核心架构概览

### 项目结构

```
Cherry Studio
├── src/
│   ├── main/                          # Electron 主进程
│   │   └── apiServer/                 # 内置 API 服务器
│   │       ├── services/
│   │       │   ├── models.ts          # 模型列表服务
│   │       │   └── chat-completion.ts # 聊天补全服务
│   │       ├── routes/
│   │       │   └── chat.ts            # 聊天路由
│   │       └── utils/
│   │           └── index.ts           # 工具函数
│   ├── renderer/                      # 渲染进程 (React)
│   │   └── src/
│   │       ├── services/
│   │       │   ├── ApiService.ts      # API 调用服务
│   │       │   └── HealthCheckService.ts  # 健康检查服务
│   │       └── aiCore/                # AI 核心中间件
│   │           └── prepareParams/     # 参数构建
│   └── preload/                       # IPC 桥接
└── packages/
    └── ai-sdk-provider/               # AI SDK 提供商
```

### Provider 模型定义

```typescript
// 核心 Provider 接口
interface Provider {
  id: string;           // 唯一标识 (如 "openai", "cherryin")
  type: 'openai' | 'anthropic' | 'ollama' | ...;
  name: string;         // 显示名称
  apiKey: string;       // API 密钥
  apiHost: string;      // API 地址
  enabled: boolean;     // 是否启用
  models?: Model[];     // 模型列表
  anthropicApiHost?: string;  // Anthropic API 地址
  apiOptions?: {
    isNotSupportArrayContent?: boolean;
    isSupportDeveloperRole?: boolean;
    isNotSupportStreamOptions?: boolean;
    isNotSupportEnableThinking?: boolean;
    // ...更多选项
  };
}

interface Model {
  id: string;           // 模型 ID (如 "gpt-4")
  name: string;         // 显示名称
  provider: string;     // 所属 Provider ID
  owned_by?: string;    // 拥有者
  endpoint_type?: string;  // 端点类型
  // ...其他模型属性
}
```

### 模型 ID 命名规范

**格式**: `providerId:modelId` (如 `openai:gpt-4`, `cherryin:claude-sonnet-4-20250514`)

```typescript
// 从完整 ID 提取真实模型 ID
function getRealProviderModel(modelStr: string): string {
  return modelStr.split(':').slice(1).join(':');
}

// 示例
const fullModelId = "cherryin:claude-sonnet-4-20250514";
const modelId = getRealProviderModel(fullModelId); // "claude-sonnet-4-20250514"
const providerId = fullModelId.split(':')[0]; // "cherryin"
```

---

## 模型列表获取

### 核心流程

```
获取模型列表流程:

1. 从 Redux Store 获取所有 Provider
   └─ reduxService.select('state.llm.providers')

2. 过滤启用的 OpenAI/Anthropic 类型 Provider
   └─ type === 'openai' || type === 'anthropic'

3. 收集所有 Provider 的模型
   └─ providers.map(p => p.models || []).flat()

4. 转换为 OpenAI 兼容格式
   └─ transformModelToOpenAI(model, provider)

5. 去重 (按 provider:model_id)
   └─ 使用 Map 去重

6. 分页和筛选
   └─ 支持 offset/limit/providerType 筛选
```

### 核心代码实现

```typescript
// src/main/apiServer/services/models.ts

import { getAvailableProviders, listAllAvailableModels, transformModelToOpenAI } from '../utils';

class ModelsService {
  async getModels(filter: ModelsFilter): Promise<ApiModelsResponse> {
    // 1. 获取可用的 Provider
    let providers = await getAvailableProviders();

    // 2. 按 Provider 类型筛选
    if (filter.providerType === 'anthropic') {
      providers = providers.filter((p) => p.type === 'anthropic' || !isEmpty(p.anthropicApiHost?.trim()));
    }

    // 3. 获取所有模型
    const models = await listAllAvailableModels(providers);

    // 4. 去重并转换格式
    const uniqueModels = new Map<string, ApiModel>();
    for (const model of models) {
      const provider = providers.find((p) => p.id === model.provider);
      if (!provider) continue;

      const openAIModel = transformModelToOpenAI(model, provider);
      const fullModelId = openAIModel.id; // 格式: "provider:model_id"

      if (!uniqueModels.has(fullModelId)) {
        uniqueModels.set(fullModelId, openAIModel);
      }
    }

    // 5. 分页
    let modelData = Array.from(uniqueModels.values());
    const offset = filter?.offset || 0;
    const limit = filter?.limit;
    if (limit !== undefined) {
      modelData = modelData.slice(offset, offset + limit);
    }

    return {
      object: 'list',
      data: modelData,
      total: modelData.length,
      offset,
      limit
    };
  }
}

export const modelsService = new ModelsService();
```

### Provider 工具函数

```typescript
// src/main/apiServer/utils/index.ts

// 缓存配置
const PROVIDERS_CACHE_KEY = 'api-server:providers';
const PROVIDERS_CACHE_TTL = 10 * 1000; // 10秒缓存

// 获取可用的 Provider (带缓存)
export async function getAvailableProviders(): Promise<Provider[]> {
  // 1. 尝试从缓存获取
  const cached = CacheService.get<Provider[]>(PROVIDERS_CACHE_KEY);
  if (cached && cached.length > 0) {
    return cached;
  }

  // 2. 从 Redux Store 获取
  const providers = await reduxService.select('state.llm.providers');

  // 3. 过滤支持的 Provider
  const supportedProviders = providers.filter(
    (p: Provider) => p.enabled && (p.type === 'openai' || p.type === 'anthropic')
  );

  // 4. 缓存结果
  CacheService.set(PROVIDERS_CACHE_KEY, supportedProviders, PROVIDERS_CACHE_TTL);

  return supportedProviders;
}

// 收集所有可用模型
export async function listAllAvailableModels(providers?: Provider[]): Promise<Model[]> {
  if (!providers) {
    providers = await getAvailableProviders();
  }
  return providers.map((p: Provider) => p.models || []).flat();
}

// 转换为 OpenAI 格式
export function transformModelToOpenAI(model: Model, provider?: Provider): ApiModel {
  return {
    id: `${model.provider}:${model.id}`,  // 完整模型 ID
    object: 'model',
    name: model.name,
    created: Math.floor(Date.now() / 1000),
    owned_by: model.owned_by || provider?.name || model.provider,
    provider: model.provider,
    provider_name: provider?.name,
    provider_type: provider?.type,
    provider_model_id: model.id
  };
}
```

### API 响应格式

```typescript
// OpenAI 兼容的模型列表响应格式
interface ApiModelsResponse {
  object: 'list';
  data: ApiModel[];
  total?: number;      // 总是返回总数
  offset?: number;     // 偏移量
  limit?: number;      // 限制数量
}

interface ApiModel {
  id: string;              // "provider:model_id"
  object: 'model';
  created: number;
  name: string;
  owned_by: string;
  provider?: string;       // Provider ID
  provider_name?: string;  // Provider 显示名称
  provider_type?: string;  // Provider 类型
  provider_model_id?: string;  // 原始模型 ID
}
```

---

## 模型可用性检测

### 检测流程

```
模型健康检查流程:

1. 获取 Provider 的所有 API Key
   └─ 支持多 Key 用逗号分隔
   └─ 本地模型可传入空字符串

2. 对每个模型尝试调用 API
   └─ 并发或顺序执行
   └─ 记录延迟时间

3. 判断结果
   └─ SUCCESS: API 调用成功
   └─ FAILED: API 调用失败
   └─ 计算平均延迟

4. 返回检测结果
   └─ 每个模型的状态
   └─ 每个 API Key 的状态
```

### 核心代码实现

```typescript
// src/renderer/src/services/ApiService.ts

/**
 * 检查单个模型的可用性
 * @returns 包含延迟时间的成功标志
 */
export async function checkModel(
  provider: Provider,
  model: Model,
  timeout = 15000
): Promise<{ latency: number }> {
  const startTime = performance.now();
  await checkApi(provider, model, timeout);
  return { latency: performance.now() - startTime };
}

/**
 * 内部 API 检查实现
 */
export async function checkApi(
  provider: Provider,
  model: Model,
  timeout = 15000
): Promise<void> {
  // 1. 验证必填字段
  checkApiProvider(provider);

  const ai = new AiProviderNew(model, provider);

  // 2. 判断模型类型
  if (isEmbeddingModel(model)) {
    // 嵌入模型：获取维度
    const timerPromise = new Promise((_, reject) =>
      setTimeout(() => reject('Timeout'), timeout)
    );
    await Promise.race([ai.getEmbeddingDimensions(model), timerPromise]);
  } else {
    // 普通聊天模型：发送测试消息
    const assistant = getDefaultAssistant();
    assistant.model = model;
    assistant.prompt = 'test';

    const params: StreamTextParams = {
      system: assistant.prompt,
      prompt: 'hi',
      abortSignal: signal
    };

    await ai.completions(model.id, params, config);
  }
}

/**
 * 验证 Provider 必填字段
 */
export function checkApiProvider(provider: Provider): void {
  // 不需要 API Key 的 Provider
  const isExcludedProvider =
    (isSystemProvider(provider) && NOT_SUPPORT_API_KEY_PROVIDERS.includes(provider.id)) ||
    NOT_SUPPORT_API_KEY_PROVIDER_TYPES.includes(provider.type);

  if (!isExcludedProvider) {
    if (!provider.apiKey) {
      throw new Error('API Key is required');
    }
  }

  if (!provider.apiHost && provider.type !== 'vertexai') {
    throw new Error('API Host is required');
  }

  if (isEmpty(provider.models)) {
    throw new Error('Models are required');
  }
}
```

### 健康检查服务

```typescript
// src/renderer/src/services/HealthCheckService.ts

import { checkModel } from './ApiService';

interface ApiKeyWithStatus {
  key: string;
  status: 'success' | 'failed';
  latency: number;
  error?: string;
}

interface ModelWithStatus {
  model: Model;
  status: 'success' | 'failed' | 'not_checked';
  latency?: number;
  keyResults: ApiKeyWithStatus[];
}

/**
 * 使用多个 API Key 检查单个模型的连通性
 */
export async function checkModelWithMultipleKeys(
  provider: Provider,
  model: Model,
  apiKeys: string[],
  timeout?: number
): Promise<ApiKeyWithStatus[]> {
  const checkPromises = apiKeys.map(async (key) => {
    const startTime = Date.now();
    await checkModel({ ...provider, apiKey: key }, model, timeout);
    const latency = Date.now() - startTime;

    return {
      key,
      status: HealthStatus.SUCCESS,
      latency
    };
  });

  const results = await Promise.allSettled(checkPromises);

  return results.map((result, index) => {
    if (result.status === 'fulfilled') {
      return result.value;
    }
    return {
      key: apiKeys[index],
      status: HealthStatus.FAILED,
      error: formatErrorMessage(result.reason)
    };
  });
}

/**
 * 并发或顺序检查多个模型
 */
export async function checkModelsHealth(
  options: {
    provider: Provider;
    models: Model[];
    apiKeys: string[];
    isConcurrent: boolean;  // true: 并发, false: 顺序
    timeout?: number;
  },
  onModelChecked?: (result: ModelWithStatus, index: number) => void
): Promise<ModelWithStatus[]> {
  const { provider, models, apiKeys, isConcurrent, timeout } = options;

  const modelPromises = models.map(async (model, index) => {
    const keyResults = await checkModelWithMultipleKeys(provider, model, apiKeys, timeout);
    const analysis = aggregateApiKeyResults(keyResults);

    const result: ModelWithStatus = {
      model,
      status: analysis.status,
      latency: analysis.latency,
      keyResults
    };

    onModelChecked?.(result, index);
    return result;
  });

  if (isConcurrent) {
    await Promise.all(modelPromises);
  } else {
    for (const promise of modelPromises) {
      await promise;
    }
  }

  return results;
}
```

### React Hook 使用示例

```typescript
// src/renderer/src/pages/settings/ProviderSettings/ModelList/useHealthCheck.ts

import { checkModelsHealth } from '@renderer/services/HealthCheckService';
import { splitApiKeyString } from '@renderer/utils/api';
import { useCallback, useState } from 'react';

export const useHealthCheck = (provider: Provider, models: Model[]) => {
  const [modelStatuses, setModelStatuses] = useState<ModelWithStatus[]>([]);
  const [isChecking, setIsChecking] = useState(false);

  const runHealthCheck = useCallback(async () => {
    const modelsToCheck = models.filter((model) => !isRerankModel(model));

    // 1. 解析 API Keys (支持多个 Key 用逗号分隔)
    const keys = splitApiKeyString(provider.apiKey);
    if (keys.length === 0) {
      keys.push('');  // 本地模型支持空 Key
    }

    // 2. 弹出配置对话框
    const result = await HealthCheckPopup.show({
      title: '健康检查',
      provider,
      apiKeys: keys
    });

    if (result.cancelled) return;

    // 3. 初始化状态
    const initialStatuses: ModelWithStatus[] = modelsToCheck.map((model) => ({
      model,
      checking: true,
      status: HealthStatus.NOT_CHECKED,
      keyResults: []
    }));
    setModelStatuses(initialStatuses);
    setIsChecking(true);

    // 4. 执行健康检查
    const checkResults = await checkModelsHealth(
      {
        provider,
        models: modelsToCheck,
        apiKeys: result.apiKeys,
        isConcurrent: result.isConcurrent,
        timeout: result.timeout
      },
      (checkResult, index) => {
        setModelStatuses((current) => {
          const updated = [...current];
          if (updated[index]) {
            updated[index] = { ...updated[index], ...checkResult, checking: false };
          }
          return updated;
        });
      }
    );

    setIsChecking(false);
  }, [models, provider]);

  return { isChecking, modelStatuses, runHealthCheck };
};
```

---

## 聊天对话补全

### 核心流程

```
聊天请求处理流程:

1. 接收请求参数
   └─ messages: 消息数组
   └─ model: 模型 ID (provider:model_id 格式)
   └─ stream: 是否流式输出
   └─ 其他参数 (temperature, maxTokens 等)

2. 验证请求
   └─ 验证 messages 数组
   └─ 验证 model 格式

3. 解析 Provider 和模型
   └─ 从 model ID 提取 providerId 和 modelId
   └─ 验证 provider 是否存在且启用

4. 创建 OpenAI 客户端
   └─ 使用 provider 的 apiHost 和 apiKey

5. 发起 API 请求
   └─ 流式: 返回 AsyncIterable
   └─ 非流式: 返回完整响应

6. 错误处理
   └─ 401: API Key 无效
   └─ 429: 超出速率限制
   └─ 502: 上游错误
```

### 核心代码实现

```typescript
// src/main/apiServer/services/chat-completion.ts

import OpenAI from '@cherrystudio/openai';
import type { ChatCompletionCreateParams } from '@cherrystudio/openai/resources';

class ChatCompletionService {
  /**
   * 解析 Provider 上下文
   */
  async resolveProviderContext(model: string): Promise<{
    ok: boolean;
    provider: Provider;
    modelId: string;
    client: OpenAI;
  }> {
    // 1. 验证模型 ID 格式
    const modelValidation = await validateModelId(model);
    if (!modelValidation.valid) {
      return { ok: false, error: modelValidation.error! };
    }

    const provider = modelValidation.provider!;
    const modelId = modelValidation.modelId!;

    // 2. 检查 Provider 类型
    if (provider.type !== 'openai') {
      return {
        ok: false,
        error: {
          type: 'unsupported_provider_type',
          message: `Provider '${provider.id}' of type '${provider.type}' is not supported`,
          code: 'unsupported_provider_type'
        }
      };
    }

    // 3. 创建 OpenAI 客户端
    const client = new OpenAI({
      baseURL: provider.apiHost,
      apiKey: provider.apiKey
    });

    return { ok: true, provider, modelId, client };
  }

  /**
   * 准备请求参数
   */
  async prepareRequest(
    request: ChatCompletionCreateParams,
    stream: boolean
  ): Promise<PrepareRequestResult> {
    // 1. 验证请求
    const validation = this.validateRequest(request);
    if (!validation.isValid) {
      return { status: 'validation_error', errors: validation.errors };
    }

    // 2. 解析 Provider
    const providerContext = await this.resolveProviderContext(request.model!);
    if (!providerContext.ok) {
      return { status: 'model_error', error: providerContext.error };
    }

    const { provider, modelId, client } = providerContext;

    // 3. 构建 Provider 请求
    return {
      status: 'ok',
      provider,
      modelId,
      client,
      providerRequest: stream
        ? { ...request, model: modelId, stream: true as const }
        : { ...request, model: modelId, stream: false as const }
    };
  }

  /**
   * 验证请求参数
   */
  validateRequest(request: ChatCompletionCreateParams): ValidationResult {
    const errors: string[] = [];

    if (!request.messages) {
      errors.push('Messages array is required');
    } else if (!Array.isArray(request.messages)) {
      errors.push('Messages must be an array');
    } else if (request.messages.length === 0) {
      errors.push('Messages array cannot be empty');
    } else {
      request.messages.forEach((message, index) => {
        if (!message.role) {
          errors.push(`Message ${index}: role is required`);
        }
        if (!message.content) {
          errors.push(`Message ${index}: content is required`);
        }
      });
    }

    return { isValid: errors.length === 0, errors };
  }

  /**
   * 处理非流式补全请求
   */
  async processCompletion(request: ChatCompletionCreateParams) {
    const preparation = await this.prepareRequest(request, false);

    if (preparation.status === 'validation_error') {
      throw new ChatCompletionValidationError(preparation.errors);
    }
    if (preparation.status === 'model_error') {
      throw new ChatCompletionModelError(preparation.error);
    }

    const { client, providerRequest } = preparation;

    // 调用 OpenAI API
    const response = await client.chat.completions.create(
      providerRequest
    ) as OpenAI.Chat.Completions.ChatCompletion;

    return { response };
  }

  /**
   * 处理流式补全请求
   */
  async processStreamingCompletion(request: ChatCompletionCreateParams) {
    const preparation = await this.prepareRequest(request, true);

    if (preparation.status === 'validation_error') {
      throw new ChatCompletionValidationError(preparation.errors);
    }
    if (preparation.status === 'model_error') {
      throw new ChatCompletionModelError(preparation.error);
    }

    const { client, providerRequest } = preparation;

    const stream = await client.chat.completions.create(
      providerRequest as ChatCompletionCreateParamsStreaming
    ) as AsyncIterable<OpenAI.Chat.Completions.ChatCompletionChunk>;

    return { stream };
  }
}

export const chatCompletionService = new ChatCompletionService();
```

### API 路由

```typescript
// src/main/apiServer/routes/chat.ts

import express from 'express';
import { chatCompletionService } from '../services/chat-completion';

const router = express.Router();

/**
 * POST /v1/chat/completions
 *
 * OpenAI 兼容的聊天补全接口
 */
router.post('/completions', async (req, res) => {
  const request = req.body;

  if (!request) {
    return res.status(400).json({
      error: {
        message: 'Request body is required',
        type: 'invalid_request_error',
        code: 'missing_body'
      }
    });
  }

  const isStreaming = !!request.stream;

  if (isStreaming) {
    // 流式响应
    const { stream } = await chatCompletionService.processStreamingCompletion(request);

    res.setHeader('Content-Type', 'text/event-stream; charset=utf-8');
    res.setHeader('Cache-Control', 'no-cache, no-transform');
    res.setHeader('Connection', 'keep-alive');

    try {
      for await (const chunk of stream) {
        res.write(`data: ${JSON.stringify(chunk)}\n\n`);
      }
      res.write('data: [DONE]\n\n');
    } catch (error) {
      res.write(`data: ${JSON.stringify({ error: { message: 'Stream error' } })}\n\n`);
    } finally {
      res.end();
    }
    return;
  }

  // 非流式响应
  const { response } = await chatCompletionService.processCompletion(request);
  return res.json(response);
});

export { router as chatRoutes };
```

---

## 消息格式转换

### 消息结构

```typescript
// 原始消息格式 (src/renderer/src/types/newMessage.ts)

interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  blocks?: MessageBlock[];
  // ...其他属性
}

type MessageBlock =
  | TextMessageBlock
  | ImageMessageBlock
  | FileMessageBlock
  | ThinkingMessageBlock
  | ToolCallMessageBlock
  | ToolResultMessageBlock;

// OpenAI 格式转换
interface OpenAIMessage {
  role: 'user' | 'assistant' | 'system';
  content: string | Array<TextPart | ImagePart | FilePart>;
}

interface TextPart {
  type: 'text';
  text: string;
}

interface ImagePart {
  type: 'image';
  image: string;  // base64 或 URL
  mediaType?: string;
}

interface FilePart {
  type: 'file';
  file_data: string;  // fileid://xxx 或 base64
}
```

### 消息转换核心代码

```typescript
// src/renderer/src/aiCore/prepareParams/messageConverter.ts

import { convertFileBlockToFilePart, convertFileBlockToTextPart } from './fileProcessor';

/**
 * 将消息转换为 AI SDK 参数格式
 */
export async function convertMessageToSdkParam(
  message: Message,
  isVisionModel = false,
  model?: Model
): Promise<ModelMessage | ModelMessage[]> {
  const content = getMainTextContent(message);
  const fileBlocks = findFileBlocks(message);
  const imageBlocks = findImageBlocks(message);

  if (message.role === 'user' || message.role === 'system') {
    return convertMessageToUserModelMessage(content, fileBlocks, imageBlocks, isVisionModel, model);
  }
  return convertMessageToAssistantModelMessage(content, fileBlocks, [], model);
}

/**
 * 转换为用户消息
 */
async function convertMessageToUserModelMessage(
  content: string,
  fileBlocks: FileMessageBlock[],
  imageBlocks: ImageMessageBlock[],
  isVisionModel = false,
  model?: Model
): Promise<UserModelMessage | UserModelMessage[]> {
  const parts: Array<TextPart | FilePart | ImagePart> = [];

  // 1. 添加文本内容
  if (content) {
    parts.push({ type: 'text', text: content });
  }

  // 2. 处理图片 (仅视觉模型)
  if (isVisionModel) {
    for (const imageBlock of imageBlocks) {
      if (imageBlock.file) {
        const image = await window.api.file.base64Image(imageBlock.file.id + imageBlock.file.ext);
        parts.push({
          type: 'image',
          image: image.base64,
          mediaType: image.mime
        });
      } else if (imageBlock.url) {
        parts.push({ type: 'image', image: imageBlock.url });
      }
    }
  }

  // 3. 处理文件
  for (const fileBlock of fileBlocks) {
    const file = fileBlock.file;

    // 优先尝试原生文件支持
    if (model) {
      const filePart = await convertFileBlockToFilePart(fileBlock, model);
      if (filePart) {
        // 如果是文件 ID 引用，拆分为 system + user
        if (typeof filePart.data === 'string' && filePart.data.startsWith('fileid://')) {
          return [
            { role: 'system', content: filePart.data },
            { role: 'user', content: parts.length > 0 ? parts : '' }
          ];
        }
        parts.push(filePart);
        continue;
      }
    }

    // 回退到文本提取
    const textPart = await convertFileBlockToTextPart(fileBlock);
    if (textPart) {
      parts.push(textPart);
    }
  }

  return { role: 'user', content: parts };
}

/**
 * 转换为助手消息
 */
async function convertMessageToAssistantModelMessage(
  content: string,
  fileBlocks: FileMessageBlock[],
  thinkingBlocks: ThinkingMessageBlock[],
  model?: Model
): Promise<AssistantModelMessage> {
  const parts: Array<TextPart | ReasoningPart | FilePart> = [];

  if (content) {
    parts.push({ type: 'text', text: content });
  }

  // 添加思考内容
  for (const thinkingBlock of thinkingBlocks) {
    parts.push({ type: 'reasoning', text: thinkingBlock.content });
  }

  // 处理文件
  for (const fileBlock of fileBlocks) {
    const filePart = await convertFileBlockToFilePart(fileBlock, model);
    if (filePart) {
      parts.push(filePart);
    }
  }

  return { role: 'assistant', content: parts };
}
```

### 参数构建

```typescript
// src/renderer/src/aiCore/prepareParams/parameterBuilder.ts

import { buildProviderOptions } from '../utils/options';
import { getMaxTokens, getTemperature, getTopP } from './modelParameters';

export async function buildStreamTextParams(
  messages: Message[],
  assistant: Assistant,
  provider: Provider,
  options: {
    mcpTools?: MCPTool[];
    webSearchProviderId?: string;
    requestOptions?: {
      signal?: AbortSignal;
      timeout?: number;
      headers?: Record<string, string>;
    };
  }
): Promise<{
  params: StreamTextParams;
  modelId: string;
  capabilities: {
    enableReasoning: boolean;
    enableWebSearch: boolean;
    enableGenerateImage: boolean;
    enableUrlContext: boolean;
  };
}> {
  const model = assistant.model || getDefaultModel();

  // 1. 构建 Provider Options
  const { providerOptions, standardParams } = buildProviderOptions(assistant, model, provider, {
    enableReasoning: isSupportedThinkingTokenModel(model),
    enableWebSearch: isWebSearchModel(model),
    enableGenerateImage: isGenerateImageModel(model)
  });

  // 2. 设置工具 (MCP)
  const tools = setupToolsConfig(options.mcpTools);

  // 3. 构建基础参数
  const params: StreamTextParams = {
    messages: sdkMessages,
    maxOutputTokens: getMaxTokens(assistant, model),
    temperature: getTemperature(assistant, model),
    topP: getTopP(assistant, model),
    ...standardParams,
    abortSignal: options.requestOptions?.signal,
    headers: options.requestOptions?.headers,
    providerOptions,
    stopWhen: stepCountIs(20),
    maxRetries: 0
  };

  if (tools) {
    params.tools = tools;
  }

  if (assistant.prompt) {
    params.system = assistant.prompt;
  }

  return { params, modelId: model.id, capabilities };
}
```

### 完整的请求 JSON 示例

```json
{
  "model": "cherryin:claude-sonnet-4-20250514",
  "messages": [
    {
      "role": "system",
      "content": "你是一个有用的助手。"
    },
    {
      "role": "user",
      "content": [
        {"type": "text", "text": "请分析这张图片中的内容。"},
        {
          "type": "image",
          "image": "data:image/png;base64,iVBORw0KGgo...",
          "mediaType": "image/png"
        }
      ]
    },
    {
      "role": "assistant",
      "content": [
        {"type": "text", "text": "图片中显示的是..."}
      ]
    },
    {
      "role": "user",
      "content": "继续详细说明"
    }
  ],
  "temperature": 0.7,
  "top_p": 0.9,
  "max_output_tokens": 4096,
  "stream": false
}
```

---

## 类型定义参考

### API 相关类型

```typescript
// src/renderer/src/types/apiModels.ts

// 筛选参数
interface ApiModelsFilter {
  providerType?: 'openai' | 'anthropic';
  offset?: number;
  limit?: number;
}

// 模型响应
interface ApiModelsResponse {
  object: 'list';
  data: ApiModel[];
  total?: number;
  offset?: number;
  limit?: number;
}

// 单个模型
interface ApiModel {
  id: string;              // "provider:model_id"
  object: 'model';
  created: number;
  name: string;
  owned_by: string;
  provider?: string;
  provider_name?: string;
  provider_type?: string;
  provider_model_id?: string;
}

// 聊天补全请求
interface ChatCompletionRequest {
  model: string;           // "provider:model_id"
  messages: Array<{
    role: 'system' | 'user' | 'assistant' | 'developer';
    content: string | Array<any>;
    name?: string;
  }>;
  temperature?: number;
  top_p?: number;
  max_tokens?: number;
  stream?: boolean;
  stream_options?: {
    include_usage?: boolean;
  };
  tools?: Array<{
    type: 'function';
    function: {
      name: string;
      description?: string;
      parameters?: any;
    };
  }>;
  // ...更多参数
}

// 聊天补全响应
interface ChatCompletionResponse {
  id: string;
  object: 'chat.completion';
  created: number;
  model: string;
  choices: Array<{
    index: number;
    message: {
      role: 'assistant';
      content: string | null;
      tool_calls?: Array<{
        id: string;
        type: 'function';
        function: {
          name: string;
          arguments: string;
        };
      }>;
    };
    finish_reason: 'stop' | 'length' | 'tool_calls' | 'content_filter' | null;
    logprobs: any;
  }>;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
  service_tier?: string;
}
```

### 健康检查相关类型

```typescript
// src/renderer/src/types/healthCheck.ts

enum HealthStatus {
  SUCCESS = 'success',
  FAILED = 'failed',
  NOT_CHECKED = 'not_checked',
  CHECKING = 'checking'
}

interface ApiKeyWithStatus {
  key: string;
  status: HealthStatus;
  latency?: number;
  error?: string;
}

interface ModelWithStatus {
  model: Model;
  status: HealthStatus;
  latency?: number;
  error?: string;
  checking: boolean;
  keyResults: ApiKeyWithStatus[];
}

interface ModelCheckOptions {
  provider: Provider;
  models: Model[];
  apiKeys: string[];
  isConcurrent: boolean;
  timeout?: number;
}
```

---

## 快速参考

### 核心服务

| 服务 | 路径 | 职责 |
|------|------|------|
| `modelsService` | `src/main/apiServer/services/models.ts` | 获取模型列表 |
| `chatCompletionService` | `src/main/apiServer/services/chat-completion.ts` | 处理聊天补全 |
| `HealthCheckService` | `src/renderer/src/services/HealthCheckService.ts` | 模型可用性检测 |
| `ApiService` | `src/renderer/src/services/ApiService.ts` | API 调用封装 |

### 关键函数

| 函数 | 位置 | 说明 |
|------|------|------|
| `getAvailableProviders` | `utils/index.ts` | 获取可用 Provider |
| `validateModelId` | `utils/index.ts` | 验证模型 ID |
| `transformModelToOpenAI` | `utils/index.ts` | 转换为 OpenAI 格式 |
| `checkModel` | `ApiService.ts` | 检查模型可用性 |
| `checkApi` | `ApiService.ts` | 执行 API 调用 |
| `convertMessagesToSdkMessages` | `messageConverter.ts` | 转换消息格式 |

### 重要配置

```typescript
// Provider 缓存配置
const PROVIDERS_CACHE_KEY = 'api-server:providers';
const PROVIDERS_CACHE_TTL = 10 * 1000; // 10秒

// API Key 轮换缓存
const KEY_CACHE_NAME = 'provider:{providerId}:last_used_key';

// 健康检查默认值
const DEFAULT_TIMEOUT = 15000;
```
