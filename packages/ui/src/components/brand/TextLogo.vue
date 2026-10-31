<template>
	<span class="plainblock-logo" :class="{ animate }">
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1070 1195" class="plainblock-logo__mark" aria-hidden="true">
			<polygon points="535,14 1054,272 535,532 15,272" fill="#f9f9f9"/><polygon points="15,272 535,532 535,1180 15,920" fill="#cccccc"/><polygon points="1054,272 535,532 535,1180 1054,920" fill="#ebebeb"/><polygon points="535,14 1054,272 1054,920 535,1180 15,920 15,272" fill="none" stroke="#4d4d4d" stroke-width="26"/>
		</svg>
		<span class="plainblock-logo__name">Plainblock</span>
	</span>
</template>

<script setup lang="ts">
defineProps<{ animate?: boolean }>()
</script>

<style lang="scss" scoped>
.plainblock-logo {
	display: inline-flex;
	align-items: center;
	gap: 0.5rem;
	height: 1.75rem;
	font-weight: 800;
	letter-spacing: -0.02em;
	line-height: 1;
}

.plainblock-logo__mark {
	height: 100%;
	width: auto;
}

.plainblock-logo__name {
	font-size: 1.25rem;
}
</style>
