import { ref, watch } from 'vue'

const TOUR_DONE_KEY = 'plainblock-tour-done'

function readDone(): boolean {
	try {
		return localStorage.getItem(TOUR_DONE_KEY) === '1'
	} catch {
		return false
	}
}

export const tourDone = ref(readDone())
export const tourRequested = ref(false)

watch(tourDone, (done) => {
	try {
		if (done) {
			localStorage.setItem(TOUR_DONE_KEY, '1')
		} else {
			localStorage.removeItem(TOUR_DONE_KEY)
		}
	} catch {
		// storage blocked
	}
})

export function resetTour() {
	tourDone.value = false
}

export function startTour() {
	tourRequested.value = true
}
