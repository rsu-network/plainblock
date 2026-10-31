import { reactive, ref } from 'vue'

export type CustomThemeColors = {
	brand: string
	background: string
	raised: string
	cards: string
	border: string
	text: string
	secondaryText: string
}

export type CustomTheme = {
	id: string
	name: string
	colors: CustomThemeColors
	/** Multiplier for every corner radius. 0 = sharp, 1 = default, 2 = very round. */
	radius: number
	font: string | null
}

export const FONT_OPTIONS: { value: string | null; label: string }[] = [
	{ value: null, label: 'Default (Inter)' },
	{ value: "system-ui, -apple-system, 'Segoe UI', sans-serif", label: 'System' },
	{ value: "'Trebuchet MS', 'Segoe UI', sans-serif", label: 'Humanist' },
	{ value: "Verdana, 'Segoe UI', sans-serif", label: 'Wide' },
	{ value: "Georgia, 'Times New Roman', serif", label: 'Serif' },
	{ value: "Consolas, 'Courier New', monospace", label: 'Monospace' },
]

export const DEFAULT_THEME_COLORS: CustomThemeColors = {
	brand: '#1bd96a',
	background: '#16181c',
	raised: '#27292e',
	cards: '#34363c',
	border: '#42444a',
	text: '#ecedf1',
	secondaryText: '#9a9fab',
}

export const PRESET_THEMES: Omit<CustomTheme, 'id'>[] = [
	{
		name: 'Midnight blue',
		colors: {
			brand: '#4f9cff',
			background: '#0e1320',
			raised: '#171e30',
			cards: '#212a42',
			border: '#2f3a58',
			text: '#e8eeff',
			secondaryText: '#93a0c4',
		},
		radius: 1,
		font: null,
	},
	{
		name: 'Sunset',
		colors: {
			brand: '#ff8a4c',
			background: '#1c1416',
			raised: '#2a1d20',
			cards: '#3a282c',
			border: '#4f3539',
			text: '#fff1ea',
			secondaryText: '#c9a79b',
		},
		radius: 1.3,
		font: null,
	},
	{
		name: 'Forest',
		colors: {
			brand: '#7bd96a',
			background: '#101712',
			raised: '#18231b',
			cards: '#223127',
			border: '#2f4337',
			text: '#e9f5ea',
			secondaryText: '#97b09b',
		},
		radius: 0.8,
		font: null,
	},
	{
		name: 'Rose',
		colors: {
			brand: '#ff6fa8',
			background: '#1a1218',
			raised: '#271a24',
			cards: '#36232f',
			border: '#4a3041',
			text: '#ffeef6',
			secondaryText: '#c7a3b7',
		},
		radius: 1.6,
		font: null,
	},
	{
		name: 'Terminal',
		colors: {
			brand: '#33ff66',
			background: '#050805',
			raised: '#0b120b',
			cards: '#121c12',
			border: '#1e301e',
			text: '#c8ffd4',
			secondaryText: '#6aa97a',
		},
		radius: 0,
		font: "Consolas, 'Courier New', monospace",
	},
	{
		name: 'Paper',
		colors: {
			brand: '#2a8f5a',
			background: '#ececea',
			raised: '#f6f6f4',
			cards: '#ffffff',
			border: '#d6d6d2',
			text: '#1d2125',
			secondaryText: '#5b6168',
		},
		radius: 1,
		font: null,
	},
]

const CUSTOM_THEMES_STORAGE_KEY = 'plainblock-custom-themes'
const ACTIVE_CUSTOM_THEME_STORAGE_KEY = 'plainblock-active-custom-theme'

const MANAGED_PROPERTIES = [
	'--surface-1',
	'--surface-1-5',
	'--surface-2',
	'--surface-2-5',
	'--surface-3',
	'--surface-4',
	'--surface-5',
	'--color-text-primary',
	'--color-text-default',
	'--color-text-tertiary',
	'--color-brand',
	'--color-brand-highlight',
	'--color-brand-shadow',
	'--color-accent-contrast',
	'--radius-xs',
	'--radius-sm',
	'--radius-md',
	'--radius-lg',
	'--radius-xl',
	'--font-standard',
	'--brand-gradient-bg',
	'--brand-gradient-strong-bg',
	'--brand-gradient-button',
	'--brand-gradient-border',
	'--brand-gradient-fade-out-color',
	'--splash-tint-top',
	'--splash-tint-bottom',
	'--splash-overlay',
	'--splash-cube-opacity',
]

const BASE_RADII: Record<string, number> = {
	'--radius-xs': 0.25,
	'--radius-sm': 0.5,
	'--radius-md': 0.75,
	'--radius-lg': 1,
	'--radius-xl': 1.25,
}

function parseHex(hex: string): [number, number, number] | null {
	const normalized = hex.replace('#', '')
	if (normalized.length !== 6) return null
	const value = Number.parseInt(normalized, 16)
	if (Number.isNaN(value)) return null
	return [(value >> 16) & 255, (value >> 8) & 255, value & 255]
}

function toHex([r, g, b]: [number, number, number]): string {
	return `#${[r, g, b].map((channel) => Math.round(channel).toString(16).padStart(2, '0')).join('')}`
}

function mix(from: string, to: string, amount: number): string {
	const a = parseHex(from)
	const b = parseHex(to)
	if (!a || !b) return from
	return toHex([
		a[0] + (b[0] - a[0]) * amount,
		a[1] + (b[1] - a[1]) * amount,
		a[2] + (b[2] - a[2]) * amount,
	])
}

function rgba(hex: string, alpha: number): string {
	const rgb = parseHex(hex)
	if (!rgb) return hex
	return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${alpha})`
}

function readableOn(hex: string): string {
	const rgb = parseHex(hex)
	if (!rgb) return '#000000'
	const luminance = (0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]) / 255
	return luminance > 0.6 ? '#000000' : '#ffffff'
}

function loadCustomThemes(): CustomTheme[] {
	try {
		const raw = localStorage.getItem(CUSTOM_THEMES_STORAGE_KEY)
		return raw ? JSON.parse(raw) : []
	} catch {
		return []
	}
}

function loadActiveId(): string | null {
	try {
		return localStorage.getItem(ACTIVE_CUSTOM_THEME_STORAGE_KEY)
	} catch {
		return null
	}
}

const DEFAULT_THEME_ID = 'plainblock-midnight-blue'
const SEEDED_STORAGE_KEY = 'plainblock-default-theme-seeded'

const customThemes = reactive<CustomTheme[]>(loadCustomThemes())
const activeCustomThemeId = ref<string | null>(loadActiveId())

function seedDefaultTheme() {
	try {
		if (localStorage.getItem(SEEDED_STORAGE_KEY)) return
		localStorage.setItem(SEEDED_STORAGE_KEY, '1')
	} catch {
		return
	}
	const preset = PRESET_THEMES.find((theme) => theme.name === 'Midnight blue')
	if (!preset || customThemes.some((theme) => theme.id === DEFAULT_THEME_ID)) return
	customThemes.unshift({ ...preset, id: DEFAULT_THEME_ID })
	if (!activeCustomThemeId.value) activeCustomThemeId.value = DEFAULT_THEME_ID
	persist()
}

function persist() {
	try {
		localStorage.setItem(CUSTOM_THEMES_STORAGE_KEY, JSON.stringify(customThemes))
		if (activeCustomThemeId.value) {
			localStorage.setItem(ACTIVE_CUSTOM_THEME_STORAGE_KEY, activeCustomThemeId.value)
		} else {
			localStorage.removeItem(ACTIVE_CUSTOM_THEME_STORAGE_KEY)
		}
	} catch {
		// storage blocked or full
	}
}

function clearOverrides() {
	const style = document.documentElement.style
	for (const property of MANAGED_PROPERTIES) {
		style.removeProperty(property)
	}
}

/** Applies a theme to the whole app without saving it. Pass null to go back to the base theme. */
export function previewTheme(theme: Pick<CustomTheme, 'colors' | 'radius' | 'font'> | null) {
	clearOverrides()
	if (!theme) return

	const style = document.documentElement.style
	const { colors } = theme

	style.setProperty('--surface-1', colors.background)
	style.setProperty('--surface-1-5', mix(colors.background, colors.raised, 0.2))
	style.setProperty('--surface-2', mix(colors.background, colors.raised, 0.5))
	style.setProperty('--surface-2-5', mix(colors.background, colors.raised, 0.8))
	style.setProperty('--surface-3', colors.raised)
	style.setProperty('--surface-4', colors.cards)
	style.setProperty('--surface-5', colors.border)

	style.setProperty('--color-text-primary', colors.text)
	style.setProperty('--color-text-default', mix(colors.text, colors.secondaryText, 0.25))
	style.setProperty('--color-text-tertiary', colors.secondaryText)

	style.setProperty('--color-brand', colors.brand)
	style.setProperty('--color-brand-highlight', rgba(colors.brand, 0.25))
	style.setProperty('--color-brand-shadow', rgba(colors.brand, 0.7))
	style.setProperty('--color-accent-contrast', readableOn(colors.brand))

	const sidebar = mix(colors.background, colors.raised, 0.6)
	style.setProperty('--brand-gradient-bg', sidebar)
	style.setProperty('--brand-gradient-strong-bg', sidebar)
	style.setProperty('--brand-gradient-button', rgba(colors.text, 0.08))
	style.setProperty('--brand-gradient-border', rgba(colors.text, 0.1))
	style.setProperty(
		'--brand-gradient-fade-out-color',
		`linear-gradient(to bottom, ${rgba(sidebar, 0)}, ${sidebar} 80%)`,
	)

	style.setProperty('--splash-tint-top', rgba(colors.brand, 0.22))
	style.setProperty('--splash-tint-bottom', rgba(colors.raised, 0.5))
	style.setProperty('--splash-overlay', rgba(colors.background, 0.64))
	style.setProperty('--splash-cube-opacity', '0.1')

	for (const [property, base] of Object.entries(BASE_RADII)) {
		style.setProperty(property, `${(base * theme.radius).toFixed(3)}rem`)
	}

	if (theme.font) {
		style.setProperty('--font-standard', theme.font)
	}
}

export function applyActiveCustomTheme() {
	const active = activeCustomThemeId.value
		? customThemes.find((theme) => theme.id === activeCustomThemeId.value)
		: null
	previewTheme(active ?? null)
}

function upsertCustomTheme(theme: CustomTheme) {
	const index = customThemes.findIndex((existing) => existing.id === theme.id)
	if (index === -1) {
		customThemes.push(theme)
	} else {
		customThemes[index] = theme
	}
	persist()
	applyActiveCustomTheme()
}

function deleteCustomTheme(id: string) {
	const index = customThemes.findIndex((theme) => theme.id === id)
	if (index !== -1) customThemes.splice(index, 1)
	if (activeCustomThemeId.value === id) activeCustomThemeId.value = null
	persist()
	applyActiveCustomTheme()
}

function setActiveCustomTheme(id: string | null) {
	activeCustomThemeId.value = id
	persist()
	applyActiveCustomTheme()
}

export function createThemeId(): string {
	if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
		return crypto.randomUUID()
	}
	return `${Date.now()}-${Math.random().toString(36).slice(2)}`
}

const HEX = /^#[0-9a-fA-F]{6}$/

/** Parses shared theme JSON. Returns null if it is not a valid theme. */
export function parseThemeJson(text: string): CustomTheme | null {
	try {
		const data = JSON.parse(text)
		if (!data || typeof data !== 'object' || typeof data.name !== 'string') return null
		const colors = data.colors as Partial<CustomThemeColors> | undefined
		if (!colors) return null
		const merged = { ...DEFAULT_THEME_COLORS, ...colors }
		for (const value of Object.values(merged)) {
			if (typeof value !== 'string' || !HEX.test(value)) return null
		}
		const radius = Number(data.radius ?? 1)
		return {
			id: createThemeId(),
			name: data.name.slice(0, 60),
			colors: merged,
			radius: Number.isFinite(radius) ? Math.min(Math.max(radius, 0), 2.5) : 1,
			font: FONT_OPTIONS.some((option) => option.value === data.font) ? data.font : null,
		}
	} catch {
		return null
	}
}

export function themeToJson(theme: CustomTheme): string {
	return JSON.stringify(
		{ name: theme.name, colors: theme.colors, radius: theme.radius, font: theme.font },
		null,
		2,
	)
}

export function useCustomThemes() {
	return {
		customThemes,
		activeCustomThemeId,
		upsertCustomTheme,
		deleteCustomTheme,
		setActiveCustomTheme,
	}
}

seedDefaultTheme()
applyActiveCustomTheme()
