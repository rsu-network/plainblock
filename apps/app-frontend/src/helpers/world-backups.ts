import { invoke } from '@tauri-apps/api/core'

export interface WorldBackup {
	world: string
	file_name: string
	created: number
	size: number
}

export async function listWorldBackups(instanceId: string): Promise<WorldBackup[]> {
	return await invoke('plugin:world-backups|world_backups_list', { instanceId })
}

export async function createWorldBackup(instanceId: string, world: string): Promise<WorldBackup> {
	return await invoke('plugin:world-backups|world_backups_create', { instanceId, world })
}

export async function restoreWorldBackup(
	instanceId: string,
	world: string,
	fileName: string,
): Promise<void> {
	return await invoke('plugin:world-backups|world_backups_restore', {
		instanceId,
		world,
		fileName,
	})
}

export async function deleteWorldBackup(
	instanceId: string,
	world: string,
	fileName: string,
): Promise<void> {
	return await invoke('plugin:world-backups|world_backups_delete', {
		instanceId,
		world,
		fileName,
	})
}
