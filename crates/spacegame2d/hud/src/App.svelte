<script lang="ts">
  import type { LocalPlayerHudModel } from './model';
  export let player: LocalPlayerHudModel | undefined;
  const slot = player ? String(player.playerSlot).padStart(2, '0') : '--';
</script>
<main class:error={!player} class="panel" style:--signal={player?.colorHex ?? '#98A4AE'}>
  {#if player}<section class="readout"><span>PLAYER</span><strong>{slot}</strong></section><section class="readout"><span>COLOR</span><strong>{player.color.toUpperCase()}</strong></section>{:else}<section class="readout"><span>HUD</span><strong>ERROR</strong></section>{/if}
</main>
