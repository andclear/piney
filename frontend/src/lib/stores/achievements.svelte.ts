/**
 * 成就状态管理
 */

import { writable, derived } from 'svelte/store';
import { api } from '$lib/api';

export interface Achievement {
    id: string;
    name: string;
    description: string;
    icon: string;
    unlocked_at: string | null;
    progress: number;
}

interface AchievementsState {
    achievements: Achievement[];
    loading: boolean;
    error: string | null;
}

const initialState: AchievementsState = {
    achievements: [],
    loading: false,
    error: null
};

function createAchievementsStore() {
    const { subscribe, set, update } = writable<AchievementsState>(initialState);

    return {
        subscribe,

        // 加载成就列表
        loadAchievements: async () => {
            update(state => ({ ...state, loading: true, error: null }));

            const response = await api.get<Achievement[]>('/achievements');

            if (response.success && response.data) {
                update(state => ({
                    ...state,
                    achievements: response.data!,
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

        // 重置状态
        reset: () => set(initialState)
    };
}

export const achievements = createAchievementsStore();

// 派生状态：已解锁的成就
export const unlockedAchievements = derived(achievements, $achievements =>
    $achievements.achievements.filter(a => a.unlocked_at !== null)
);

// 派生状态：解锁进度百分比
export const unlockProgress = derived(achievements, $achievements => {
    const total = $achievements.achievements.length;
    const unlocked = $achievements.achievements.filter(a => a.unlocked_at !== null).length;
    return total > 0 ? Math.round((unlocked / total) * 100) : 0;
});
