<script setup lang="ts">
import { Button, injectNotificationManager, SkinPreviewRenderer } from '@modrinth/ui'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'
import { useRouter } from 'vue-router'

import { add_and_equip_custom_skin, type SkinModel } from '@/helpers/skins'

type Tool = 'pencil' | 'eraser' | 'fill' | 'picker'

const SIZE = 64
const ZOOM = 8
const HISTORY_LIMIT = 60
const SKIN_TONE = '#c69c6d'

const PALETTE = [
	'#000000',
	'#ffffff',
	'#c69c6d',
	'#8d5524',
	'#f1c27d',
	'#e0ac69',
	'#e23b3b',
	'#f08a24',
	'#f2d024',
	'#3ec45a',
	'#2f9be6',
	'#3a3fd1',
	'#8a3fd1',
	'#ec6cb4',
	'#6b6b6b',
	'#2b2b2b',
]

const REGIONS: { label: string; x: number; y: number; w: number; h: number }[] = [
	{ label: 'Head', x: 0, y: 0, w: 32, h: 16 },
	{ label: 'Hat', x: 32, y: 0, w: 32, h: 16 },
	{ label: 'R leg', x: 0, y: 16, w: 16, h: 16 },
	{ label: 'Body', x: 16, y: 16, w: 24, h: 16 },
	{ label: 'R arm', x: 40, y: 16, w: 16, h: 16 },
	{ label: 'R leg 2', x: 0, y: 32, w: 16, h: 16 },
	{ label: 'Jacket', x: 16, y: 32, w: 24, h: 16 },
	{ label: 'R sleeve', x: 40, y: 32, w: 16, h: 16 },
	{ label: 'L leg 2', x: 0, y: 48, w: 16, h: 16 },
	{ label: 'L leg', x: 16, y: 48, w: 16, h: 16 },
	{ label: 'L arm', x: 32, y: 48, w: 16, h: 16 },
	{ label: 'L sleeve', x: 48, y: 48, w: 16, h: 16 },
]

const BASE_REGIONS = [
	{ x: 0, y: 0, w: 32, h: 16 },
	{ x: 0, y: 16, w: 16, h: 16 },
	{ x: 16, y: 16, w: 24, h: 16 },
	{ x: 40, y: 16, w: 16, h: 16 },
	{ x: 16, y: 48, w: 16, h: 16 },
	{ x: 32, y: 48, w: 16, h: 16 },
]

const router = useRouter()
const { addNotification, handleError } = injectNotificationManager()

const display = useTemplateRef<HTMLCanvasElement>('display')
const fileInput = useTemplateRef<HTMLInputElement>('fileInput')

const texture = document.createElement('canvas')
texture.width = SIZE
texture.height = SIZE
const textureContext = texture.getContext('2d', { willReadFrequently: true })!

const tool = ref<Tool>('pencil')
const color = ref('#e23b3b')
const brushSize = ref(1)
const showGuide = ref(true)
const showGrid = ref(true)
const variant = ref<SkinModel>('CLASSIC')
const previewSrc = ref(texture.toDataURL('image/png'))
const hoverPixel = ref<{ x: number; y: number } | null>(null)
const busy = ref(false)

const undoStack = ref<ImageData[]>([])
const redoStack = ref<ImageData[]>([])
const canUndo = computed(() => undoStack.value.length > 0)
const canRedo = computed(() => redoStack.value.length > 0)

let drawing = false
let lastPixel: { x: number; y: number } | null = null
let previewTimer: ReturnType<typeof setTimeout> | undefined

function snapshot() {
	return textureContext.getImageData(0, 0, SIZE, SIZE)
}

function pushUndo() {
	undoStack.value.push(snapshot())
	if (undoStack.value.length > HISTORY_LIMIT) undoStack.value.shift()
	redoStack.value = []
}

function undo() {
	const previous = undoStack.value.pop()
	if (!previous) return
	redoStack.value.push(snapshot())
	textureContext.putImageData(previous, 0, 0)
	redraw()
}

function redo() {
	const next = redoStack.value.pop()
	if (!next) return
	undoStack.value.push(snapshot())
	textureContext.putImageData(next, 0, 0)
	redraw()
}

function schedulePreview() {
	if (previewTimer) return
	previewTimer = setTimeout(() => {
		previewTimer = undefined
		previewSrc.value = texture.toDataURL('image/png')
	}, 200)
}

function redraw() {
	const canvas = display.value
	if (!canvas) return
	const context = canvas.getContext('2d')!
	context.clearRect(0, 0, canvas.width, canvas.height)

	for (let y = 0; y < SIZE; y++) {
		for (let x = 0; x < SIZE; x++) {
			context.fillStyle = (x + y) % 2 === 0 ? '#3a3d44' : '#2f3238'
			context.fillRect(x * ZOOM, y * ZOOM, ZOOM, ZOOM)
		}
	}

	context.imageSmoothingEnabled = false
	context.drawImage(texture, 0, 0, SIZE * ZOOM, SIZE * ZOOM)

	if (showGrid.value) {
		context.strokeStyle = 'rgba(255,255,255,0.06)'
		context.lineWidth = 1
		context.beginPath()
		for (let i = 0; i <= SIZE; i++) {
			context.moveTo(i * ZOOM + 0.5, 0)
			context.lineTo(i * ZOOM + 0.5, SIZE * ZOOM)
			context.moveTo(0, i * ZOOM + 0.5)
			context.lineTo(SIZE * ZOOM, i * ZOOM + 0.5)
		}
		context.stroke()
	}

	if (showGuide.value) {
		context.strokeStyle = 'rgba(80,180,255,0.8)'
		context.fillStyle = 'rgba(255,255,255,0.9)'
		context.font = '11px sans-serif'
		context.lineWidth = 1
		for (const region of REGIONS) {
			context.strokeRect(
				region.x * ZOOM + 0.5,
				region.y * ZOOM + 0.5,
				region.w * ZOOM - 1,
				region.h * ZOOM - 1,
			)
			context.fillText(region.label, region.x * ZOOM + 4, region.y * ZOOM + 13)
		}
	}

	if (hoverPixel.value) {
		context.strokeStyle = '#ffffff'
		const half = Math.floor(brushSize.value / 2)
		context.strokeRect(
			(hoverPixel.value.x - half) * ZOOM + 0.5,
			(hoverPixel.value.y - half) * ZOOM + 0.5,
			brushSize.value * ZOOM - 1,
			brushSize.value * ZOOM - 1,
		)
	}
}

function hexToRgb(hex: string): [number, number, number] {
	const value = Number.parseInt(hex.slice(1), 16)
	return [(value >> 16) & 255, (value >> 8) & 255, value & 255]
}

function rgbToHex(r: number, g: number, b: number) {
	return `#${[r, g, b].map((channel) => channel.toString(16).padStart(2, '0')).join('')}`
}

function paintPixel(x: number, y: number) {
	const half = Math.floor(brushSize.value / 2)
	const startX = x - half
	const startY = y - half
	if (tool.value === 'eraser') {
		textureContext.clearRect(startX, startY, brushSize.value, brushSize.value)
	} else {
		textureContext.fillStyle = color.value
		textureContext.fillRect(startX, startY, brushSize.value, brushSize.value)
	}
}

function paintLine(from: { x: number; y: number }, to: { x: number; y: number }) {
	const steps = Math.max(Math.abs(to.x - from.x), Math.abs(to.y - from.y), 1)
	for (let step = 0; step <= steps; step++) {
		paintPixel(
			Math.round(from.x + ((to.x - from.x) * step) / steps),
			Math.round(from.y + ((to.y - from.y) * step) / steps),
		)
	}
}

function floodFill(startX: number, startY: number) {
	const image = snapshot()
	const data = image.data
	const index = (x: number, y: number) => (y * SIZE + x) * 4
	const origin = index(startX, startY)
	const target = [data[origin], data[origin + 1], data[origin + 2], data[origin + 3]]
	const [r, g, b] = hexToRgb(color.value)
	if (target[0] === r && target[1] === g && target[2] === b && target[3] === 255) return

	const matches = (position: number) =>
		data[position] === target[0] &&
		data[position + 1] === target[1] &&
		data[position + 2] === target[2] &&
		data[position + 3] === target[3]

	const stack: [number, number][] = [[startX, startY]]
	while (stack.length > 0) {
		const [x, y] = stack.pop()!
		if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) continue
		const position = index(x, y)
		if (!matches(position)) continue
		data[position] = r
		data[position + 1] = g
		data[position + 2] = b
		data[position + 3] = 255
		stack.push([x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1])
	}
	textureContext.putImageData(image, 0, 0)
}

function pixelAt(event: PointerEvent) {
	const canvas = display.value!
	const rect = canvas.getBoundingClientRect()
	const x = Math.floor(((event.clientX - rect.left) / rect.width) * SIZE)
	const y = Math.floor(((event.clientY - rect.top) / rect.height) * SIZE)
	return { x: Math.min(SIZE - 1, Math.max(0, x)), y: Math.min(SIZE - 1, Math.max(0, y)) }
}

function pickColor(x: number, y: number) {
	const [r, g, b, a] = textureContext.getImageData(x, y, 1, 1).data
	if (a === 0) return
	color.value = rgbToHex(r, g, b)
	tool.value = 'pencil'
}

function onPointerDown(event: PointerEvent) {
	if (event.button !== 0) return
	const pixel = pixelAt(event)
	if (tool.value === 'picker') {
		pickColor(pixel.x, pixel.y)
		return
	}
	pushUndo()
	if (tool.value === 'fill') {
		floodFill(pixel.x, pixel.y)
		redraw()
		schedulePreview()
		return
	}
	drawing = true
	lastPixel = pixel
	display.value?.setPointerCapture(event.pointerId)
	paintPixel(pixel.x, pixel.y)
	redraw()
	schedulePreview()
}

function onPointerMove(event: PointerEvent) {
	const pixel = pixelAt(event)
	hoverPixel.value = pixel
	if (drawing && lastPixel) {
		paintLine(lastPixel, pixel)
		lastPixel = pixel
		schedulePreview()
	}
	redraw()
}

function onPointerUp() {
	drawing = false
	lastPixel = null
	previewSrc.value = texture.toDataURL('image/png')
}

function onPointerLeave() {
	hoverPixel.value = null
	redraw()
}

function newTemplate() {
	pushUndo()
	textureContext.clearRect(0, 0, SIZE, SIZE)
	textureContext.fillStyle = SKIN_TONE
	for (const region of BASE_REGIONS) {
		textureContext.fillRect(region.x, region.y, region.w, region.h)
	}
	redraw()
	previewSrc.value = texture.toDataURL('image/png')
}

function clearAll() {
	pushUndo()
	textureContext.clearRect(0, 0, SIZE, SIZE)
	redraw()
	previewSrc.value = texture.toDataURL('image/png')
}

function loadImage(url: string) {
	return new Promise<HTMLImageElement>((resolve, reject) => {
		const image = new Image()
		image.onload = () => resolve(image)
		image.onerror = () => reject(new Error('This image could not be read.'))
		image.src = url
	})
}

async function onFileChosen(event: Event) {
	const input = event.target as HTMLInputElement
	const file = input.files?.[0]
	input.value = ''
	if (!file) return
	const url = URL.createObjectURL(file)
	try {
		const image = await loadImage(url)
		if (image.width !== 64 || (image.height !== 64 && image.height !== 32)) {
			throw new Error('A skin has to be a 64x64 PNG (or the old 64x32 format).')
		}
		pushUndo()
		textureContext.clearRect(0, 0, SIZE, SIZE)
		textureContext.drawImage(image, 0, 0)
		if (image.height === 32) {
			textureContext.drawImage(texture, 0, 16, 16, 16, 16, 48, 16, 16)
			textureContext.drawImage(texture, 40, 16, 16, 16, 32, 48, 16, 16)
		}
		redraw()
		previewSrc.value = texture.toDataURL('image/png')
	} catch (error) {
		handleError(error)
	} finally {
		URL.revokeObjectURL(url)
	}
}

function pngBytes(): Promise<Uint8Array> {
	return new Promise((resolve, reject) => {
		texture.toBlob(async (blob) => {
			if (!blob) {
				reject(new Error('The skin could not be exported.'))
				return
			}
			resolve(new Uint8Array(await blob.arrayBuffer()))
		}, 'image/png')
	})
}

async function exportPng() {
	busy.value = true
	try {
		const path = await save({
			title: 'Save skin',
			defaultPath: 'skin.png',
			filters: [{ name: 'PNG image', extensions: ['png'] }],
		})
		if (!path) return
		await writeFile(path, await pngBytes())
		addNotification({ type: 'success', title: 'Skin saved', text: path })
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}

async function useInMinecraft() {
	busy.value = true
	try {
		await add_and_equip_custom_skin(await pngBytes(), variant.value)
		addNotification({
			type: 'success',
			title: 'Skin applied',
			text: 'It was added to your skins and equipped.',
		})
		void router.push('/skins')
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}

function onKeydown(event: KeyboardEvent) {
	if (!(event.ctrlKey || event.metaKey)) return
	if (event.key.toLowerCase() === 'z') {
		event.preventDefault()
		if (event.shiftKey) redo()
		else undo()
	} else if (event.key.toLowerCase() === 'y') {
		event.preventDefault()
		redo()
	}
}

onMounted(() => {
	newTemplate()
	undoStack.value = []
	window.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
	window.removeEventListener('keydown', onKeydown)
	if (previewTimer) clearTimeout(previewTimer)
})

function toggleGuide() {
	showGuide.value = !showGuide.value
	redraw()
}

function toggleGrid() {
	showGrid.value = !showGrid.value
	redraw()
}

const tools: { id: Tool; label: string }[] = [
	{ id: 'pencil', label: 'Pencil' },
	{ id: 'eraser', label: 'Eraser' },
	{ id: 'fill', label: 'Fill' },
	{ id: 'picker', label: 'Pick color' },
]
</script>

<template>
	<div class="flex flex-col gap-4 p-6">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div>
				<h1 class="m-0 text-2xl font-semibold text-contrast">Skin creator</h1>
				<p class="m-0 mt-1 text-secondary">
					Paint on the skin texture. The 3D preview updates while you draw.
				</p>
			</div>
			<div class="flex flex-wrap gap-2">
				<Button type="outlined" @click="router.push('/skins')">Back to skins</Button>
				<Button type="outlined" :disabled="busy" @click="exportPng">Save as PNG</Button>
				<Button type="colored" color="brand" :disabled="busy" @click="useInMinecraft">
					Use in Minecraft
				</Button>
			</div>
		</div>

		<div class="flex flex-wrap items-start gap-6">
			<div class="flex flex-col gap-3">
				<div class="flex flex-wrap items-center gap-2">
					<Button
						v-for="entry in tools"
						:key="entry.id"
						:type="tool === entry.id ? 'colored' : 'outlined'"
						:color="tool === entry.id ? 'brand' : undefined"
						@click="tool = entry.id"
					>
						{{ entry.label }}
					</Button>
					<Button type="outlined" :disabled="!canUndo" @click="undo">Undo</Button>
					<Button type="outlined" :disabled="!canRedo" @click="redo">Redo</Button>
				</div>

				<canvas
					ref="display"
					:width="SIZE * ZOOM"
					:height="SIZE * ZOOM"
					class="max-w-full cursor-crosshair touch-none rounded-xl border border-solid border-surface-5"
					style="image-rendering: pixelated; width: 512px; height: 512px"
					@pointerdown="onPointerDown"
					@pointermove="onPointerMove"
					@pointerup="onPointerUp"
					@pointerleave="onPointerLeave"
				/>

				<div class="flex flex-wrap items-center gap-3">
					<input
						v-model="color"
						type="color"
						class="size-10 cursor-pointer rounded-lg border-none bg-transparent p-0"
						aria-label="Color"
					/>
					<div class="flex max-w-80 flex-wrap gap-1">
						<button
							v-for="swatch in PALETTE"
							:key="swatch"
							type="button"
							class="size-6 cursor-pointer rounded-md border border-solid border-white/20 p-0"
							:style="{ backgroundColor: swatch }"
							:aria-label="swatch"
							@click="color = swatch"
						/>
					</div>
				</div>

				<div class="flex flex-wrap items-center gap-4 text-sm text-secondary">
					<label class="flex items-center gap-2">
						Brush
						<select
							v-model.number="brushSize"
							class="rounded-lg border border-solid border-surface-5 bg-surface-3 p-1 text-contrast"
						>
							<option :value="1">1 px</option>
							<option :value="2">2 px</option>
							<option :value="3">3 px</option>
						</select>
					</label>
					<label class="flex items-center gap-2">
						Model
						<select
							v-model="variant"
							class="rounded-lg border border-solid border-surface-5 bg-surface-3 p-1 text-contrast"
						>
							<option value="CLASSIC">Classic (Steve)</option>
							<option value="SLIM">Slim (Alex)</option>
						</select>
					</label>
					<label class="flex items-center gap-2">
						<input type="checkbox" :checked="showGuide" @change="toggleGuide" />
						Body parts
					</label>
					<label class="flex items-center gap-2">
						<input type="checkbox" :checked="showGrid" @change="toggleGrid" />
						Grid
					</label>
				</div>

				<div class="flex flex-wrap gap-2">
					<Button type="outlined" @click="fileInput?.click()">Import PNG</Button>
					<Button type="outlined" @click="newTemplate">New from template</Button>
					<Button type="outlined" @click="clearAll">Clear</Button>
				</div>
				<input
					ref="fileInput"
					type="file"
					accept="image/png"
					class="hidden"
					@change="onFileChosen"
				/>
			</div>

			<div class="flex h-[560px] min-w-72 flex-1 items-center justify-center">
				<SkinPreviewRenderer
					:texture-src="previewSrc"
					:variant="variant"
					:initial-rotation="Math.PI / 8"
				/>
			</div>
		</div>
	</div>
</template>
