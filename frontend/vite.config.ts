import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
    plugins: [tailwindcss(), sveltekit()],
    server: {
        port: 5173,
        strictPort: true,
        host: '0.0.0.0',
        proxy: {
            '/api': {
                target: 'http://127.0.0.1:9696',
                changeOrigin: true
            },
            '/uploads': {
                target: 'http://127.0.0.1:9696',
                changeOrigin: true
            },
            '/cards': {
                target: 'http://127.0.0.1:9696',
                changeOrigin: true
            },
            '/images': {
                target: 'http://127.0.0.1:9696',
                changeOrigin: true
            }
        }
    }
});
