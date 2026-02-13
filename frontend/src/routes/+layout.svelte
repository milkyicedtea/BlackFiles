<script lang="ts">
  import type {Snippet} from "svelte"
  import "./layout.css"

  let { children }: { children: Snippet } = $props()
</script>

<svelte:head>
  <link rel="icon" type="image/svg" href="../core/assets/favicon.io"/>
</svelte:head>

{@render children()}