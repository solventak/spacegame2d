<script lang="ts">
  let {
    tone,
    certainty = 'confirmed',
    size = 6,
  }: {
    tone: 'friendly' | 'enemy' | 'neutral';
    certainty?: 'confirmed' | 'estimated' | 'stale';
    size?: number;
  } = $props();
</script>

<i
  class={tone}
  class:estimated={certainty === 'estimated'}
  class:stale={certainty === 'stale'}
  style:width={`${size}px`}
  style:height={`${size}px`}
  aria-hidden="true"
></i>

<style>
  i {
    display: inline-block;
    flex: none;
    border: 1px solid currentColor;
    border-radius: 999px;
    background: currentColor;
  }
  .friendly { color: var(--cyan-500); }
  .enemy { color: var(--coral-500); }
  .neutral { color: var(--gray-300); }
  .estimated { border-style: dashed; background: transparent; }
  .stale { background: transparent; opacity: .5; }
</style>
