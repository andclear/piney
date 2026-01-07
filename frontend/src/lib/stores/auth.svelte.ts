import { goto } from '$app/navigation';
import { page } from '$app/stores';
import { get } from 'svelte/store';

const API_BASE = import.meta.env.VITE_API_BASE || "http://localhost:9696";

class AuthStore {
    authenticated = $state(false);
    initialized = $state(false); // Backend config initialized
    loading = $state(true);
    username = $state<string | null>(null);
    avatar = $state<string>("");

    async init() {
        try {
            const token = localStorage.getItem('auth_token');
            const headers: HeadersInit = {};
            if (token) {
                headers['Authorization'] = `Bearer ${token}`;
            }

            const res = await fetch(`${API_BASE}/api/auth/status`, {
                headers
            });

            if (res.ok) {
                const data = await res.json();
                this.initialized = data.initialized;

                // If backend says initialized=true and username is present, we are logged in
                // (My status endpoint returns username if token is valid? 
                // Wait, my Rust logic for get_status currently returns username from Config, ignoring token!
                // I need to update Rust get_status logic to actually check token if I want "authenticated" field to be accurate there?
                // OR, I just rely on token validation middleware for protected routes.
                // Re-reading Rust Plan: "authenticated: false, // Client should check this via middleware/token"
                // So Status endpoint ONLY tells us if Config exists.
                // To check if Token is valid, we should probably try a protected endpoint or verify token locally (bad idea) or have a /me endpoint.

                // Let's rely on:
                // 1. If !data.initialized => Redirect Signup.
                // 2. If data.initialized:
                //      If we have token in localStorage => Assume logged in (validity checked on next API call).
                //      Else => Redirect Login.

                this.authenticated = !!token;
                this.username = data.username;

                // Create a separate call to get avatar?
                // Or we can just do it here.
                if (this.authenticated) {
                    this.fetchAvatar();
                }
            }
        } catch (e) {
            console.error("Auth check failed", e);
        } finally {
            this.loading = false;
            this.checkRedirect();
        }
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
            // Allow sign-up page? No, if initialized, signup is disabled (or should be).
            // Actually setup endpoint blocks it.
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

    async setup(token: string) {
        localStorage.setItem('auth_token', token);
        this.initialized = true;
        this.authenticated = true;
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
            const res = await fetch(`${API_BASE}/api/settings`, { headers });
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
