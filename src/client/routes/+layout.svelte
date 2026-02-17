<script lang="ts">
  import { navigating } from "$app/state"
  import { onMount, type Snippet } from "svelte"
  import "./layout.css"
  import { SvelteTheme } from "svelte-themes"
  import AuthScreen from "@client/components/AuthScreen.svelte"
  import LoadingScreen from "@client/components/LoadingScreen.svelte"

  interface LayoutProps {
    children: Snippet
  }

  let { children }: LayoutProps = $props()

  let authenticated = $state(false)
  let checking = $state(true)

  const isNavigating = $derived(navigating.complete !== null)

  onMount(async () => {
    try {
      const res = await fetch('/api/check', {
        credentials: 'include'
      })
      authenticated = res.ok
    } finally {
      checking = false
    }
  })
</script>

<svelte:head>
  <link rel="icon" type="image/svg" href="/favicon.ico"/>
</svelte:head>

<SvelteTheme
  themes={['light', 'dark']}
  defaultTheme="dark"
  attribute="data-theme"
  storageKey="theme"
>
  {#if checking || isNavigating}
    <LoadingScreen/>
  {:else if !authenticated}
    <AuthScreen onAuth={() => authenticated = true}/>
  {:else}
    {@render children()}
  {/if}
</SvelteTheme>