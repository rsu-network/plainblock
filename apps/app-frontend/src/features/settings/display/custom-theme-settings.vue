<script setup lang="ts">
import {
	EditIcon,
	PlusIcon,
	RadioButtonCheckedIcon,
	RadioButtonIcon,
	TrashIcon,
} from '@modrinth/assets'
import { Button, Combobox, Input, injectNotificationManager, Slider } from '@modrinth/ui'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import {
	type CustomTheme,
	type CustomThemeColors,
	createThemeId,
	DEFAULT_THEME_COLORS,
	FONT_OPTIONS,
	parseThemeJson,
	PRESET_THEMES,
	previewTheme,
	themeToJson,
	applyActiveCustomTheme,
	useCustomThemes,
} from '@/composables/use-custom-themes.ts'
import { copyToClipboard } from '@/helpers/utils.js'

const { addNotification } = injectNotificationManager()
const {
	customThemes,
	activeCustomThemeId,
	upsertCustomTheme,
	deleteCustomTheme,
	setActiveCustomTheme,
} = useCustomThemes()

const COLOR_FIELDS: { key: keyof CustomThemeColors; label: string; hint: string }[] = [
	{ key: 'brand', label: 'Accent', hint: 'Buttons, highlights and links' },
	{ key: 'background', label: 'Background', hint: 'Main page background' },
	{ key: 'raised', label: 'Panels', hint: 'Sidebars and raised areas' },
	{ key: 'cards', label: 'Cards and buttons', hint: 'Cards, inputs, buttons' },
	{ key: 'border', label: 'Borders', hint: 'Lines and dividers' },
	{ key: 'text', label: 'Text', hint: 'Headings and main text' },
	{ key: 'secondaryText', label: 'Secondary text', hint: 'Descriptions and hints' },
]

const editing = ref<CustomTheme | null>(null)
const importText = ref('')
const showImport = ref(false)

const isEditingExisting = computed(
	() => !!editing.value && customThemes.some((theme) => theme.id === editing.value?.id),
)

const fontOptions = FONT_OPTIONS.map((option) => ({
	value: option.value ?? '__default',
	label: option.label,
}))

const editingFont = computed({
	get: () => editing.value?.font ?? '__default',
	set: (value: string) => {
		if (editing.value) editing.value.font = value === '__default' ? null : value
	},
})

watch(
	editing,
	(theme) => {
		if (theme) previewTheme(theme)
	},
	{ deep: true },
)

function stopEditing() {
	editing.value = null
	applyActiveCustomTheme()
}

onBeforeUnmount(() => {
	if (editing.value) applyActiveCustomTheme()
})

function startCreate() {
	editing.value = {
		id: createThemeId(),
		name: '',
		colors: { ...DEFAULT_THEME_COLORS },
		radius: 1,
		font: null,
	}
}

function startEdit(theme: CustomTheme) {
	editing.value = JSON.parse(JSON.stringify(theme))
}

function usePreset(preset: Omit<CustomTheme, 'id'>) {
	editing.value = {
		id: createThemeId(),
		name: preset.name,
		colors: { ...preset.colors },
		radius: preset.radius,
		font: preset.font,
	}
}

function duplicate(theme: CustomTheme) {
	editing.value = {
		...JSON.parse(JSON.stringify(theme)),
		id: createThemeId(),
		name: `${theme.name} copy`,
	}
}

function save() {
	if (!editing.value || !editing.value.name.trim()) return
	const theme = { ...editing.value, name: editing.value.name.trim() }
	upsertCustomTheme(theme)
	if (!activeCustomThemeId.value) setActiveCustomTheme(theme.id)
	editing.value = null
	applyActiveCustomTheme()
}

function toggleActive(id: string) {
	setActiveCustomTheme(activeCustomThemeId.value === id ? null : id)
}

function remove(id: string) {
	deleteCustomTheme(id)
	if (editing.value?.id === id) stopEditing()
}

async function exportTheme(theme: CustomTheme) {
	await copyToClipboard(themeToJson(theme))
	addNotification({
		type: 'success',
		title: 'Theme copied',
		text: 'The theme was copied to your clipboard. Share it with others.',
	})
}

function importTheme() {
	const theme = parseThemeJson(importText.value)
	if (!theme) {
		addNotification({
			type: 'error',
			title: 'Invalid theme',
			text: 'This text is not a valid Plainblock theme.',
		})
		return
	}
	upsertCustomTheme(theme)
	importText.value = ''
	showImport.value = false
}
</script>

<template>
	<section class="mt-8 border-0 border-t border-solid border-surface-4 pt-6">
		<div class="flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-xl font-semibold text-contrast">Custom themes</h2>
				<p class="m-0 mt-1">
					Change colors, corner roundness and font. The theme is applied on top of the base theme
					you picked above.
				</p>
			</div>
			<div v-if="!editing" class="flex gap-2">
				<Button type="outlined" @click="showImport = !showImport">Import</Button>
				<Button type="colored" color="brand" @click="startCreate">
					<PlusIcon />
					New theme
				</Button>
			</div>
		</div>

		<div v-if="showImport && !editing" class="mt-4 flex flex-col gap-2">
			<textarea
				v-model="importText"
				rows="6"
				placeholder="Paste a shared theme here..."
				class="w-full rounded-xl border border-solid border-surface-5 bg-surface-3 p-3 font-mono text-sm text-contrast"
			/>
			<div class="flex gap-2">
				<Button type="colored" color="brand" :disabled="!importText.trim()" @click="importTheme">
					Import theme
				</Button>
				<Button type="outlined" @click="showImport = false">Cancel</Button>
			</div>
		</div>

		<div v-if="!editing" class="mt-4">
			<h3 class="m-0 text-base font-semibold text-contrast">Start from a preset</h3>
			<div class="mt-2 flex flex-wrap gap-2">
				<button
					v-for="preset in PRESET_THEMES"
					:key="preset.name"
					type="button"
					class="flex cursor-pointer items-center gap-2 rounded-xl border border-solid border-surface-5 bg-surface-3 px-3 py-2 text-contrast hover:bg-surface-4"
					@click="usePreset(preset)"
				>
					<span class="flex">
						<span
							v-for="color in [preset.colors.brand, preset.colors.background, preset.colors.cards]"
							:key="color"
							class="-ml-1 size-4 rounded-full border border-solid border-white/20 first:ml-0"
							:style="{ backgroundColor: color }"
						/>
					</span>
					{{ preset.name }}
				</button>
			</div>
		</div>

		<div
			v-if="customThemes.length > 0 && !editing"
			class="mt-6 grid gap-3"
			style="grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr))"
		>
			<div
				v-for="theme in customThemes"
				:key="theme.id"
				class="flex cursor-pointer flex-col gap-2 rounded-2xl border border-solid p-3"
				:class="activeCustomThemeId === theme.id ? 'border-brand bg-surface-3' : 'border-surface-5'"
				@click="toggleActive(theme.id)"
			>
				<div class="flex items-center justify-between gap-2">
					<div class="flex min-w-0 items-center gap-2">
						<RadioButtonCheckedIcon
							v-if="activeCustomThemeId === theme.id"
							class="shrink-0 text-brand"
						/>
						<RadioButtonIcon v-else class="shrink-0 text-secondary" />
						<span class="truncate font-semibold text-contrast">{{ theme.name }}</span>
					</div>
					<div class="flex shrink-0 items-center gap-1">
						<button
							class="cursor-pointer border-none bg-transparent p-1 text-secondary hover:text-contrast"
							title="Edit"
							@click.stop="startEdit(theme)"
						>
							<EditIcon />
						</button>
						<button
							class="cursor-pointer border-none bg-transparent p-1 text-xs text-secondary hover:text-contrast"
							title="Duplicate"
							@click.stop="duplicate(theme)"
						>
							Copy
						</button>
						<button
							class="cursor-pointer border-none bg-transparent p-1 text-xs text-secondary hover:text-contrast"
							title="Copy theme to share"
							@click.stop="exportTheme(theme)"
						>
							Share
						</button>
						<button
							class="cursor-pointer border-none bg-transparent p-1 text-secondary hover:text-red"
							title="Delete"
							@click.stop="remove(theme.id)"
						>
							<TrashIcon />
						</button>
					</div>
				</div>
				<div class="flex items-center gap-1">
					<span
						v-for="field in COLOR_FIELDS"
						:key="field.key"
						class="size-5 rounded-full border border-solid border-white/20"
						:style="{ backgroundColor: theme.colors[field.key] }"
						:title="field.label"
					/>
				</div>
			</div>
		</div>

		<div
			v-if="editing"
			class="mt-4 flex flex-col gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-2 p-4"
		>
			<p class="m-0 text-secondary">
				You see your changes live. Nothing is saved until you press Save.
			</p>

			<div class="flex flex-col gap-1">
				<label class="text-sm font-semibold text-contrast" for="custom-theme-name">Name</label>
				<Input
					id="custom-theme-name"
					v-model="editing.name"
					placeholder="e.g. My theme"
					class="max-w-72"
				/>
			</div>

			<div class="grid gap-3" style="grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr))">
				<div v-for="field in COLOR_FIELDS" :key="field.key" class="flex items-center gap-3">
					<input
						v-model="editing.colors[field.key]"
						type="color"
						class="size-10 shrink-0 cursor-pointer rounded-lg border-none bg-transparent p-0"
					/>
					<div class="flex flex-col">
						<span class="text-sm font-semibold text-contrast">{{ field.label }}</span>
						<span class="text-xs text-secondary">{{ field.hint }}</span>
					</div>
				</div>
			</div>

			<div class="flex flex-col gap-1">
				<span class="text-sm font-semibold text-contrast">
					Corner roundness ({{ editing.radius.toFixed(1) }}x)
				</span>
				<Slider v-model="editing.radius" :min="0" :max="2.5" :step="0.1" class="max-w-96" />
			</div>

			<div class="flex flex-col gap-1">
				<span class="text-sm font-semibold text-contrast">Font</span>
				<Combobox v-model="editingFont" :options="fontOptions" name="Font" class="max-w-72" />
			</div>

			<div class="flex items-center gap-2">
				<Button type="outlined" @click="stopEditing">Cancel</Button>
				<div class="flex-grow" />
				<Button type="colored" color="brand" :disabled="!editing.name.trim()" @click="save">
					{{ isEditingExisting ? 'Save changes' : 'Create theme' }}
				</Button>
			</div>
		</div>
	</section>
</template>
