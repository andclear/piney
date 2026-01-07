<script lang="ts">
	import * as Sidebar from "$lib/components/ui/sidebar/index.js";
	import AppLogo from "$lib/components/app-logo.svelte";
</script>

<Sidebar.Menu>
	<Sidebar.MenuItem>
		<Sidebar.MenuButton
			size="lg"
			class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
		>
			<AppLogo class="size-8" />
			<div class="grid flex-1 text-left text-sm leading-tight">
				<span class="truncate font-semibold">小兄许</span>
			</div>
		</Sidebar.MenuButton>
	</Sidebar.MenuItem>
</Sidebar.Menu>
