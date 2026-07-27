<script lang="ts">
  import { onMount } from 'svelte';
  import { send, subscribeState } from './bridge';
  import type { UiState } from './model';
  export let initialState: UiState | undefined;
  let state = initialState;
  let address = state?.address ?? '';
  let previousState = state;
  $: if (state !== previousState) { address = state?.address ?? ''; previousState = state; }
  onMount(() => subscribeState((next) => { state = next; address = next.address; }));
  const message = (value: UiState | undefined) => {
    if (!value || value.kind === 'disconnected') return value?.reason === 'sessionLost' ? 'Disconnected from the server.' : 'Enter a server address to join.';
    if (value.kind === 'connectionFailed') return 'Connection failed. Check the address and try again.';
    if (value.kind === 'rejected') return 'The server rejected this connection.';
    if (value.kind === 'serverFull') return 'The server is full. Try again later.';
    if (value.kind === 'versionMismatch') return 'Client and server versions do not match. Update the client.';
    return '';
  };
  function connect() { send({ schemaVersion: 1, kind: 'connect', address }); }
</script>

{#if state?.kind === 'connected'}
  <main class="panel" style:--signal={state.localPlayer.colorHex}>
    <header class="panel-header"><span>LOCAL COMMAND</span><span class="signal-status"><i></i>{state.localPlayer.color.toUpperCase()}</span></header>
    <div class="hairline"></div>
    <div class="readout-row"><section class="readout"><span>PLAYER</span><strong>{String(state.localPlayer.playerSlot).padStart(2, '0')}</strong></section><section class="readout"><span>COLOR</span><strong>{state.localPlayer.color.toUpperCase()}</strong></section></div>
  </main>
{:else}
  <main class="connection-shell">
    <form class="connection-form" on:submit|preventDefault={connect}>
      <p class="eyebrow">SPACEGAME 2D</p><h1>CONNECT</h1><p class="message">{state?.kind === 'connecting' ? 'Connecting…' : message(state)}</p>
      <label>SERVER ADDRESS<input aria-label="Server address" bind:value={address} disabled={state?.kind === 'connecting'} /></label>
      {#if state?.kind === 'connecting'}
        <button type="button" on:click={() => send({ schemaVersion: 1, kind: 'cancel' })}>Cancel</button>
      {:else}
        <button type="submit">Connect</button>
      {/if}
    </form>
  </main>
{/if}
