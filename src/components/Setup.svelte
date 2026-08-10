<script lang="ts">
  import { api } from '../lib/api';

  interface Props {
    onComplete: () => void;
  }

  let { onComplete }: Props = $props();

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  let url = $state('http://');
  let token = $state('');
  let testStatus = $state<'idle' | 'testing' | 'ok' | 'error'>('idle');
  let testError = $state('');
  let saving = $state(false);

  async function handleTest() {
    testStatus = 'testing';
    testError = '';
    try {
      await api.testConnection(isTauri ? url : '', token);
      testStatus = 'ok';
    } catch (e) {
      testStatus = 'error';
      testError = typeof e === 'string' ? e : 'Connexion échouée';
    }
  }

  async function handleContinue() {
    saving = true;
    try {
      await api.saveConfig(isTauri ? url : '', token);
      api.triggerSync().catch(() => {
        // Ignore — might be temporarily offline
      });
      onComplete();
    } catch (e) {
      testStatus = 'error';
      testError = typeof e === 'string' ? e : 'Erreur lors de la sauvegarde';
    } finally {
      saving = false;
    }
  }

  function onUrlInput() {
    if (testStatus !== 'idle') testStatus = 'idle';
  }

  function onTokenInput() {
    if (testStatus !== 'idle') testStatus = 'idle';
  }
</script>

<div class="setup-overlay">
  <div class="card">
    <p class="kicker accent">Configuration</p>
    <div class="rule ink"></div>

    <h1 class="display">Serveur</h1>

    {#if isTauri}
      <p class="body lede">Connecte l'application à ton serveur NAS pour activer la synchronisation.</p>
    {:else}
      <p class="body lede">Entre ton token d'accès pour te connecter.</p>
    {/if}

    {#if isTauri}
      <div class="row">
        <label class="kicker" for="url">URL du serveur</label>
        <input
          class="field"
          id="url"
          type="url"
          bind:value={url}
          oninput={onUrlInput}
          placeholder="http://192.168.1.100:8080"
          autocomplete="off"
          spellcheck="false"
        />
      </div>
    {/if}

    <div class="row">
      <label class="kicker" for="token">Token d'accès</label>
      <input
        class="field"
        id="token"
        type="password"
        bind:value={token}
        oninput={onTokenInput}
        placeholder="••••••••••••"
        autocomplete="off"
      />
    </div>

    {#if testStatus === 'ok'}
      <p class="status ok">Connexion réussie</p>
    {:else if testStatus === 'error'}
      <p class="status error">{testError}</p>
    {/if}

    <div class="actions">
      <button
        class="btn"
        onclick={handleTest}
        disabled={testStatus === 'testing' || (isTauri ? (!url || !token) : !token)}
      >
        {testStatus === 'testing' ? 'Test…' : 'Tester'}
      </button>

      <button
        class="btn primary"
        onclick={handleContinue}
        disabled={testStatus !== 'ok' || saving}
      >
        {saving ? 'Enregistrement…' : 'Continuer'}
      </button>
    </div>
  </div>
</div>

<style>
  .setup-overlay {
    position: fixed;
    inset: 0;
    background: var(--paper);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--lh) var(--margin);
    z-index: 100;
    overflow-y: auto;
  }

  .card {
    width: 100%;
    max-width: 456px;
  }

  .display {
    font-size: 48px;
    line-height: 56px;               /* 7 baselines */
    font-weight: 800;
    letter-spacing: -0.035em;
    margin-bottom: var(--bl);
  }

  .lede {
    color: var(--ink-mid);
    margin-bottom: var(--lh);
    max-width: 40ch;
  }

  .row {
    display: flex;
    flex-direction: column;
    gap: var(--bl);
    margin-bottom: var(--lh);
  }

  .status {
    font-family: var(--mono);
    font-size: 11px;
    line-height: var(--lh);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    padding-left: 10px;
    margin-bottom: var(--lh);
  }

  .status.ok    { color: var(--ink); border-left: 2px solid var(--ink); }
  .status.error { color: var(--accent); border-left: 2px solid var(--accent); }

  .actions {
    display: flex;
    gap: var(--bl);
    margin-top: var(--lh);
  }

  .actions .btn { flex: 1; }

  @media (max-width: 640px) {
    .display { font-size: 34px; line-height: 40px; }
  }
</style>
