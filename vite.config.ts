import * as path from "path"
import { defineConfig } from 'vite'
import react from "@vitejs/plugin-react"
import tanstackRouter from "@tanstack/router-plugin/vite"

export default defineConfig({
  plugins: [
    tanstackRouter({
      target: "react",
      autoCodeSplitting: true,
      routesDirectory: path.resolve('./src/client/routes'),
      generatedRouteTree: path.resolve('./src/client/routeTree.gen.ts'),
    }),
    react(),
  ],

  resolve: {
    tsconfigPaths: true
  },

  server: {
    host: '0.0.0.0',
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:4000',
        changeOrigin: true,
        ws: true,
      },
    },
  }
})
