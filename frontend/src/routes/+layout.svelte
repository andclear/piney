<script lang="ts">
	import "../app.css";
	import { onMount } from "svelte";
	import { auth } from "$lib/stores/auth.svelte";
	import { settings } from "$lib/stores/settings.svelte";
	import { Toaster } from "$lib/components/ui/sonner/index.js";
	import { ModeWatcher } from "mode-watcher";

	let { children } = $props();

	onMount(() => {
		// 初始化认证状态
		auth.init();

		// 加载设置
		settings.loadSettings();
	});
</script>

<ModeWatcher />
<Toaster />

{#if auth.loading}
	<div class="flex h-screen w-full items-center justify-center bg-background">
		<div class="flex flex-col items-center gap-2">
			<!-- Simple CSS Spinner or Text -->
			<div
				class="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent"
			></div>
			<p class="text-muted-foreground text-sm">加载中...</p>
		</div>
	</div>
{:else}
	<div class="min-h-screen bg-background">
		{@render children()}
	</div>
{/if}
