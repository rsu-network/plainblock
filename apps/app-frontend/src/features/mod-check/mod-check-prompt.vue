<script setup lang="ts">
import { Button } from '@modrinth/ui'
import { useRouter } from 'vue-router'

import { pendingModCheck } from '@/helpers/mod-check'

const router = useRouter()

function launchAnyway() {
	pendingModCheck.value?.resolve(true)
}

function cancel() {
	pendingModCheck.value?.resolve(false)
}

function review() {
	const pending = pendingModCheck.value
	if (!pending) return
	pending.resolve(false)
	void router.push(`/instance/${encodeURIComponent(pending.instanceId)}`)
}
</script>

<template>
	<Teleport to="body">
		<div
			v-if="pendingModCheck"
			class="fixed inset-0 z-[9998] flex items-center justify-center bg-black/60 p-4"
			role="dialog"
			aria-modal="true"
			aria-label="Mod check"
			@keydown.esc="cancel"
		>
			<div
				class="flex max-h-[80vh] w-full max-w-xl flex-col gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-6 shadow-2xl"
			>
				<div>
					<h2 class="m-0 text-xl font-semibold text-contrast">
						Problems found in your mods
					</h2>
					<p class="m-0 mt-1 text-secondary">
						The game will probably crash with these. You can fix them first or launch anyway.
					</p>
				</div>

				<ul class="m-0 flex flex-col gap-2 overflow-y-auto p-0">
					<li
						v-for="(issue, index) in pendingModCheck.issues"
						:key="index"
						class="flex list-none flex-col rounded-xl border border-solid border-surface-5 bg-surface-4 px-3 py-2"
					>
						<span class="truncate font-semibold text-contrast">{{ issue.file }}</span>
						<span class="text-sm text-secondary">{{ issue.message }}</span>
					</li>
				</ul>

				<div class="flex items-center justify-end gap-2">
					<Button type="outlined" @click="launchAnyway">Launch anyway</Button>
					<Button type="colored" color="brand" @click="review">Fix mods</Button>
				</div>
			</div>
		</div>
	</Teleport>
</template>
