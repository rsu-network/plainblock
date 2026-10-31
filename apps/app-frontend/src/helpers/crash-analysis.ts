export interface CrashProblem {
	message: string
	solutions: string[]
}

const MAX_PROBLEMS = 8

const JAVA_BY_CLASS_VERSION = (major: number) => major - 44

function matches(text: string, pattern: RegExp): RegExpMatchArray[] {
	const flags = pattern.flags.includes('g') ? pattern.flags : `${pattern.flags}g`
	return [...text.matchAll(new RegExp(pattern.source, flags))]
}

function addProblem(problems: Map<string, CrashProblem>, problem: CrashProblem) {
	if (!problems.has(problem.message) && problems.size < MAX_PROBLEMS) {
		problems.set(problem.message, problem)
	}
}

/**
 * Looks for well-known crash causes in a Minecraft log or crash report.
 * Runs completely offline; nothing is uploaded.
 */
export function analyseCrash(text: string): CrashProblem[] {
	const problems = new Map<string, CrashProblem>()
	if (!text.trim()) return []

	if (/java\.lang\.OutOfMemoryError/.test(text)) {
		const metaspace = /OutOfMemoryError:\s*Metaspace/i.test(text)
		addProblem(problems, {
			message: metaspace
				? 'Minecraft ran out of class memory (Metaspace)'
				: 'Minecraft ran out of memory',
			solutions: [
				'Open the instance settings and raise the maximum memory (4 to 6 GB is a good start for modded play).',
				'Close other memory-hungry programs while playing.',
				'Remove heavy mods or lower render distance and shader quality.',
			],
		})
	}

	for (const match of matches(text, /UnsupportedClassVersionError[^\n]*class file version (\d+)/)) {
		const java = JAVA_BY_CLASS_VERSION(Number(match[1]))
		addProblem(problems, {
			message: `A mod or Minecraft needs Java ${java} or newer`,
			solutions: [
				`In the instance settings, choose Java ${java} (or let Plainblock install it automatically).`,
				'If you recently changed the Minecraft version, check which Java it requires.',
			],
		})
	}

	for (const match of matches(
		text,
		/Mod '([^']+)' \(([^)]+)\)[^\n]*?requires ([^\n]*?)(?:, which is missing|, but |!|\n)/,
	)) {
		const [, name, id, requirement] = match
		addProblem(problems, {
			message: `Mod "${name}" (${id}) has a missing or incompatible dependency`,
			solutions: [
				`It requires ${requirement.trim()}.`,
				`Install the required mod or version, or update/remove "${name}".`,
			],
		})
	}

	for (const match of matches(
		text,
		/Mod ID: '([^']+)', Requested by: '([^']+)', Expected range: '([^']*)', Actual version: '([^']*)'/,
	)) {
		const [, id, requestedBy, expected, actual] = match
		addProblem(problems, {
			message: `"${requestedBy}" needs ${id} ${expected}, but ${actual === '[MISSING]' ? 'it is missing' : `version ${actual} is installed`}`,
			solutions: [
				`Install or update ${id} to a version matching ${expected}.`,
				`If that is not possible, remove "${requestedBy}".`,
			],
		})
	}

	const suspected = text.match(/Suspected Mods?:\s*([^\n]+)/i)
	if (suspected && !/^none\b/i.test(suspected[1].trim())) {
		addProblem(problems, {
			message: `The crash report points at: ${suspected[1].trim()}`,
			solutions: [
				'Update these mods to the newest version for your Minecraft version.',
				'If it still crashes, disable them one by one to find the cause.',
			],
		})
	}

	if (/Mixin apply failed|MixinApplyError|InvalidInjectionException|Mixin transformation of .* failed/i.test(text)) {
		const modMatch =
			text.match(/Mixin config ([\w.\-]+?)\.mixins\.json/i) ??
			text.match(/from mod ([\w.\-]+)/i) ??
			text.match(/\[([\w.\-]+)\.mixins\.json/i)
		addProblem(problems, {
			message: modMatch
				? `A mixin from "${modMatch[1]}" failed to apply (mod conflict)`
				: 'A mod failed to apply a mixin (mod conflict)',
			solutions: [
				'Update the named mod and the mods that change the same part of the game.',
				'Make sure all mods are built for this exact Minecraft version and loader.',
				'Remove one of the conflicting mods.',
			],
		})
	}

	for (const match of matches(
		text,
		/(?:NoClassDefFoundError|ClassNotFoundException):\s*([\w./$]+)/,
	)) {
		const className = match[1].replaceAll('/', '.')
		addProblem(problems, {
			message: `A required class is missing: ${className}`,
			solutions: [
				'A mod is missing a library or another mod it depends on.',
				'Install the required dependency, or update the mod that uses it.',
				'Check that the mod matches your Minecraft version and loader.',
			],
		})
		if (problems.size >= MAX_PROBLEMS) break
	}

	if (/Duplicate mods? found|Duplicate mod ID|Found duplicate mod/i.test(text)) {
		addProblem(problems, {
			message: 'The same mod is installed more than once',
			solutions: [
				'Open the Mods tab and remove the duplicate file (often an old version next to a new one).',
			],
		})
	}

	if (/GLFW error|WGL: The driver does not appear to support OpenGL|Pixel format not accelerated|OpenGL.*not supported/i.test(text)) {
		addProblem(problems, {
			message: 'Your graphics driver does not support what Minecraft needs',
			solutions: [
				'Update your graphics driver from the GPU manufacturer (not through Windows Update).',
				'Make sure Minecraft uses your dedicated graphics card and not the integrated one.',
				'Turn off shaders and try again.',
			],
		})
	}

	if (/Failed to start the minecraft server|Address already in use|Port \d+ already in use/i.test(text)) {
		addProblem(problems, {
			message: 'A required network port is already in use',
			solutions: [
				'Close other Minecraft instances or servers and try again.',
			],
		})
	}

	return [...problems.values()]
}

/** Builds the response shape the console page already knows how to display. */
export function toInsights(problems: CrashProblem[]) {
	return {
		id: 'local',
		name: 'Plainblock crash help',
		type: 'local',
		version: '1',
		title: 'Crash help',
		analysis: {
			problems: problems.map((problem) => ({
				message: problem.message,
				counter: 1,
				entry: { level: 0, time: null, prefix: '', lines: [] },
				solutions: problem.solutions.map((message) => ({ message })),
			})),
			information: [],
		},
	}
}
