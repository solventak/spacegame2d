<script lang="ts">
  import { onMount } from 'svelte';
  import { send, subscribe } from './bridge';
  import {
    protocolVersion,
    type BridgeId,
    type ConnectionState,
    type EngineToUi,
    type ProtocolErrorCode,
    type RequestId,
  } from './generated/ui-engine-ipc';

  let state: ConnectionState | undefined;
  let address = '';
  let displayName = '';
  let nameTouched = false;
  let activeRequest: RequestId | undefined;
  let bridgeId: BridgeId = `bridge-${crypto.randomUUID().replace(/[^A-Za-z0-9_-]/g, '')}`;
  let protocolError: ProtocolErrorCode | undefined;

  const requestId = (): RequestId => `request-${crypto.randomUUID().replace(/[^A-Za-z0-9_-]/g, '')}`;
  const connecting = () => ['resolvingHost', 'openingSocket', 'handshaking'].includes(state?.stage ?? '');
  const hasAddress = () => address.trim().length > 0;
  const normalizedName = () => displayName.trim().normalize('NFC');
  const graphemes = (value: string) => {
    const segmenter = typeof Intl !== 'undefined' && Intl.Segmenter
      ? new Intl.Segmenter(undefined, { granularity: 'grapheme' })
      : undefined;
    return segmenter ? Array.from(segmenter.segment(value)).length : Array.from(value).length;
  };
  const nameError = () => {
    const name = normalizedName();
    if (!name) return 'NAME REQUIRED';
    if (/[\p{Cc}\p{Cf}]/u.test(name)) return 'CONTROL CHARACTERS NOT ALLOWED';
    if (graphemes(name) > 24) return 'MAXIMUM 24 CHARACTERS';
    return undefined;
  };
  const hasName = () => !nameError();
  const failed = () => state?.stage === 'failed';
  const idleReason = () => state?.stage === 'idle' ? state.reason : undefined;

  const linkLabel = () => {
    if (state?.stage === 'resolvingHost') return 'RESOLVING HOST';
    if (state?.stage === 'openingSocket') return 'OPENING SOCKET';
    if (state?.stage === 'handshaking') return 'HANDSHAKING';
    if (state?.stage === 'failed') {
      return {
        timeout: 'NO RESPONSE',
        network: 'NETWORK UNAVAILABLE',
        rejected: 'LINK REJECTED',
        serverFull: 'SERVER FULL',
        versionMismatch: 'VERSION MISMATCH',
      }[state.reason];
    }
    if (idleReason() === 'sessionLost') return 'SESSION LOST';
    if (idleReason() === 'cancelled') return 'LINK ABORTED';
    return 'STANDBY';
  };

  const headerLabel = () => {
    if (connecting()) return 'LINKING';
    if (failed() || idleReason() === 'sessionLost') return 'LINK FAILED';
    return 'OFFLINE';
  };

  const hint = () => {
    if (connecting()) return 'ABORT TO CANCEL HANDSHAKE';
    if (state?.stage === 'failed') return 'CHECK ADDRESS AND TRY AGAIN';
    if (idleReason() === 'sessionLost') return 'SESSION LOST — RECONNECT AVAILABLE';
    if (idleReason() === 'cancelled') return 'ATTEMPT ABORTED';
    return 'ENTER ADDRESS TO CONNECT';
  };

  const progressStage = () => state?.stage ?? 'idle';

  onMount(() => {
    const stop = subscribe((message: EngineToUi) => {
      if (message.bridgeId !== bridgeId) return;
      if (message.kind === 'heartbeat') {
        send({ kind: 'heartbeatAcknowledged', protocolVersion, bridgeId, sequence: message.sequence });
        return;
      }
      if (message.kind === 'protocolError') {
        protocolError = message.code;
        return;
      }
      const next = message.state;
      if (next.stage !== 'idle' && activeRequest && next.requestId !== activeRequest) return;
      state = next;
      address = next.address;
      if (next.stage !== 'idle') displayName = next.displayName;
      if (next.stage === 'idle' || next.stage === 'failed' || next.stage === 'connected') {
        activeRequest = undefined;
      }
    });
    send({ kind: 'uiReady', protocolVersion, bridgeId });
    return stop;
  });

  function connect() {
    nameTouched = true;
    if (!hasAddress() || !hasName() || connecting()) return;
    activeRequest = requestId();
    send({ kind: 'connectRequested', protocolVersion, bridgeId, requestId: activeRequest, address, displayName: normalizedName() });
  }

  function cancel() {
    if (activeRequest) {
      send({ kind: 'connectionCancelled', protocolVersion, bridgeId, requestId: activeRequest });
    }
  }

  function retry() {
    window.location.reload();
  }
</script>

{#if protocolError}
  <main class="connection-shell" aria-live="assertive">
    <section class="connection-panel bridge-error">
      <header class="panel-header"><span>UI ENGINE LINK</span><span class="state-dot enemy"></span></header>
      <div class="panel-hairline"></div>
      <h1>UI BRIDGE ERROR</h1>
      <p class="instrument-copy">IPC {protocolError}</p>
      <button class="command-button commit" type="button" on:click={retry}>RETRY</button>
    </section>
  </main>
{:else if state?.stage === 'connected'}
  <main class="panel" style:--signal={state.localPlayer.colorHex}>
    <header class="panel-header"><span>LOCAL COMMAND</span><span class="signal-status"><i></i>{state.localPlayer.color.toUpperCase()}</span></header>
    <div class="panel-hairline"></div>
    <div class="readout-row">
      <section class="readout"><span>CALLSIGN</span><strong>{state.displayName}</strong></section>
      <section class="readout"><span>COLOR</span><strong>{state.localPlayer.color.toUpperCase()}</strong></section>
    </div>
  </main>
{:else}
  <main class="connection-shell">
    <div class="field-grid" aria-hidden="true"></div>
    <div class="field-lift" aria-hidden="true"></div>

    <header class="connection-chrome top-chrome">
      <span>BUILD N/A</span>
      <span class:active={connecting()} class:failed={failed() || idleReason() === 'sessionLost'} class="chrome-link">
        <i></i>{headerLabel()}
      </span>
    </header>

    <section class="connection-content" aria-label="Server connection">
      <div class="product-lockup">
        <h1>RELAY OPERATIONS</h1>
        <div><i></i><span>NO SERVER LINKED</span><i></i></div>
      </div>

      <form class="connection-panel" on:submit|preventDefault={connect}>
        <header class="panel-header"><span>CONNECT TO SERVER</span><span>DIRECT LINK</span></header>

        <label class="address-field">
          <span>DISPLAY NAME</span>
          <span class:active={hasName()} class:error={nameTouched && !hasName()} class="address-input">
            <b>ID</b>
            <i></i>
            <input aria-label="Display name" aria-invalid={nameTouched && !hasName()} bind:value={displayName} on:input={() => nameTouched = true} disabled={connecting()} placeholder="Callsign" spellcheck="false" autocomplete="off" />
            <em class:error-text={graphemes(normalizedName()) > 24}>{graphemes(normalizedName())}/24</em>
          </span>
          {#if nameTouched && !hasName()}<small class="field-error">{nameError()}</small>{/if}
        </label>

        <label class="address-field">
          <span>SERVER ADDRESS</span>
          <span class:active={hasAddress()} class:error={failed()} class="address-input">
            <b>HOST:PORT</b>
            <i></i>
            <input aria-label="Server address" aria-invalid={failed()} bind:value={address} disabled={connecting()} placeholder="server.example:4000" spellcheck="false" autocomplete="off" />
            {#if hasAddress()}<em>READY</em>{/if}
          </span>
        </label>

        <div class:active={connecting()} class="connection-progress" data-stage={progressStage()}><i></i></div>

        <div class="link-row">
          <section class="link-status" aria-live="polite">
            <span>LINK STATUS</span>
            <strong class:active={connecting()} class:error={failed() || idleReason() === 'sessionLost'}>{linkLabel()}</strong>
          </section>
          {#if connecting()}
            <button class="command-button ghost" type="button" on:click={cancel}>ABORT</button>
          {:else}
            <button class="command-button commit" type="submit" disabled={!hasAddress() || !hasName()}>CONNECT</button>
          {/if}
        </div>

        <div class="panel-hairline"></div>
        <div class="connection-details"><span>{hint()}</span><span>RTT N/A</span></div>
      </form>

      {#if failed()}
        <p class="failure-note">{linkLabel()} — RETRY AVAILABLE</p>
      {/if}
    </section>

    <footer class="connection-chrome bottom-chrome">
      <span>CLIENT N/A · UI IPC {protocolVersion}</span>
      <span>DIRECT CONNECTION</span>
    </footer>
  </main>
{/if}
