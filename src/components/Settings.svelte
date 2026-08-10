<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  let url = $state('');
  let token = $state('');
  let testStatus = $state<'idle' | 'testing' | 'ok' | 'error'>('idle');
  let testError = $state('');
  let saving = $state(false);
  let syncStatus = $state<'idle' | 'syncing' | 'ok' | 'error'>('idle');
  let syncError = $state('');

  onMount(async () => {
    const cfg = await api.getConfig();
    if (cfg) {
      url = cfg.server_url;
      token = cfg.token;
    }
  });

  function onFieldInput() {
    if (testStatus !== 'idle') testStatus = 'idle';
  }

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

  async function handleSave() {
    saving = true;
    try {
      await api.saveConfig(isTauri ? url : '', token);
      onClose();
    } catch {
      testStatus = 'error';
      testError = 'Erreur lors de la sauvegarde';
    } finally {
      saving = false;
    }
  }

  async function handleSync() {
    syncStatus = 'syncing';
    syncError = '';
    try {
      await api.triggerSync();
      syncStatus = 'ok';
    } catch (e) {
      syncStatus = 'error';
      syncError = typeof e === 'string' ? e : (e as Error)?.message ?? 'Erreur inconnue';
    }
  }
</script>

<div class="overlay" role="dialog" aria-modal="true">
  <div class="card">
    <div class="card-header">
      <p class="kicker accent">Réglages</p>
      <button class="close-btn" onclick={onClose} aria-label="Fermer">×</button>
    </div>
    <div class="rule ink"></div>

    <h1 class="display">Serveur</h1>

    <section>
      {#if isTauri}
        <div class="row">
          <label class="kicker" for="url">URL du serveur</label>
          <input
            class="field"
            id="url"
            type="url"
            bind:value={url}
            oninput={onFieldInput}
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
          oninput={onFieldInput}
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
          onclick={handleSave}
          disabled={testStatus !== 'ok' || saving}
        >
          {saving ? 'Enregistrement…' : 'Enregistrer'}
        </button>
      </div>
    </section>

    {#if isTauri}
      <section class="sync-section">
        <div class="rule"></div>
        <p class="kicker">Synchronisation</p>
        <div class="sync-row">
          <button
            class="btn"
            onclick={handleSync}
            disabled={syncStatus === 'syncing'}
          >
            {syncStatus === 'syncing' ? 'Sync…' : 'Forcer la sync'}
          </button>
          {#if syncStatus === 'ok'}
            <span class="sync-status ok">Sync réussie</span>
          {:else if syncStatus === 'error'}
            <span class="sync-status error" title={syncError}>Échec — {syncError}</span>
          {/if}
        </div>
      </section>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(17, 19, 21, 0.32);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--lh) var(--margin);
    z-index: 100;
    overflow-y: auto;
  }

  .card {
    background: var(--paper);
    border: 1px solid var(--ink);
    padding: var(--lh) 32px calc(var(--lh) + env(safe-area-inset-bottom, 0px));
    width: 100%;
    max-width: 456px;
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: var(--lh);
  }

  .display {
    font-size: 40px;
    line-height: 48px;               /* 6 baselines */
    font-weight: 800;
    letter-spacing: -0.035em;
    margin-bottom: var(--lh);
  }

  .close-btn {
    font-size: 24px;
    line-height: var(--lh);
    color: var(--ink-mid);
  }

  .close-btn:hover { color: var(--accent); }

  .sync-section { margin-top: var(--lh); }

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
  }

  .actions .btn { flex: 1; }

  .sync-row {
    display: flex;
    align-items: center;
    gap: var(--lh);
    margin-top: var(--bl);
  }

  .sync-status {
    font-family: var(--mono);
    font-size: 11px;
    line-height: var(--lh);
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }

  .sync-status.ok    { color: var(--ink-mid); }
  .sync-status.error { color: var(--accent); }
</style>
