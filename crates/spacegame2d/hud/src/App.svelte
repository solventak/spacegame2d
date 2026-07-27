<script lang="ts">
  import { onMount } from 'svelte';
  import { send, subscribe } from './bridge';
  import type { BridgeId, ConnectionState, EngineToUi, ProtocolErrorCode, RequestId } from './generated/ui-engine-ipc';
  let state: ConnectionState | undefined;
  let address = '';
  let activeRequest: RequestId | undefined;
  let bridgeId: BridgeId = `bridge-${crypto.randomUUID().replace(/[^A-Za-z0-9_-]/g, '')}`;
  let protocolError: ProtocolErrorCode | undefined;
  const requestId = (): RequestId => `request-${crypto.randomUUID().replace(/[^A-Za-z0-9_-]/g, '')}`;
  onMount(() => {
    const stop = subscribe((message: EngineToUi) => {
      if (message.bridgeId !== bridgeId) return;
      if (message.kind === 'heartbeat') { send({ kind: 'heartbeatAcknowledged', protocolVersion: 1, bridgeId, sequence: message.sequence }); return; }
      if (message.kind === 'protocolError') { protocolError = message.code; return; }
      const next = message.state;
      if (next.stage !== 'idle' && activeRequest && next.requestId !== activeRequest) return;
      state = next; address = next.address;
      if (next.stage === 'idle' || next.stage === 'failed' || next.stage === 'connected') activeRequest = undefined;
    });
    send({ kind: 'uiReady', protocolVersion: 1, bridgeId });
    return stop;
  });
  function connect() { activeRequest = requestId(); send({ kind: 'connectRequested', protocolVersion: 1, bridgeId, requestId: activeRequest, address }); }
  function cancel() { if (activeRequest) send({ kind: 'connectionCancelled', protocolVersion: 1, bridgeId, requestId: activeRequest }); }
  function retry() { window.location.reload(); }
  const connecting = () => ['resolvingHost', 'openingSocket', 'handshaking'].includes(state?.stage ?? '');
  const message = () => {
    if (!state || state.stage === 'idle') return state?.reason === 'sessionLost' ? 'Disconnected from the server.' : 'Enter a server address to join.';
    if (state.stage === 'failed') return state.reason === 'serverFull' ? 'The server is full. Try again later.' : state.reason === 'versionMismatch' ? 'Client and server versions do not match. Update the client.' : state.reason === 'rejected' ? 'The server rejected this connection.' : 'Connection failed. Check the address and try again.';
    return 'Connecting…';
  };
</script>

{#if protocolError}
  <main class="connection-shell"><section class="connection-form"><p class="eyebrow">SPACEGAME 2D</p><h1>CONNECTION ERROR</h1><p class="message">The UI connection failed safely. Code: {protocolError}</p><button type="button" on:click={retry}>Retry</button></section></main>
{:else if state?.stage === 'connected'}
  <main class="panel" style:--signal={state.localPlayer.colorHex}><header class="panel-header"><span>LOCAL COMMAND</span><span class="signal-status"><i></i>{state.localPlayer.color.toUpperCase()}</span></header><div class="hairline"></div><div class="readout-row"><section class="readout"><span>PLAYER</span><strong>{String(state.localPlayer.playerSlot).padStart(2, '0')}</strong></section><section class="readout"><span>COLOR</span><strong>{state.localPlayer.color.toUpperCase()}</strong></section></div></main>
{:else}
  <main class="connection-shell"><form class="connection-form" on:submit|preventDefault={connect}><p class="eyebrow">SPACEGAME 2D</p><h1>CONNECT</h1><p class="message">{message()}</p><label>SERVER ADDRESS<input aria-label="Server address" bind:value={address} disabled={connecting()} /></label>{#if connecting()}<button type="button" on:click={cancel}>Cancel</button>{:else}<button type="submit">Connect</button>{/if}</form></main>
{/if}
