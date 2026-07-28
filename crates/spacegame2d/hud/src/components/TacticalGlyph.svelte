<script lang="ts">
  let {
    tone,
    framed = false,
    size = 24,
  }: {
    tone: 'friendly' | 'enemy';
    framed?: boolean;
    size?: number;
  } = $props();
</script>

<svg
  class:framed
  class:friendly={tone === 'friendly'}
  class:enemy={tone === 'enemy'}
  width={size}
  height={size}
  viewBox="0 0 30 30"
  aria-hidden="true"
>
  {#if framed}<circle cx="15" cy="15" r="13" />{/if}
  <path d="M15 8.5 21.5 15 15 21.5 8.5 15Z" />
</svg>

<style>
  svg {
    display: block;
    overflow: visible;
    color: var(--gray-300);
  }
  svg.friendly { color: var(--cyan-300); }
  svg.enemy { color: var(--coral-500); }
  circle { fill: none; stroke: currentColor; stroke-width: 1.25; }
  path { fill: currentColor; stroke: currentColor; stroke-linejoin: miter; stroke-width: 1; }
</style>
