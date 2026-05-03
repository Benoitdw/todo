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
      <h1>Paramètres</h1>
      <button class="close-btn" onclick={onClose} aria-label="Fermer">×</button>
    </div>

    <section>
      <h2>Serveur</h2>

      {#if isTauri}
        <div class="field">
          <label for="url">URL du serveur</label>
          <input
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

      <div class="field">
        <label for="token">Token d'accès</label>
        <input
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
          class="btn secondary"
          onclick={handleTest}
          disabled={testStatus === 'testing' || (isTauri ? (!url || !token) : !token)}
        >
          {testStatus === 'testing' ? 'Test en cours…' : 'Tester'}
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
        <h2>Synchronisation</h2>
        <div class="sync-row">
          <button
            class="btn secondary"
            onclick={handleSync}
            disabled={syncStatus === 'syncing'}
          >
            {syncStatus === 'syncing' ? 'Sync en cours…' : 'Forcer la sync'}
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
    background: rgba(0, 0, 0, 0.3);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .card {
    background: #fff;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 1.75rem 2rem;
    width: 100%;
    max-width: 400px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.12);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.25rem;
  }

  h1 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 600;
    color: #111;
  }

  h2 {
    margin: 0 0 0.75rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: #aaa;
    cursor: pointer;
    padding: 0 2px;
    line-height: 1;
  }

  .close-btn:hover { color: #333; }

  section {
    border-top: 1px solid #eee;
    padding-top: 1.25rem;
    margin-top: 0;
  }

  .sync-section {
    margin-top: 1.25rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-bottom: 0.85rem;
  }

  label {
    font-size: 0.8rem;
    font-weight: 500;
    color: #444;
  }

  input {
    padding: 0.5rem 0.65rem;
    border: 1px solid #d0d0d0;
    border-radius: 5px;
    font-size: 0.9rem;
    color: #111;
    background: #fff;
    outline: none;
    transition: border-color 0.15s;
  }

  input:focus { border-color: #555; }

  .status {
    font-size: 0.82rem;
    margin: 0.25rem 0 0.75rem;
    padding: 0.4rem 0.65rem;
    border-radius: 4px;
  }

  .status.ok  { background: #f0faf0; color: #2a7a2a; border: 1px solid #b8e0b8; }
  .status.error { background: #fff5f5; color: #a00; border: 1px solid #f0c0c0; }

  .actions {
    display: flex;
    gap: 0.65rem;
    justify-content: flex-end;
  }

  .sync-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .sync-status {
    font-size: 0.82rem;
  }

  .sync-status.ok    { color: #2a7a2a; }
  .sync-status.error { color: #a00; }

  .btn {
    padding: 0.5rem 1rem;
    border-radius: 5px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: background 0.15s, opacity 0.15s;
  }

  .btn:disabled { opacity: 0.45; cursor: not-allowed; }

  .btn.secondary { background: #eee; color: #333; }
  .btn.secondary:hover:not(:disabled) { background: #e0e0e0; }

  .btn.primary { background: #222; color: #fff; }
  .btn.primary:hover:not(:disabled) { background: #111; }
</style>
