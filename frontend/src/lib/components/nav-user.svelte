<script lang="ts">
	import * as Avatar from "$lib/components/ui/avatar/index.js";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
	import * as Sidebar from "$lib/components/ui/sidebar/index.js";
	import { useSidebar } from "$lib/components/ui/sidebar/index.js";
	import LogOutIcon from "@lucide/svelte/icons/log-out";
	import MoonIcon from "@lucide/svelte/icons/moon";
	import SunIcon from "@lucide/svelte/icons/sun";
	import UserRoundPen from "@lucide/svelte/icons/user-round-pen";
	import ChevronsUpDownIcon from "@lucide/svelte/icons/chevrons-up-down";
	import { toggleMode, mode } from "mode-watcher";

	import { get } from "svelte/store";
	import { auth } from "$lib/stores/auth.svelte";
	import { settings } from "$lib/stores/settings.svelte";
	import UserProfileDialog from "$lib/components/user-profile-dialog.svelte";

	let { user }: { user: { name: string; avatar: string } } = $props();
	const sidebar = useSidebar();

	let showProfileDialog = $state(false);

	function handleLogout() {
		auth.logout();
	}

	function handleModeToggle(e: MouseEvent) {
		e.preventDefault();
		toggleMode();
		// Wait for mode to update then save to DB
		setTimeout(() => {
			const current = (mode as any).current || (mode as any);
			console.log("Saving theme preference to DB:", current);

			if (current && typeof current === "string") {
				settings.updateSettings({ theme: current as any });
			}
		}, 100);
	}
</script>

<Sidebar.Menu>
	<Sidebar.MenuItem>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Sidebar.MenuButton
						size="lg"
						class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
						{...props}
					>
						<Avatar.Root class="size-8 rounded-full">
							<Avatar.Image src={user.avatar} alt={user.name} />
							<Avatar.Fallback class="rounded-full"
								>{user.name?.[0]?.toUpperCase() ||
									"A"}</Avatar.Fallback
							>
						</Avatar.Root>
						<div
							class="grid flex-1 text-start text-sm leading-tight"
						>
							<span class="truncate font-medium">{user.name}</span
							>
						</div>
						<ChevronsUpDownIcon class="ms-auto size-4" />
					</Sidebar.MenuButton>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content
				class="w-(--bits-dropdown-menu-anchor-width) min-w-56 rounded-lg"
				side={sidebar.isMobile ? "bottom" : "right"}
				align="end"
				sideOffset={4}
			>
				<DropdownMenu.Label class="p-0 font-normal">
					<div
						class="flex items-center gap-2 px-1 py-1.5 text-start text-sm"
					>
						<Avatar.Root class="size-8 rounded-full">
							<Avatar.Image src={user.avatar} alt={user.name} />
							<Avatar.Fallback class="rounded-full"
								>{user.name?.[0]?.toUpperCase() ||
									"A"}</Avatar.Fallback
							>
						</Avatar.Root>
						<div
							class="grid flex-1 text-start text-sm leading-tight"
						>
							<span class="truncate font-medium">{user.name}</span
							>
						</div>
					</div>
				</DropdownMenu.Label>
				<DropdownMenu.Separator />
				<DropdownMenu.Group>
					<DropdownMenu.Item onclick={handleModeToggle}>
						<div class="flex items-center gap-2">
							<div class="relative size-4">
								<!-- Light Mode: Show Moon. Dark Mode: Hidden -->
								<MoonIcon
									class="size-4 absolute rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0"
								/>
								<!-- Dark Mode: Show Sun. Light Mode: Hidden -->
								<SunIcon
									class="size-4 absolute rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100"
								/>
							</div>
							<span class="block dark:hidden">深色模式</span>
							<span class="hidden dark:block">浅色模式</span>
						</div>
					</DropdownMenu.Item>
					<DropdownMenu.Item
						onSelect={() => (showProfileDialog = true)}
					>
						<UserRoundPen />
						用户设置
					</DropdownMenu.Item>
				</DropdownMenu.Group>
				<DropdownMenu.Separator />
				<DropdownMenu.Item onSelect={handleLogout}>
					<LogOutIcon />
					登出
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.MenuItem>
</Sidebar.Menu>

<UserProfileDialog bind:open={showProfileDialog} {user} />
