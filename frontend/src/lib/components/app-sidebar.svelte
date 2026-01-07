<script lang="ts" module>
	import LayoutDashboard from "@lucide/svelte/icons/layout-dashboard";
	import IdCard from "@lucide/svelte/icons/id-card";
	import Upload from "@lucide/svelte/icons/upload";
	import Dna from "@lucide/svelte/icons/dna";
	import BookUser from "@lucide/svelte/icons/book-user";
	import DatabaseBackup from "@lucide/svelte/icons/database-backup";
	import Settings from "@lucide/svelte/icons/settings";
	import BookPlus from "@lucide/svelte/icons/book-plus";
	import SquareCode from "@lucide/svelte/icons/square-code";
	import MessageSquareReply from "@lucide/svelte/icons/message-square-reply";
	import Trash from "@lucide/svelte/icons/trash-2";

	/*
	 * Data is now handled in the instance script below
	 */
</script>

<script lang="ts">
	import NavUser from "./nav-user.svelte";
	import TeamSwitcher from "./team-switcher.svelte";
	import * as Sidebar from "$lib/components/ui/sidebar/index.js";
	import { auth } from "$lib/stores/auth.svelte";
	import type { ComponentProps } from "svelte";

	const data = {
		// Group 1: 基础功能 (无标题)
		base: [
			{ title: "看板", url: "/", icon: LayoutDashboard },
			{ title: "角色库", url: "/characters", icon: IdCard },
			{ title: "全局世界书", url: "/worldinfo", icon: BookUser },
			{ title: "导入", url: "/import", icon: Upload },
		],
		// Group 2: AI 实验室
		aiLab: [
			{ title: "角色融合", url: "/ai/fusion", icon: Dna },
			{
				title: "世界书生成",
				url: "/ai/worldbook-create",
				icon: BookPlus,
			},
			{ title: "前端样式", url: "/ai/code", icon: SquareCode },
			{
				title: "QR生成",
				url: "/ai/quickreply",
				icon: MessageSquareReply,
			},
		],
		// Group 3: 系统
		system: [
			{ title: "回收站", url: "/system/trash", icon: Trash },
			{ title: "数据备份", url: "/system/backup", icon: DatabaseBackup },
			{ title: "设置", url: "/settings", icon: Settings },
		],
	};

	let userData = $derived({
		name: auth.username || "Admin",
		avatar: auth.avatar || "",
	});

	let {
		ref = $bindable(null),
		collapsible = "icon",
		...restProps
	}: ComponentProps<typeof Sidebar.Root> = $props();
</script>

<Sidebar.Root {collapsible} {...restProps}>
	<Sidebar.Header>
		<TeamSwitcher />
	</Sidebar.Header>
	<Sidebar.Content>
		<!-- Base Group -->
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each data.base as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<item.icon />
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>

		<!-- AI Lab Group -->
		<Sidebar.Group>
			<Sidebar.GroupLabel>AI 实验室</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each data.aiLab as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<item.icon />
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>

		<!-- System Group -->
		<Sidebar.Group>
			<Sidebar.GroupLabel>系统</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each data.system as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<item.icon />
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Content>
	<Sidebar.Footer>
		<NavUser user={userData} />
	</Sidebar.Footer>
	<Sidebar.Rail />
</Sidebar.Root>
