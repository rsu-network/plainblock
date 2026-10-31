import { invoke } from '@tauri-apps/api/core'

export const CLASS_ID_MODS = 6
export const CLASS_ID_RESOURCE_PACKS = 12
export const CLASS_ID_MODPACKS = 4471
export const CLASS_ID_SHADER_PACKS = 6552

export type CurseForgeModLoader = 'vanilla' | 'forge' | 'fabric' | 'quilt' | 'neoforge'

export interface CurseForgeAuthor {
	name: string
	url?: string
}

export interface CurseForgeCategory {
	id: number
	name: string
}

export interface CurseForgeLogo {
	thumbnailUrl?: string
	url?: string
}

export interface CurseForgeScreenshot {
	url: string
	thumbnailUrl?: string
}

export interface CurseForgeMod {
	id: number
	name: string
	summary: string
	slug: string
	classId?: number
	mainFileId: number
	downloadCount: number
	dateModified: string
	logo?: CurseForgeLogo
	screenshots: CurseForgeScreenshot[]
	authors: CurseForgeAuthor[]
	categories: CurseForgeCategory[]
	links?: { websiteUrl?: string; wikiUrl?: string; issuesUrl?: string; sourceUrl?: string }
}

export interface CurseForgePagination {
	index: number
	pageSize: number
	resultCount: number
	totalCount: number
}

export interface CurseForgeSearchResponse {
	data: CurseForgeMod[]
	pagination: CurseForgePagination
}

export interface CurseForgeFileDependency {
	modId: number
	relationType: number
}

export interface CurseForgeFile {
	id: number
	modId: number
	fileName: string
	displayName: string
	downloadUrl?: string
	gameVersions: string[]
	dependencies: CurseForgeFileDependency[]
	fileDate: string
	releaseType: number
	downloadCount: number
	fileLength: number
}

export async function curseforgeSearch(
	query: string,
	classId: number,
	gameVersion?: string,
	modLoader?: CurseForgeModLoader,
	categoryId?: number,
	sortField?: number,
	sortOrder?: 'asc' | 'desc',
	index = 0,
	pageSize = 20,
): Promise<CurseForgeSearchResponse> {
	return await invoke('plugin:curseforge|curseforge_search', {
		query,
		classId,
		gameVersion,
		modLoader,
		categoryId,
		sortField,
		sortOrder,
		index,
		pageSize,
	})
}

export async function curseforgeGetCategories(classId: number): Promise<CurseForgeCategory[]> {
	return await invoke('plugin:curseforge|curseforge_get_categories', { classId })
}

export async function curseforgeGetMod(modId: number): Promise<CurseForgeMod> {
	return await invoke('plugin:curseforge|curseforge_get_mod', { modId })
}

export async function curseforgeGetModDescription(modId: number): Promise<string> {
	return await invoke('plugin:curseforge|curseforge_get_mod_description', { modId })
}

export async function curseforgeGetFiles(
	modId: number,
	gameVersion?: string,
	modLoader?: CurseForgeModLoader,
): Promise<CurseForgeFile[]> {
	return await invoke('plugin:curseforge|curseforge_get_files', {
		modId,
		gameVersion,
		modLoader,
	})
}

export async function curseforgeInstallContent(
	instanceId: string,
	modId: number,
	fileId: number,
	classId: number,
): Promise<void> {
	return await invoke('plugin:curseforge|curseforge_install_content', {
		instanceId,
		modId,
		fileId,
		classId,
	})
}

export async function curseforgeInstallModpack(modId: number, fileId: number): Promise<string> {
	return await invoke('plugin:curseforge|curseforge_install_modpack', { modId, fileId })
}

export interface CurseForgeUpdate {
	project_path: string
	mod_id: number
	name: string
	current_file_name: string
	new_file_id: number
	new_file_name: string
}

export async function curseforgeCheckUpdates(instanceId: string): Promise<CurseForgeUpdate[]> {
	return await invoke('plugin:curseforge|curseforge_check_updates', { instanceId })
}

export async function curseforgeUpdateFile(
	instanceId: string,
	projectPath: string,
	newFileId: number,
): Promise<void> {
	return await invoke('plugin:curseforge|curseforge_update_file', {
		instanceId,
		projectPath,
		newFileId,
	})
}
