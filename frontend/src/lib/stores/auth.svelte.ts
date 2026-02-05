import { goto } from '$app/navigation';
import { page } from '$app/stores';
import { get } from 'svelte/store';

import { getApiBase } from '$lib/api';

class AuthStore {
    authenticated = $state(false);
    initialized = $state(false);
    loading = $state(true);
    username = $state<string | null>(null);
    avatar = $state<string>("");

    async init() {
        let retries = 20; // 10 seconds timeout
        let lastError = null;

        while (retries > 0) {
            try {
                const token = localStorage.getItem('auth_token');
                const headers: HeadersInit = {};
                if (token) {
                    headers['Authorization'] = `Bearer ${token}`;
                }

                const res = await fetch(`${getApiBase()}/api/auth/status`, {
                    headers
                });

                if (res.ok) {
                    const data = await res.json();
                    this.initialized = data.initialized;

                    this.authenticated = !!token;
                    this.username = data.username;

                    if (this.authenticated) {
                        this.fetchAvatar();
                    }
                    this.loading = false;
                    this.checkRedirect();
                    return; // Success
                } else if (res.status === 401) {
                    // Token invalid
                    localStorage.removeItem('auth_token');
                    this.authenticated = false;
                    // Let it proceed to checkRedirect
                    break;
                }
                // Server returned error (500 etc), stop retrying?
                // Or maybe database is still locking? Retry a few times?
                // Let's count it as a retry-able failure for now if status is 5xx
                if (res.status >= 500) {
                    throw new Error(`Server Error ${res.status}`);
                }
                break; // Client error (4xx), stop retrying
            } catch (e) {
                console.warn(`Auth check failed (retries left: ${retries}):`, e);
                lastError = e;
                retries--;
                if (retries > 0) {
                    await new Promise(r => setTimeout(r, 500));
                }
            }
        }

        // If we get here, we failed or ran out of retries
        console.error("Auth initialization failed permanently:", lastError);
        this.loading = false;
        // Proceeding to checkRedirect might loop if initialized is false
        // But better than stuck loading
        this.checkRedirect();
    }

    checkRedirect() {
        if (this.loading) return;

        const path = window.location.pathname;

        if (!this.initialized) {
            if (path !== '/sign-up') goto('/sign-up');
            return;
        }

        if (this.initialized && !this.authenticated) {
            if (path !== '/login' && path !== '/sign-up') goto('/login');
            return;
        }

        if (this.initialized && this.authenticated) {
            if (path === '/login' || path === '/sign-up') goto('/');
            return;
        }
    }

    async login(username: string, token: string) {
        localStorage.setItem('auth_token', token);
        this.authenticated = true;
        this.username = username;
        this.fetchAvatar();
        this.checkRedirect();
    }

    async setup(token: string, username: string) {
        localStorage.setItem('auth_token', token);
        this.initialized = true;
        this.authenticated = true;
        this.username = username;
        this.fetchAvatar();
        this.checkRedirect();
    }

    logout() {
        localStorage.removeItem('auth_token');
        this.authenticated = false;
        this.username = null;
        goto('/login');
    }
    async fetchAvatar() {
        try {
            const token = localStorage.getItem('auth_token');
            const headers: HeadersInit = {};
            if (token) {
                headers['Authorization'] = `Bearer ${token}`;
            }
            const res = await fetch(`${getApiBase()}/api/settings`, { headers });
            if (res.ok) {
                const settings = await res.json();
                if (settings.avatar) {
                    this.avatar = settings.avatar;
                }
            }
        } catch (e) {
            console.error("Failed to fetch avatar", e);
        }
    }
}

export const auth = new AuthStore();
