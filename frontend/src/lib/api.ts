/**
 * API 调用统一封装
 * 
 * 智能检测运行环境：
 * - 浏览器/Docker 模式：使用 fetch
 * - Tauri 模式：通过 localhost HTTP 请求
 */

// API 基础路径
export const getApiBase = (): string => {
    // 检测是否在 Tauri 环境 (支持 v1 和 v2)
    if (typeof window !== 'undefined' &&
        ((window as any).__TAURI__ || (window as any).__TAURI_INTERNALS__)) {
        return 'http://127.0.0.1:9696';
    }
    // 开发环境或 Docker 模式建议使用相对路径
    return '';
};

export const API_BASE = getApiBase();

/**
 * 智能解析 URL
 * 如果是以 /api 开头的相对路径，则根据环境补全 API_BASE
 */
export function resolveUrl(url: string | null | undefined): string {
    if (!url) return "/default.webp";
    if (
        url.startsWith("http://") ||
        url.startsWith("https://") ||
        url.startsWith("data:") ||
        url.startsWith("blob:")
    ) {
        return url;
    }
    // 如果是 /api, /cards, /uploads 开头的路径，且没有 API_BASE 前缀，则补全
    if (url.startsWith("/api") || url.startsWith("/cards") || url.startsWith("/uploads")) {
        return `${API_BASE}${url}`;
    }
    return url;
}

// 请求选项类型
interface RequestOptions {
    method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
    body?: unknown;
    headers?: Record<string, string>;
}

// API 响应类型
export interface ApiResponse<T = unknown> {
    success: boolean;
    data?: T;
    message?: string;
    error?: string;
}

// 分页响应类型
export interface PaginatedResponse<T> {
    items: T[];
    total: number;
    page: number;
    limit: number;
    total_pages: number;
}

/**
 * 统一的 API 调用函数
 */
export async function apiCall<T = unknown>(
    path: string,
    options: RequestOptions = {}
): Promise<ApiResponse<T>> {
    const { method = 'GET', body, headers = {} } = options;

    const apiBase = getApiBase();
    const url = `${apiBase}/api${path}`;

    const requestHeaders: HeadersInit = {
        'Content-Type': 'application/json',
        ...headers
    };

    // 添加认证 token（如果存在）
    const token = typeof localStorage !== 'undefined' ? localStorage.getItem('auth_token') : null;
    if (token) {
        requestHeaders['Authorization'] = `Bearer ${token}`;
    }

    try {
        const response = await fetch(url, {
            method,
            headers: requestHeaders,
            body: body ? JSON.stringify(body) : undefined
        });

        // 优先处理 401，不需要解析 body
        if (response.status === 401) {
            if (typeof localStorage !== 'undefined') {
                localStorage.removeItem('auth_token');
            }
            if (typeof window !== 'undefined' && !window.location.pathname.startsWith('/login')) {
                window.location.href = '/login';
            }
            return {
                success: false,
                error: '未授权，请登录'
            };
        }

        // 尝试解析 JSON，如果为空或格式错误则忽略
        let data: any = {};
        try {
            const text = await response.text();
            if (text) {
                data = JSON.parse(text);
            }
        } catch (e) {
            // 忽略解析错误
        }

        if (!response.ok) {
            return {
                success: false,
                error: data.error || data.message || `请求失败: ${response.status}`
            };
        }

        return {
            success: true,
            data: data as T
        };
    } catch (error) {
        console.error('API 调用错误:', error);
        return {
            success: false,
            error: error instanceof Error ? error.message : '网络请求失败'
        };
    }
}

// 便捷方法
export const api = {
    get: <T>(path: string) => apiCall<T>(path, { method: 'GET' }),
    post: <T>(path: string, body?: unknown) => apiCall<T>(path, { method: 'POST', body }),
    put: <T>(path: string, body?: unknown) => apiCall<T>(path, { method: 'PUT', body }),
    patch: <T>(path: string, body?: unknown) => apiCall<T>(path, { method: 'PATCH', body }),
    delete: <T>(path: string) => apiCall<T>(path, { method: 'DELETE' })
};

// 类型导出
export type { RequestOptions };
