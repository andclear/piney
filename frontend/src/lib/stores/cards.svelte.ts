/**
 * 角色卡状态管理
 */

import { writable, derived } from 'svelte/store';
import { api } from '$lib/api';

// 角色卡类型定义
export interface CardSummary {
    id: string;
    name: string;
    thumbnail_path: string | null;
    ai_summary: string | null;
    is_favorite: boolean;
    created_at: string;
}

export interface CardDetail extends CardSummary {
    file_hash: string;
    file_name: string;
    file_size: number;
    file_type: string;
    description: string | null;
    personality: string | null;
    scenario: string | null;
    first_message: string | null;
    mes_example: string | null;
    creator_notes: string | null;
    system_prompt: string | null;
    ai_tags: string[] | null;
    user_notes: string | null;
    updated_at: string;
}

interface CardsState {
    cards: CardSummary[];
    currentCard: CardDetail | null;
    loading: boolean;
    error: string | null;
    total: number;
    page: number;
    limit: number;
}

const initialState: CardsState = {
    cards: [],
    currentCard: null,
    loading: false,
    error: null,
    total: 0,
    page: 1,
    limit: 20
};

function createCardsStore() {
    const { subscribe, set, update } = writable<CardsState>(initialState);

    return {
        subscribe,

        // 加载卡片列表
        loadCards: async (page = 1, limit = 20) => {
            update(state => ({ ...state, loading: true, error: null }));

            const response = await api.get<{
                items: CardSummary[];
                total: number;
                page: number;
                limit: number;
            }>(`/cards?page=${page}&limit=${limit}`);

            if (response.success && response.data) {
                update(state => ({
                    ...state,
                    cards: response.data!.items,
                    total: response.data!.total,
                    page: response.data!.page,
                    limit: response.data!.limit,
                    loading: false
                }));
            } else {
                update(state => ({
                    ...state,
                    error: response.error || '加载失败',
                    loading: false
                }));
            }
        },

        // 加载单个卡片
        loadCard: async (id: string) => {
            update(state => ({ ...state, loading: true, error: null }));

            const response = await api.get<CardDetail>(`/cards/${id}`);

            if (response.success && response.data) {
                update(state => ({
                    ...state,
                    currentCard: response.data!,
                    loading: false
                }));
            } else {
                update(state => ({
                    ...state,
                    error: response.error || '加载失败',
                    loading: false
                }));
            }
        },

        // 清除当前卡片
        clearCurrentCard: () => {
            update(state => ({ ...state, currentCard: null }));
        },

        // 切换收藏状态
        toggleFavorite: async (id: string) => {
            update(state => ({
                ...state,
                cards: state.cards.map(card =>
                    card.id === id ? { ...card, is_favorite: !card.is_favorite } : card
                )
            }));

            // 调用 API
            await api.patch(`/cards/${id}`, { is_favorite: true });
        },

        // 重置状态
        reset: () => set(initialState)
    };
}

export const cards = createCardsStore();

// 派生状态：是否有卡片
export const hasCards = derived(cards, $cards => $cards.cards.length > 0);

// 派生状态：收藏的卡片
export const favoriteCards = derived(cards, $cards =>
    $cards.cards.filter(card => card.is_favorite)
);
