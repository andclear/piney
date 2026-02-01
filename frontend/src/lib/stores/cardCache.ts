/**
 * 角色卡数据缓存 Store
 * 用于缓存已访问的卡片数据，减少重复请求
 */

import { writable, get } from 'svelte/store';

interface CacheEntry {
    data: any;
    timestamp: number;
}

// 缓存过期时间（5分钟）
const CACHE_TTL = 5 * 60 * 1000;

// 最大缓存数量
const MAX_CACHE_SIZE = 50;

function createCardCache() {
    const cache = writable<Map<string, CacheEntry>>(new Map());

    return {
        subscribe: cache.subscribe,

        /**
         * 获取缓存的卡片数据
         */
        get(id: string): any | null {
            const entries = get(cache);
            const entry = entries.get(id);

            if (!entry) return null;

            // 检查是否过期
            if (Date.now() - entry.timestamp > CACHE_TTL) {
                entries.delete(id);
                cache.set(entries);
                return null;
            }

            return entry.data;
        },

        /**
         * 缓存卡片数据
         */
        set(id: string, data: any): void {
            cache.update(entries => {
                // 如果超过最大缓存数量，删除最旧的
                if (entries.size >= MAX_CACHE_SIZE) {
                    const oldestKey = entries.keys().next().value;
                    if (oldestKey) entries.delete(oldestKey);
                }

                entries.set(id, {
                    data,
                    timestamp: Date.now()
                });

                return entries;
            });
        },

        /**
         * 使指定卡片缓存失效
         */
        invalidate(id: string): void {
            cache.update(entries => {
                entries.delete(id);
                return entries;
            });
        },

        /**
         * 清空所有缓存
         */
        clear(): void {
            cache.set(new Map());
        }
    };
}

export const cardCache = createCardCache();

/**
 * 列表页面刷新信号
 * 当封面或其他需要列表刷新的内容更新时，设置为 true
 */
export const listNeedsRefresh = writable<boolean>(false);
