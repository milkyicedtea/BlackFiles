import tailwindcss from '@tailwindcss/vite';
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), svelte()],

	server: {
		proxy: {
			'/api': 'http://localhost:8000',
			'/files': 'http://localhost:8000',
		},

	}
});
