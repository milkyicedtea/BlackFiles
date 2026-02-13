import adapter from '@sveltejs/adapter-static'
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'
import type {Config} from "@sveltejs/kit"

const config: Config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'null',
			precompress: false,
			strict: true
		}),
		paths: {
			assets: '.src/core/assets'
		}
	}
}

export default config
