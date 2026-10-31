<script setup lang="ts">
import { DownloadIcon, HistoryIcon, TrashIcon } from '@modrinth/assets'
import { Button, Card, injectNotificationManager } from '@modrinth/ui'
import { computed, ref } from 'vue'

import {
	type CurseForgeUpdate,
	curseforgeCheckUpdates,
	curseforgeUpdateFile,
} from '@/helpers/curseforge'
import {
	createWorldBackup,
	deleteWorldBackup,
	listWorldBackups,
	restoreWorldBackup,
	type WorldBackup,
} from '@/helpers/world-backups'
import { checkMods, type ModIssue } from '@/helpers/mod-check'
import {
	applyOptimization,
	canUndoOptimization,
	type OptimizationPlan,
	planOptimization,
	undoOptimization,
} from '@/helpers/optimize'
import { get_instance_worlds } from '@/helpers/worlds'

import { injectInstancePage } from '../instance-context'

const { handleError, addNotification } = injectNotificationManager()
const instancePage = injectInstancePage()
const instanceId = computed(() => instancePage.instanceId.value)

const backups = ref<WorldBackup[]>([])
const worldNames = ref<string[]>([])
const loadingBackups = ref(true)
const busy = ref<string | null>(null)
const confirmingRestore = ref<string | null>(null)

const updates = ref<CurseForgeUpdate[] | null>(null)
const checkingUpdates = ref(false)
const updatingPath = ref<string | null>(null)

const backupsByWorld = computed(() => {
	const groups = new Map<string, WorldBackup[]>()
	for (const backup of backups.value) {
		groups.set(backup.world, [...(groups.get(backup.world) ?? []), backup])
	}
	for (const name of worldNames.value) {
		if (!groups.has(name)) groups.set(name, [])
	}
	return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b))
})

function restoreKey(backup: WorldBackup) {
	return `${backup.world}/${backup.file_name}`
}

function formatSize(bytes: number) {
	if (bytes > 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`
	if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
	return `${Math.max(1, Math.round(bytes / 1024))} KB`
}

function formatDate(seconds: number) {
	return new Date(seconds * 1000).toLocaleString()
}

function backupKind(fileName: string) {
	if (fileName.includes('manual_keep')) return 'Manual'
	if (fileName.includes('before-restore_keep')) return 'Before restore'
	return 'Automatic'
}

async function loadBackups() {
	loadingBackups.value = true
	try {
		backups.value = await listWorldBackups(instanceId.value)
		const worlds = await get_instance_worlds(instanceId.value).catch(() => [])
		worldNames.value = worlds.flatMap((world) =>
			world.type === 'singleplayer' ? [world.path.split(/[\\/]/).pop() ?? world.path] : [],
		)
	} catch (error) {
		handleError(error)
	} finally {
		loadingBackups.value = false
	}
}

async function backupNow(world: string) {
	busy.value = `create:${world}`
	try {
		await createWorldBackup(instanceId.value, world)
		await loadBackups()
		addNotification({ type: 'success', title: 'Backup created', text: world })
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = null
	}
}

async function restore(backup: WorldBackup) {
	const key = restoreKey(backup)
	if (confirmingRestore.value !== key) {
		confirmingRestore.value = key
		return
	}
	confirmingRestore.value = null
	busy.value = `restore:${key}`
	try {
		await restoreWorldBackup(instanceId.value, backup.world, backup.file_name)
		await loadBackups()
		addNotification({
			type: 'success',
			title: 'World restored',
			text: `${backup.world} was rolled back. The previous state was saved as a backup.`,
		})
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = null
	}
}

async function remove(backup: WorldBackup) {
	busy.value = `delete:${restoreKey(backup)}`
	try {
		await deleteWorldBackup(instanceId.value, backup.world, backup.file_name)
		await loadBackups()
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = null
	}
}

async function checkUpdates() {
	checkingUpdates.value = true
	try {
		updates.value = await curseforgeCheckUpdates(instanceId.value)
	} catch (error) {
		handleError(error)
	} finally {
		checkingUpdates.value = false
	}
}

async function applyUpdate(update: CurseForgeUpdate) {
	updatingPath.value = update.project_path
	try {
		await curseforgeUpdateFile(instanceId.value, update.project_path, update.new_file_id)
		updates.value = (updates.value ?? []).filter(
			(entry) => entry.project_path !== update.project_path,
		)
	} catch (error) {
		handleError(error)
	} finally {
		updatingPath.value = null
	}
}

async function updateAll() {
	for (const update of [...(updates.value ?? [])]) {
		await applyUpdate(update)
	}
}

const modIssues = ref<ModIssue[] | null>(null)
const checkingMods = ref(false)

async function runModCheck() {
	checkingMods.value = true
	try {
		modIssues.value = await checkMods(instanceId.value)
	} catch (error) {
		handleError(error)
	} finally {
		checkingMods.value = false
	}
}

const plan = ref<OptimizationPlan | null>(null)
const planning = ref(false)
const applying = ref(false)
const canUndo = ref(canUndoOptimization(instanceId.value))

async function showPlan() {
	planning.value = true
	try {
		plan.value = await planOptimization(instanceId.value)
	} catch (error) {
		handleError(error)
	} finally {
		planning.value = false
	}
}

async function applyPlan() {
	if (!plan.value) return
	applying.value = true
	try {
		await applyOptimization(instanceId.value, plan.value)
		plan.value = null
		canUndo.value = true
		addNotification({
			type: 'success',
			title: 'Instance optimized',
			text: 'Memory and Java settings were updated. You can undo this here.',
		})
	} catch (error) {
		handleError(error)
	} finally {
		applying.value = false
	}
}

async function undoPlan() {
	try {
		await undoOptimization(instanceId.value)
		canUndo.value = false
		addNotification({ type: 'success', title: 'Settings restored', text: '' })
	} catch (error) {
		handleError(error)
	}
}

function formatMb(value: number | null) {
	if (value === null) return 'default'
	return `${Number((value / 1024).toFixed(1))} GB`
}

void loadBackups()
</script>

<template>
	<div class="flex flex-col gap-6">
		<Card class="flex flex-col gap-3">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h2 class="m-0 text-xl font-semibold text-contrast">Mod check</h2>
					<p class="m-0 mt-1 text-secondary">
						Finds missing dependencies, duplicate mods and mods for the wrong loader. This also runs
						automatically before the game starts.
					</p>
				</div>
				<Button :disabled="checkingMods" @click="runModCheck">
					{{ checkingMods ? 'Checking...' : 'Check mods' }}
				</Button>
			</div>
			<div v-if="modIssues && modIssues.length === 0" class="text-secondary">
				No problems found.
			</div>
			<div
				v-for="(issue, index) in modIssues ?? []"
				:key="index"
				class="flex flex-col rounded-xl border border-solid border-surface-5 bg-surface-3 px-3 py-2 text-sm"
			>
				<span class="truncate font-semibold text-contrast">{{ issue.file }}</span>
				<span class="text-secondary">{{ issue.message }}</span>
			</div>
		</Card>

		<Card class="flex flex-col gap-3">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h2 class="m-0 text-xl font-semibold text-contrast">Optimize</h2>
					<p class="m-0 mt-1 text-secondary">
						Picks a fitting memory amount for your computer and mods and adds proven Java settings
						for smoother gameplay.
					</p>
				</div>
				<div class="flex gap-2">
					<Button v-if="canUndo" type="outlined" @click="undoPlan">Undo</Button>
					<Button :disabled="planning" @click="showPlan">
						{{ planning ? 'Analysing...' : 'Analyse' }}
					</Button>
				</div>
			</div>
			<div
				v-if="plan"
				class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-surface-3 p-3 text-sm"
			>
				<span class="text-contrast">
					{{ plan.modCount }} mods, {{ formatMb(plan.totalMemoryMb) }} memory in this computer.
				</span>
				<span class="text-contrast">
					Memory for the game: {{ formatMb(plan.currentMemoryMb) }} ->
					<strong>{{ formatMb(plan.recommendedMemoryMb) }}</strong>
				</span>
				<span class="text-secondary">
					{{
						plan.flagsToAdd.length > 0
							? `${plan.flagsToAdd.length} Java settings will be added.`
							: 'The Java settings are already in place.'
					}}
				</span>
				<div class="flex justify-end">
					<Button type="colored" color="brand" :disabled="applying" @click="applyPlan">
						{{ applying ? 'Applying...' : 'Apply' }}
					</Button>
				</div>
			</div>
		</Card>

		<Card class="flex flex-col gap-3">
			<div>
				<h2 class="m-0 text-xl font-semibold text-contrast">World backups</h2>
				<p class="m-0 mt-1 text-secondary">
					Changed worlds are backed up automatically before the game starts. The 10 newest automatic
					backups per world are kept. Manual backups are never deleted for you.
				</p>
			</div>

			<div v-if="loadingBackups" class="text-secondary">Loading...</div>
			<div v-else-if="backupsByWorld.length === 0" class="text-secondary">
				No worlds found in this instance yet.
			</div>

			<div
				v-for="[world, worldBackups] in backupsByWorld"
				:key="world"
				class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-surface-3 p-3"
			>
				<div class="flex items-center justify-between gap-2">
					<span class="font-semibold text-contrast">{{ world }}</span>
					<Button :disabled="busy !== null" @click="backupNow(world)">
						<HistoryIcon />
						{{ busy === `create:${world}` ? 'Backing up...' : 'Back up now' }}
					</Button>
				</div>
				<div v-if="worldBackups.length === 0" class="text-sm text-secondary">No backups yet.</div>
				<div
					v-for="backup in worldBackups"
					:key="backup.file_name"
					class="flex items-center justify-between gap-2 text-sm"
				>
					<div class="flex flex-col">
						<span class="text-contrast">{{ formatDate(backup.created) }}</span>
						<span class="text-secondary">
							{{ backupKind(backup.file_name) }} - {{ formatSize(backup.size) }}
						</span>
					</div>
					<div class="flex items-center gap-2">
						<Button
							:type="confirmingRestore === restoreKey(backup) ? 'colored' : 'outlined'"
							:color="confirmingRestore === restoreKey(backup) ? 'red' : undefined"
							:disabled="busy !== null"
							@click="restore(backup)"
						>
							{{ confirmingRestore === restoreKey(backup) ? 'Click again to confirm' : 'Restore' }}
						</Button>
						<Button
							type="outlined"
							:disabled="busy !== null"
							aria-label="Delete backup"
							@click="remove(backup)"
						>
							<TrashIcon />
						</Button>
					</div>
				</div>
			</div>
		</Card>

		<Card class="flex flex-col gap-3">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h2 class="m-0 text-xl font-semibold text-contrast">CurseForge updates</h2>
					<p class="m-0 mt-1 text-secondary">
						Checks content that was installed from the CurseForge tab for newer versions.
					</p>
				</div>
				<div class="flex gap-2">
					<Button :disabled="checkingUpdates" @click="checkUpdates">
						{{ checkingUpdates ? 'Checking...' : 'Check for updates' }}
					</Button>
					<Button
						v-if="updates && updates.length > 1"
						type="colored"
						color="brand"
						:disabled="updatingPath !== null"
						@click="updateAll"
					>
						<DownloadIcon />
						Update all
					</Button>
				</div>
			</div>

			<div v-if="updates && updates.length === 0" class="text-secondary">
				Everything is up to date.
			</div>
			<div
				v-for="update in updates ?? []"
				:key="update.project_path"
				class="flex items-center justify-between gap-2 rounded-xl border border-solid border-surface-5 bg-surface-3 p-3 text-sm"
			>
				<div class="flex min-w-0 flex-col">
					<span class="truncate font-semibold text-contrast">{{ update.name }}</span>
					<span class="truncate text-secondary">
						{{ update.current_file_name }} -> {{ update.new_file_name }}
					</span>
				</div>
				<Button
					type="colored"
					color="brand"
					:disabled="updatingPath !== null"
					@click="applyUpdate(update)"
				>
					{{ updatingPath === update.project_path ? 'Updating...' : 'Update' }}
				</Button>
			</div>
		</Card>
	</div>
</template>
