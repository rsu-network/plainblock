import { get_content_items, get as getInstance, edit } from '@/helpers/instance'
import { get_max_memory } from '@/helpers/jre.js'

const UNDO_KEY_PREFIX = 'plainblock-optimize-undo-'

const BASE_FLAGS = [
	'-XX:+UseG1GC',
	'-XX:+ParallelRefProcEnabled',
	'-XX:MaxGCPauseMillis=200',
	'-XX:+UnlockExperimentalVMOptions',
	'-XX:+DisableExplicitGC',
	'-XX:G1NewSizePercent=30',
	'-XX:G1MaxNewSizePercent=40',
	'-XX:G1ReservePercent=20',
	'-XX:G1HeapWastePercent=5',
	'-XX:G1MixedGCCountTarget=4',
	'-XX:InitiatingHeapOccupancyPercent=15',
	'-XX:G1MixedGCLiveThresholdPercent=90',
	'-XX:G1RSetUpdatingPauseTimePercent=5',
	'-XX:SurvivorRatio=32',
	'-XX:+PerfDisableSharedMem',
	'-XX:MaxTenuringThreshold=1',
]

export interface OptimizationPlan {
	modCount: number
	totalMemoryMb: number
	currentMemoryMb: number | null
	recommendedMemoryMb: number
	flagsToAdd: string[]
	extraLaunchArgs: string[]
}

function flagKey(flag: string) {
	return flag.replace(/^-XX:[+-]?/, '').split('=')[0]
}

function roundDown(value: number, step: number) {
	return Math.floor(value / step) * step
}

export function recommendMemoryMb(modCount: number, totalMemoryMb: number) {
	let wanted = 2048
	if (modCount >= 200) wanted = 8192
	else if (modCount >= 100) wanted = 6144
	else if (modCount >= 30) wanted = 4096
	else if (modCount >= 1) wanted = 3072

	const ceiling = Math.max(2048, roundDown(totalMemoryMb * 0.5, 512))
	return Math.min(wanted, ceiling)
}

export async function planOptimization(instanceId: string): Promise<OptimizationPlan> {
	const [instance, items, totalMemoryMb] = await Promise.all([
		getInstance(instanceId),
		get_content_items(instanceId).catch(() => []),
		get_max_memory() as Promise<number>,
	])

	const modCount = items.filter((item) => item.project_type === 'mod').length
	const recommendedMemoryMb = recommendMemoryMb(modCount, totalMemoryMb)
	const existing = instance?.extra_launch_args ?? []
	const existingKeys = new Set(existing.map(flagKey))
	const heapFlag = recommendedMemoryMb >= 4096 ? '-XX:G1HeapRegionSize=8M' : '-XX:G1HeapRegionSize=4M'
	const flagsToAdd = [...BASE_FLAGS, heapFlag].filter((flag) => !existingKeys.has(flagKey(flag)))

	return {
		modCount,
		totalMemoryMb,
		currentMemoryMb: instance?.memory?.maximum ?? null,
		recommendedMemoryMb,
		flagsToAdd,
		extraLaunchArgs: [...existing, ...flagsToAdd],
	}
}

export async function applyOptimization(instanceId: string, plan: OptimizationPlan) {
	const instance = await getInstance(instanceId)
	try {
		localStorage.setItem(
			UNDO_KEY_PREFIX + instanceId,
			JSON.stringify({
				memory: instance?.memory ?? null,
				extra_launch_args: instance?.extra_launch_args ?? [],
			}),
		)
	} catch {
		// undo information is a convenience only
	}
	await edit(instanceId, {
		memory: { maximum: plan.recommendedMemoryMb },
		extra_launch_args: plan.extraLaunchArgs,
	})
}

export function canUndoOptimization(instanceId: string): boolean {
	try {
		return localStorage.getItem(UNDO_KEY_PREFIX + instanceId) !== null
	} catch {
		return false
	}
}

export async function undoOptimization(instanceId: string) {
	let saved: { memory: { maximum: number } | null; extra_launch_args: string[] } | null = null
	try {
		const raw = localStorage.getItem(UNDO_KEY_PREFIX + instanceId)
		saved = raw ? JSON.parse(raw) : null
	} catch {
		saved = null
	}
	if (!saved) return
	await edit(instanceId, {
		memory: saved.memory ?? undefined,
		extra_launch_args: saved.extra_launch_args,
	})
	try {
		localStorage.removeItem(UNDO_KEY_PREFIX + instanceId)
	} catch {
		// ignore
	}
}
