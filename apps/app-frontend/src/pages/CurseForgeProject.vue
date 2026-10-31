<template>
	<div v-if="mod" class="flex flex-col gap-4 p-6">
		<Teleport to="#sidebar-teleport-target">
			<ProjectSidebarLinks
				link-target="_blank"
				:project="sidebarProject"
				class="project-sidebar-section"
			/>
			<ProjectSidebarTags :project="sidebarProject" class="project-sidebar-section" />
			<ProjectSidebarCreators
				:organization="null"
				:members="members"
				:org-link="() => ''"
				:user-link="(username) => `https://www.curseforge.com/members/${encodeURIComponent(username)}`"
				link-target="_blank"
				class="project-sidebar-section"
			/>
		</Teleport>

		<ProjectPageHeader :project="headerProject" @category="goToCategory">
			<template #actions>
				<Button
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					:disabled="installing"
					@click="installLatest"
				>
					<DownloadIcon />
					{{ installing ? 'Installing...' : 'Install' }}
				</Button>
				<ButtonLink
					v-if="mod.links?.websiteUrl"
					v-tooltip="'Open on CurseForge'"
					type="quiet"
					size="xl"
					:href="mod.links.websiteUrl"
					target="_blank"
				>
					<ExternalIcon />
				</ButtonLink>
			</template>
		</ProjectPageHeader>

		<NavTabs :links="tabs" query="tab" />

		<Card v-if="activeTab === 'description'">
			<div v-if="loadingDescription" class="text-secondary">Loading description...</div>
			<ProjectPageDescription v-else :description="descriptionHtml" />
		</Card>

		<div v-else-if="activeTab === 'versions'">
			<div v-if="loadingFiles" class="text-secondary">Loading versions...</div>
			<ProjectPageVersions
				v-else
				:loaders="allLoaders"
				:game-versions="allGameVersions"
				:versions="versions"
				:project="{ id: headerProject.id, slug: mod.slug, project_type: projectType }"
			>
				<template #actions="{ version }">
					<IconButton
						v-tooltip="'Install'"
						type="quiet"
						color="green"
						label="Install"
						:disabled="installing"
						@click.stop="() => installVersion(version.id)"
					>
						<DownloadIcon />
					</IconButton>
				</template>
			</ProjectPageVersions>
		</div>

		<Card v-else-if="activeTab === 'gallery'">
			<div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr))">
				<a
					v-for="screenshot in mod.screenshots"
					:key="screenshot.url"
					:href="screenshot.url"
					target="_blank"
					class="block overflow-hidden rounded-xl"
				>
					<img :src="screenshot.thumbnailUrl ?? screenshot.url" alt="" class="w-full object-cover" />
				</a>
			</div>
		</Card>

		<ModalWrapper ref="instancePickerModal" header="Choose an instance">
			<div class="flex flex-col gap-3 pb-16">
				<p class="m-0 text-secondary">
					Install <strong>{{ mod.name }}</strong> into which instance?
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
						:disabled="!selectedInstanceId || installing"
						@click="confirmInstall"
					>
						{{ installing ? 'Installing...' : 'Install' }}
					</Button>
				</div>
			</div>
		</ModalWrapper>
	</div>
	<div v-else-if="loadFailed" class="p-6 text-secondary">Could not load this CurseForge project.</div>
</template>

<script setup lang="ts">
import { DownloadIcon, ExternalIcon } from '@modrinth/assets'
import {
	Button,
	ButtonLink,
	Card,
	IconButton,
	injectNotificationManager,
	NavTabs,
	ProjectPageDescription,
	ProjectPageHeader,
	ProjectPageVersions,
	ProjectSidebarCreators,
	ProjectSidebarLinks,
	ProjectSidebarTags,
} from '@modrinth/ui'
import DOMPurify from 'dompurify'
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import {
	CLASS_ID_MODPACKS,
	CLASS_ID_MODS,
	CLASS_ID_RESOURCE_PACKS,
	CLASS_ID_SHADER_PACKS,
	type CurseForgeFile,
	type CurseForgeMod,
	type CurseForgeModLoader,
	curseforgeGetFiles,
	curseforgeGetMod,
	curseforgeGetModDescription,
	curseforgeInstallContent,
	curseforgeInstallModpack,
} from '@/helpers/curseforge'
import { list as listInstances } from '@/helpers/instance'
import { get_game_versions, get_loaders } from '@/helpers/tags.js'
import type { GameInstance } from '@/helpers/types'

const { handleError } = injectNotificationManager()
const route = useRoute()
const router = useRouter()

const modId = Number(route.params.id)
const mod = ref<CurseForgeMod | null>(null)
const loadFailed = ref(false)
const files = ref<CurseForgeFile[]>([])
const loadingFiles = ref(true)
const descriptionHtml = ref('')
const loadingDescription = ref(true)
const allLoaders = ref([])
const allGameVersions = ref([])

const PROJECT_TYPE_BY_CLASS: Record<number, string> = {
	[CLASS_ID_MODS]: 'mod',
	[CLASS_ID_MODPACKS]: 'modpack',
	[CLASS_ID_RESOURCE_PACKS]: 'resourcepack',
	[CLASS_ID_SHADER_PACKS]: 'shader',
}
const KNOWN_LOADERS = ['forge', 'fabric', 'quilt', 'neoforge']

const projectType = computed(
	() => PROJECT_TYPE_BY_CLASS[mod.value?.classId ?? CLASS_ID_MODS] ?? 'mod',
)
const classId = computed(() => mod.value?.classId ?? CLASS_ID_MODS)

const activeTab = computed(() => {
	const tab = String(route.query.tab ?? 'description')
	return ['description', 'versions', 'gallery'].includes(tab) ? tab : 'description'
})

const tabs = computed(() => [
	{ label: 'Description', href: '' },
	{ label: 'Versions', href: 'versions' },
	{ label: 'Gallery', href: 'gallery', shown: (mod.value?.screenshots.length ?? 0) > 0 },
])

const headerProject = computed(() => {
	const current = mod.value!
	const icon = current.logo?.url ?? current.logo?.thumbnailUrl ?? null
	return {
		id: `curseforge-${current.id}`,
		slug: current.slug,
		project_type: projectType.value,
		title: current.name,
		description: current.summary,
		icon_url: icon,
		raw_icon_url: icon,
		downloads: Math.round(current.downloadCount),
		followers: 0,
		categories: current.categories.map((category) => category.name),
		additional_categories: [],
	}
})

const sidebarProject = computed(() => ({
	...headerProject.value,
	issues_url: mod.value?.links?.issuesUrl ?? '',
	source_url: mod.value?.links?.sourceUrl ?? '',
	wiki_url: mod.value?.links?.wikiUrl ?? '',
	discord_url: '',
	site_url: mod.value?.links?.websiteUrl ?? '',
	donation_urls: [],
}))

const members = computed(() =>
	(mod.value?.authors ?? []).map((author, index) => ({
		id: `author-${index}`,
		role: index === 0 ? 'Owner' : 'Member',
		is_owner: index === 0,
		accepted: true,
		user: { id: `author-${index}`, username: author.name, avatar_url: '' },
	})),
)

function releaseType(value: number) {
	if (value === 1) return 'release'
	if (value === 2) return 'beta'
	return 'alpha'
}

const versions = computed(() =>
	[...files.value]
		.sort((a, b) => (a.fileDate < b.fileDate ? 1 : -1))
		.map((file) => ({
			id: String(file.id),
			project_id: headerProject.value.id,
			author_id: '',
			featured: false,
			name: file.displayName,
			version_number: file.fileName.replace(/\.(jar|zip)$/i, ''),
			changelog: null,
			changelog_url: null,
			date_published: file.fileDate,
			downloads: Math.round(file.downloadCount ?? 0),
			version_type: releaseType(file.releaseType),
			status: 'listed',
			requested_status: null,
			files: [
				{
					hashes: { sha1: '', sha512: '' },
					url: file.downloadUrl ?? '',
					filename: file.fileName,
					primary: true,
					size: file.fileLength ?? 0,
					file_type: null,
				},
			],
			dependencies: [],
			loaders: file.gameVersions
				.map((value) => value.toLowerCase())
				.filter((value) => KNOWN_LOADERS.includes(value)),
			game_versions: file.gameVersions.filter((value) => /^\d/.test(value)),
			displayUrlEnding: String(file.id),
		})),
)

function goToCategory() {
	router.push(`/curseforge/${projectType.value}`)
}

async function load() {
	try {
		mod.value = await curseforgeGetMod(modId)
	} catch (error) {
		loadFailed.value = true
		handleError(error)
		return
	}

	curseforgeGetModDescription(modId)
		.then((html) => {
			descriptionHtml.value = DOMPurify.sanitize(html)
		})
		.catch(() => {
			descriptionHtml.value = ''
		})
		.finally(() => {
			loadingDescription.value = false
		})

	curseforgeGetFiles(modId)
		.then((result) => {
			files.value = result
		})
		.catch(() => {
			files.value = []
		})
		.finally(() => {
			loadingFiles.value = false
		})

	get_loaders()
		.then((result: never[]) => (allLoaders.value = result))
		.catch(() => {})
	get_game_versions()
		.then((result: never[]) => (allGameVersions.value = result))
		.catch(() => {})
}

const instances = ref<GameInstance[]>([])
const instancePickerModal = ref<InstanceType<typeof ModalWrapper>>()
const selectedInstanceId = ref('')
const pendingFileId = ref<number | null>(null)
const installing = ref(false)

async function installLatest() {
	await startInstall(null)
}

async function installVersion(versionId: string) {
	await startInstall(Number(versionId))
}

async function startInstall(fileId: number | null) {
	if (!mod.value) return

	if (classId.value === CLASS_ID_MODPACKS) {
		installing.value = true
		try {
			const instanceId = await curseforgeInstallModpack(
				mod.value.id,
				fileId ?? mod.value.mainFileId,
			)
			router.push(`/instance/${encodeURIComponent(instanceId)}`)
		} catch (error) {
			handleError(error)
		} finally {
			installing.value = false
		}
		return
	}

	pendingFileId.value = fileId
	if (instances.value.length === 0) {
		instances.value = await listInstances().catch(() => [])
	}
	selectedInstanceId.value = instances.value[0]?.id ?? ''
	instancePickerModal.value?.show()
}

async function confirmInstall() {
	if (!mod.value || !selectedInstanceId.value) return
	const instance = instances.value.find((entry) => entry.id === selectedInstanceId.value)
	if (!instance) return

	installing.value = true
	try {
		let fileId = pendingFileId.value
		if (fileId === null) {
			const compatible = await curseforgeGetFiles(
				mod.value.id,
				instance.game_version,
				instance.loader as CurseForgeModLoader,
			)
			const newest = [...compatible].sort((a, b) => (a.fileDate < b.fileDate ? 1 : -1))[0]
			if (!newest) {
				throw new Error('No compatible file found for this instance')
			}
			fileId = newest.id
		}
		await curseforgeInstallContent(instance.id, mod.value.id, fileId, classId.value)
		instancePickerModal.value?.hide()
	} catch (error) {
		handleError(error)
	} finally {
		installing.value = false
	}
}

void load()
</script>

<style scoped lang="scss">
.project-sidebar-section {
	@apply p-4 flex flex-col gap-2 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid;
}
</style>
