<script lang="ts">
  let { tone = 'friendly', size = 10 }: { tone?: 'friendly' | 'enemy'; size?: number } = $props();
</script>

<span class={tone} style:--bracket-size={`${size}px`} aria-hidden="true">
  <i class="top-left"></i><i class="top-right"></i><i class="bottom-left"></i><i class="bottom-right"></i>
</span>

<style>
  span { position: absolute; inset: 0; pointer-events: none; color: var(--cyan-500); }
  span.enemy { color: var(--coral-500); }
  i { position: absolute; width: var(--bracket-size); height: var(--bracket-size); }
  .top-left { top: -1px; left: -1px; border-top: 1px solid currentColor; border-left: 1px solid currentColor; }
  .top-right { top: -1px; right: -1px; border-top: 1px solid currentColor; border-right: 1px solid currentColor; }
  .bottom-left { bottom: -1px; left: -1px; border-bottom: 1px solid currentColor; border-left: 1px solid currentColor; }
  .bottom-right { right: -1px; bottom: -1px; border-right: 1px solid currentColor; border-bottom: 1px solid currentColor; }
</style>
