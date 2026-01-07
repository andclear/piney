import { writable } from "svelte/store";

export interface BreadcrumbItem {
    label: string;
    href?: string;
}

export const breadcrumbs = writable<BreadcrumbItem[]>([]);
