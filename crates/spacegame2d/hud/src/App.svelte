<script lang="ts">
  import type { LocalPlayerHudModel } from './model';
  export let player: LocalPlayerHudModel | undefined;
  const slot = player ? String(player.playerSlot).padStart(2, '0') : '--';
  const signalName = player?.color.toUpperCase() ?? 'OFFLINE';
</script>
<main class:error={!player} class="panel" style:--signal={player?.colorHex ?? '#98A4AE'}>
  <header class="panel-header">
    <span>LOCAL COMMAND</span>
    <span class="signal-status"><i></i>{signalName}</span>
  </header>
  <div class="hairline"></div>
  {#if player}
    <div class="readout-row">
      <section class="readout"><span>PLAYER</span><strong>{slot}</strong></section>
      <section class="readout"><span>COLOR</span><strong>{signalName}</strong></section>
    </div>
  {:else}
    <section class="readout error"><span>HUD</span><strong>ERROR</strong></section>
  {/if}
</main>
