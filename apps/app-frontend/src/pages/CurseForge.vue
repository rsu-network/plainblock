<template>
	<div class="flex flex-col gap-2 p-6">
		<BrowsePageLayout />
		<Teleport to="#sidebar-teleport-target">
			<BrowseSidebar />
		</Teleport>
	</div>

	<ModalWrapper ref="instancePickerModal" header="Choose an instance">
		<div class="flex flex-col gap-3 pb-16">
			<p class="m-0 text-secondary">
				Install <strong>{{ selectedMod?.name }}</strong> into which instance?
			</p>
			<select
				v-model="selectedInstanceId"
				class="rounded-xl border border-solid border-surface-5 bg-surface-3 p-2.5 text-contrast"
			>
				<option v-for="instance in instances" :key="instance.id" :value="instance.id">
					{{ instance.name }} ({{ instance.game_version }}, {{ instance.loader }})
				</option>
			</select>
			<div class="absolute bottom-4 right-4 flex items-center gap-2">
				<Button type="outlined" @click="instancePickerModal?.hide()">Cancel</Button>
				<Button
					type="colored"
					color="brand"
					:disabled="!selectedInstanceId || confirmingInstall"
					@click="confirmInstall"
				>
					{{ confirmingInstall ? 'Installing...' : 'Install' }}
				</Button>
			</div>
		</div>
	</ModalWrapper>
</template>

<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import { DownloadIcon } from '@modrinth/assets'
import type { CardAction, ProjectType } from '@modrinth/ui'
import {
	Button,
	BrowsePageLayout,
	BrowseSidebar,
	injectNotificationManager,
	provideBrowseManager,
	useBrowseSearch,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import {
	CLASS_ID_MODPACKS,
	CLASS_ID_MODS,
	CLASS_ID_RESOURCE_PACKS,
	CLASS_ID_SHADER_PACKS,
	type CurseForgeCategory,
	type CurseForgeFile,
	type CurseForgeMod,
	type CurseForgeModLoader,
	curseforgeGetCategories,
	curseforgeGetFiles,
	curseforgeInstallContent,
	curseforgeInstallModpack,
	curseforgeSearch,
} from '@/helpers/curseforge'
import { list as listInstances } from '@/helpers/instance'
import { get_game_versions } from '@/helpers/tags'
import type { GameInstance } from '@/helpers/types'

const { handleError } = injectNotificationManager()
const route = useRoute()
const router = useRouter()

const CLASS_ID_BY_TYPE: Record<string, number> = {
	mod: CLASS_ID_MODS,
	modpack: CLASS_ID_MODPACKS,
	resourcepack: CLASS_ID_RESOURCE_PACKS,
	shader: CLASS_ID_SHADER_PACKS,
}

const projectType = computed<ProjectType>(() => (route.params.projectType as ProjectType) ?? 'mod')
const classId = computed(() => CLASS_ID_BY_TYPE[projectType.value] ?? CLASS_ID_MODS)

const CF_LOADER_BY_NAME: Record<string, CurseForgeModLoader> = {
	forge: 'forge',
	fabric: 'fabric',
	quilt: 'quilt',
	neoforge: 'neoforge',
}

const CF_SORT_BY_NAME: Record<string, number> = {
	relevance: 1,
	downloads: 6,
	follows: 2,
	newest: 3,
	updated: 3,
}

const categories = ref<CurseForgeCategory[]>([])
const categoryNameToId = computed(() => new Map(categories.value.map((c) => [c.name, c.id])))

async function loadCategories() {
	categories.value = await curseforgeGetCategories(classId.value).catch(() => [])
}

const tags = computed(() => ({
	gameVersions: gameVersions.value,
	loaders: [
		{ icon: 'forge', name: 'forge', supported_project_types: ['mod', 'modpack'] },
		{ icon: 'fabric', name: 'fabric', supported_project_types: ['mod', 'modpack'] },
		{ icon: 'quilt', name: 'quilt', supported_project_types: ['mod', 'modpack'] },
		{ icon: 'neoforge', name: 'neoforge', supported_project_types: ['mod', 'modpack'] },
	],
	categories: categories.value.map((c) => ({
		icon: '',
		name: c.name,
		project_type: projectType.value,
		header: 'categories',
	})),
}))

const gameVersions = ref<Labrinth.Tags.v2.GameVersion[]>([])
get_game_versions()
	.then((versions: Labrinth.Tags.v2.GameVersion[]) => {
		gameVersions.value = versions
	})
	.catch(() => {
		gameVersions.value = []
	})

function mapModToResult(mod: CurseForgeMod): Labrinth.Search.v3.ResultSearchProject {
	return {
		project_id: `curseforge:${mod.id}`,
		project_types: [projectType.value],
		all_project_types: [projectType.value],
		slug: mod.slug,
		author: mod.authors[0]?.name ?? '',
		author_id: null,
		organization: null,
		organization_id: null,
		name: mod.name,
		summary: mod.summary,
		categories: mod.categories.map((c) => c.name),
		display_categories: mod.categories.map((c) => c.name),
		downloads: mod.downloadCount,
		follows: 0,
		icon_url: mod.logo?.thumbnailUrl ?? mod.logo?.url ?? null,
		date_created: mod.dateModified,
		date_modified: mod.dateModified,
		license: '',
		gallery: mod.screenshots.map((s) => s.url),
		featured_gallery: mod.screenshots[0]?.url ?? null,
		color: null,
		loaders: [],
		disclosure_types: [],
	} as unknown as Labrinth.Search.v3.ResultSearchProject
}

const curseforgeModById = new Map<string, CurseForgeMod>()

async function search(): Promise<{
	projectHits: Labrinth.Search.v3.ResultSearchProject[]
	serverHits: Labrinth.Search.v3.ResultSearchProject[]
	total_hits: number
	per_page: number
}> {
	const categoryFilter = browseState.currentFilters.value.find((f) =>
		f.type.startsWith('category_'),
	)
	const loaderFilter = browseState.currentFilters.value.find(
		(f) => f.type === 'mod_loader' || f.type === 'modpack_loader',
	)

	const response = await curseforgeSearch(
		browseState.query.value,
		classId.value,
		undefined,
		loaderFilter ? CF_LOADER_BY_NAME[loaderFilter.option] : undefined,
		categoryFilter ? categoryNameToId.value.get(categoryFilter.option) : undefined,
		CF_SORT_BY_NAME[browseState.effectiveCurrentSortType.value.name] ?? 1,
		'desc',
		(browseState.currentPage.value - 1) * browseState.maxResults.value,
		browseState.maxResults.value,
	)

	for (const mod of response.data) {
		curseforgeModById.set(`curseforge:${mod.id}`, mod)
	}

	return {
		projectHits: response.data.map(mapModToResult),
		serverHits: [],
		total_hits: response.pagination.totalCount,
		per_page: browseState.maxResults.value,
	}
}

const browseState = useBrowseSearch({
	projectType,
	tags,
	search,
	persistentQueryParams: [],
})

watch(projectType, () => {
	categories.value = []
	loadCategories()
})
loadCategories()

const selectableProjectTypes = computed(() => [
	{ label: 'Mods', href: '/curseforge/mod' },
	{ label: 'Modpacks', href: '/curseforge/modpack' },
	{ label: 'Resource Packs', href: '/curseforge/resourcepack' },
	{ label: 'Shaders', href: '/curseforge/shader' },
])

const instances = ref<GameInstance[]>([])
const instancePickerModal = ref<InstanceType<typeof ModalWrapper>>()
const selectedMod = ref<CurseForgeMod | null>(null)
const selectedFile = ref<CurseForgeFile | null>(null)
const selectedInstanceId = ref<string>('')
const installingId = ref<number | null>(null)
const confirmingInstall = ref(false)

async function startInstall(result: Labrinth.Search.v3.ResultSearchProject) {
	const mod = curseforgeModById.get(result.project_id)
	if (!mod) return

	if (classId.value === CLASS_ID_MODPACKS) {
		installingId.value = mod.id
		try {
			const instanceId = await curseforgeInstallModpack(mod.id, mod.mainFileId)
			router.push(`/instance/${encodeURIComponent(instanceId)}`)
		} catch (err) {
			handleError(err)
		} finally {
			installingId.value = null
		}
		return
	}

	await openInstancePicker(mod, null)
}

async function openInstancePicker(mod: CurseForgeMod, file: CurseForgeFile | null) {
	selectedMod.value = mod
	selectedFile.value = file
	if (instances.value.length === 0) {
		instances.value = await listInstances().catch(() => [])
	}
	selectedInstanceId.value = instances.value[0]?.id ?? ''
	instancePickerModal.value?.show()
}

async function confirmInstall() {
	if (!selectedMod.value || !selectedInstanceId.value) return
	const instance = instances.value.find((i) => i.id === selectedInstanceId.value)
	if (!instance) return

	confirmingInstall.value = true
	try {
		let file = selectedFile.value
		if (!file) {
			const files = await curseforgeGetFiles(
				selectedMod.value.id,
				instance.game_version,
				instance.loader as CurseForgeModLoader,
			)
			file = [...files].sort((a, b) => (a.fileDate < b.fileDate ? 1 : -1))[0] ?? null
		}
		if (!file) {
			throw new Error('No compatible file found for this instance')
		}
		await curseforgeInstallContent(instance.id, selectedMod.value.id, file.id, classId.value)
		instancePickerModal.value?.hide()
		} catch (err) {
		handleError(err)
	} finally {
		confirmingInstall.value = false
	}
}

function getCardActions(result: Labrinth.Search.v3.ResultSearchProject): CardAction[] {
	return [
		{
			key: 'install',
			label: installingId.value === curseforgeModById.get(result.project_id)?.id
				? 'Installing...'
				: 'Install',
			icon: DownloadIcon,
			color: 'brand',
			type: 'standard',
			disabled: installingId.value === curseforgeModById.get(result.project_id)?.id,
			onClick: () => startInstall(result),
		},
	]
}

provideBrowseManager({
	tags,
	projectType,
	query: browseState.query,
	filters: browseState.filters,
	currentFilters: browseState.currentFilters,
	toggledGroups: browseState.toggledGroups,
	overriddenProvidedFilterTypes: browseState.overriddenProvidedFilterTypes,
	serverFilterTypes: computed(() => []),
	serverCurrentFilters: browseState.serverCurrentFilters,
	serverToggledGroups: browseState.serverToggledGroups,
	effectiveSortTypes: browseState.effectiveSortTypes,
	effectiveCurrentSortType: browseState.effectiveCurrentSortType,
	loading: browseState.loading,
	refreshing: browseState.refreshing,
	projectHits: browseState.projectHits,
	serverHits: browseState.serverHits,
	totalHits: browseState.totalHits,
	pageCount: browseState.pageCount,
	maxResults: browseState.maxResults,
	currentPage: browseState.currentPage,
	isServerType: computed(() => false),
	effectiveLayout: browseState.effectiveLayout,
	deprioritizedTags: browseState.deprioritizedTags,
	excludeLoaders: browseState.excludeLoaders,
	refreshSearch: browseState.refreshSearch,
	setPage: browseState.setPage,
	clearSearch: browseState.clearSearch,
	onFilterChange: browseState.onFilterChange,
	linkOverridesAdvancedPrefs: browseState.linkOverridesAdvancedPrefs,
	applySavedAdvancedPrefs: browseState.applySavedAdvancedPrefs,
	getProjectLink: (result) => ({
		path: `/curseforge/project/${result.project_id.replace('curseforge:', '')}`,
	}),
	getServerProjectLink: () => '',
	selectableProjectTypes,
	showProjectTypeTabs: computed(() => true),
	variant: 'app',
	getCardActions,
	hiddenFilterTypes: computed(() => ['compatible_dependency_project_ids', 'environment']),
})

void browseState.refreshSearch()
</script>
