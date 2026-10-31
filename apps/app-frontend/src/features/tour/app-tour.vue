<script setup lang="ts">
import {
	BoxesIcon,
	CompassIcon,
	HistoryIcon,
	PaintbrushIcon,
	PlayIcon,
	TerminalSquareIcon,
	XIcon,
} from '@modrinth/assets'
import { Button } from '@modrinth/ui'
import { type Component, computed, onMounted, ref, watch } from 'vue'

import { resetTour, tourDone, tourRequested } from '@/features/tour/tour-state'

type TourStep = {
	icon: Component
	title: string
	text: string
	points: string[]
}

const steps: TourStep[] = [
	{
		icon: PlayIcon,
		title: 'Welcome to Plainblock',
		text: 'A launcher that does what it should and nothing else.',
		points: [
			'No ads and no tracking. Nothing is collected about you.',
			'Everything you do stays on your computer.',
			'This short tour shows you around. You can skip it at any time.',
		],
	},
	{
		icon: BoxesIcon,
		title: 'Instances',
		text: 'An instance is one separate Minecraft setup with its own mods, worlds and settings.',
		points: [
			'Press "New instance" in the library to create one.',
			'Press "Play" to start it. The first start downloads what is missing.',
			'Click an instance to manage its content, worlds, files and logs.',
		],
	},
	{
		icon: CompassIcon,
		title: 'Mods from Modrinth and CurseForge',
		text: 'Both sites are built in, so you do not need a browser.',
		points: [
			'The compass opens Modrinth. The package icon below it opens CurseForge.',
			'CurseForge pages look and work like Modrinth pages.',
			'Install into an existing instance or install a whole modpack as a new one.',
		],
	},
	{
		icon: HistoryIcon,
		title: 'Backups and tools',
		text: 'Open an instance and choose "Backups & tools".',
		points: [
			'The mod check warns you before the game starts if mods are missing or clash.',
			'"Optimize" picks memory and Java settings that fit your computer.',
			'Worlds are backed up automatically before the game starts.',
			'Roll a world back with one click. The current state is saved first.',
			'Mods installed from CurseForge can be updated from the same page.',
		],
	},
	{
		icon: TerminalSquareIcon,
		title: 'Help when the game crashes',
		text: 'The "Logs" tab of an instance explains crashes in plain words.',
		points: [
			'Common problems like missing Java, too little memory or mod conflicts are detected.',
			'The analysis runs on your computer. Your logs are not uploaded.',
		],
	},
	{
		icon: PaintbrushIcon,
		title: 'Make it yours',
		text: 'Settings -> Appearance has custom themes.',
		points: [
			'Change colors, corner roundness and font, or start from a preset.',
			'Share a theme as text with friends.',
			'The skin selector has a built-in skin creator with a 3D preview.',
			'Discord status can be switched off under Privacy.',
		],
	},
]

const index = ref(0)
const open = ref(false)

const step = computed(() => steps[index.value])
const isLast = computed(() => index.value === steps.length - 1)

onMounted(() => {
	open.value = !tourDone.value
})

function close() {
	open.value = false
	index.value = 0
	tourDone.value = true
}

function next() {
	if (isLast.value) {
		close()
	} else {
		index.value += 1
	}
}

function back() {
	if (index.value > 0) index.value -= 1
}

watch(tourRequested, (value) => {
	if (!value) return
	resetTour()
	index.value = 0
	open.value = true
	tourRequested.value = false
})
</script>

<template>
	<Teleport to="body">
		<div
			v-if="open"
			class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/60 p-4"
			role="dialog"
			aria-modal="true"
			aria-label="Plainblock tour"
			@keydown.esc="close"
		>
			<div
				class="relative flex w-full max-w-lg flex-col gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-6 shadow-2xl"
			>
				<button
					type="button"
					class="absolute right-3 top-3 flex cursor-pointer items-center gap-1 rounded-lg border-none bg-transparent px-2 py-1 text-secondary hover:text-contrast"
					@click="close"
				>
					Skip
					<XIcon class="size-4" />
				</button>

				<div class="flex size-12 items-center justify-center rounded-xl bg-surface-4 text-brand">
					<component :is="step.icon" class="size-6" />
				</div>

				<div class="flex flex-col gap-1">
					<h2 class="m-0 text-2xl font-semibold text-contrast">{{ step.title }}</h2>
					<p class="m-0 text-secondary">{{ step.text }}</p>
				</div>

				<ul class="m-0 flex flex-col gap-2 pl-5">
					<li v-for="point in step.points" :key="point" class="text-primary">{{ point }}</li>
				</ul>

				<div class="flex items-center justify-between gap-3">
					<div class="flex items-center gap-1.5" aria-hidden="true">
						<span
							v-for="(_, dot) in steps"
							:key="dot"
							class="size-2 rounded-full"
							:class="dot === index ? 'bg-brand' : 'bg-surface-5'"
						/>
					</div>
					<div class="flex items-center gap-2">
						<Button v-if="index > 0" type="outlined" @click="back">Back</Button>
						<Button v-else type="outlined" @click="close">Skip tour</Button>
						<Button type="colored" color="brand" @click="next">
							{{ isLast ? 'Finish' : 'Next' }}
						</Button>
					</div>
				</div>
			</div>
		</div>
	</Teleport>
</template>
