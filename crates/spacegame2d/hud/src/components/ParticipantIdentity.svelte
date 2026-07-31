<script lang="ts">
  import StateDot from './StateDot.svelte';
  import TacticalGlyph from './TacticalGlyph.svelte';

  let {
    label,
    value,
    tone,
    glyph = false,
    presence,
  }: {
    label: string;
    value: string;
    tone: 'friendly' | 'enemy';
    glyph?: boolean;
    presence?: 'present' | 'disconnected';
  } = $props();
</script>

<div class="participant" class:friendly={tone === 'friendly'} class:enemy={tone === 'enemy'}>
  <div class="copy">
    <span>{label}</span>
    <strong>{value}</strong>
  </div>
  {#if glyph}<TacticalGlyph {tone} size={24} />{/if}
  {#if presence}
    <div class:disconnected={presence === 'disconnected'} class="presence">
      <StateDot tone={presence === 'present' ? 'enemy' : 'neutral'} certainty={presence === 'present' ? 'confirmed' : 'stale'} />
      <em>{presence.toUpperCase()}</em>
    </div>
  {/if}
</div>

<style>
  .participant { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .copy { display: grid; gap: 1px; min-width: 0; }
  .copy span {
    color: var(--gray-400);
    font: 600 10px/1.1 var(--font-condensed);
    letter-spacing: .14em;
    text-transform: uppercase;
  }
  .copy strong {
    overflow: hidden;
    color: var(--gray-100);
    font: 600 15px/1.1 var(--font-condensed);
    letter-spacing: .06em;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .friendly .copy strong { color: var(--cyan-300); font-size: 18px; font-weight: 700; letter-spacing: .1em; }
  .presence {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 7px;
    border: 1px solid rgba(255, 106, 71, .42);
    border-radius: 2px;
  }
  .presence em {
    color: var(--coral-300);
    font: 600 10px/1.1 var(--font-condensed);
    font-style: normal;
    letter-spacing: .14em;
  }
  .presence.disconnected { border-color: rgba(152, 164, 174, .3); }
  .presence.disconnected em { color: var(--gray-300); }
</style>
