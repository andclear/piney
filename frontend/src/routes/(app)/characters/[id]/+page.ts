/**
 * 角色卡详情页预加载
 */

import { API_BASE } from '$lib/api';
import { cardCache } from '$lib/stores/cardCache';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ params, fetch }) => {
    const cardId = params.id;
    const token = typeof localStorage !== 'undefined' ? localStorage.getItem('auth_token') : null;

    // 先检查缓存
    const cached = cardCache.get(cardId);
    if (cached) {
        return {
            card: JSON.parse(JSON.stringify(cached)), // Return deep copy to distinguish from cache
            fromCache: true
        };
    }

    try {
        const res = await fetch(`${API_BASE}/api/cards/${cardId}`, {
            headers: token ? { Authorization: `Bearer ${token}` } : {}
        });

        if (!res.ok) {
            return {
                card: null,
                error: '加载角色卡失败'
            };
        }

        const card = await res.json();

        // 缓存数据 (Store copy to keep cache pristine)
        cardCache.set(cardId, JSON.parse(JSON.stringify(card)));

        return {
            card,
            fromCache: false
        };
    } catch (error) {
        console.error('预加载角色卡失败:', error);
        return {
            card: null,
            error: String(error)
        };
    }
};

// 禁用 SSR，在客户端加载
export const ssr = false;
