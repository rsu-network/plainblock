import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

export type ModIssueKind = 'missing_dependency' | 'duplicate' | 'wrong_loader' | 'unreadable'

export interface ModIssue {
	kind: ModIssueKind
	file: string
	message: string
}

export async function checkMods(instanceId: string): Promise<ModIssue[]> {
	return await invoke('plugin:mod-check|mod_check_scan', { instanceId })
}

export interface PendingModCheck {
	instanceId: string
	issues: ModIssue[]
	resolve: (launch: boolean) => void
}

export const pendingModCheck = ref<PendingModCheck | null>(null)

/** Issues that make a crash very likely, as opposed to harmless leftovers. */
export function isBlocking(issue: ModIssue) {
	return issue.kind !== 'unreadable'
}

/**
 * Checks the mods of an instance right before launch. Resolves to false when the
 * user decides to fix the problems first. A failing check never blocks launching.
 */
export async function confirmLaunchAfterModCheck(instanceId: string): Promise<boolean> {
	let issues: ModIssue[]
	try {
		issues = (await checkMods(instanceId)).filter(isBlocking)
	} catch {
		return true
	}
	if (issues.length === 0) return true

	return await new Promise<boolean>((resolve) => {
		pendingModCheck.value = {
			instanceId,
			issues,
			resolve: (launch) => {
				pendingModCheck.value = null
				resolve(launch)
			},
		}
	})
}
