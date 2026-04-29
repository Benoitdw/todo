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
  <div class="setup-card">
    <h1>Configuration du serveur</h1>
    {#if isTauri}
      <p class="subtitle">Connecte l'application à ton serveur NAS pour activer la synchronisation.</p>
    {:else}
      <p class="subtitle">Entre ton token d'accès pour te connecter.</p>
    {/if}

    {#if isTauri}
      <div class="field">
        <label for="url">URL du serveur</label>
        <input
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

    <div class="field">
      <label for="token">Token d'accès</label>
      <input
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
        class="btn secondary"
        onclick={handleTest}
        disabled={testStatus === 'testing' || (isTauri ? (!url || !token) : !token)}
      >
        {testStatus === 'testing' ? 'Test en cours…' : 'Tester la connexion'}
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
    background: #f5f5f5;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .setup-card {
    background: #fff;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 2rem 2.5rem;
    width: 100%;
    max-width: 420px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
  }

  h1 {
    margin: 0 0 0.4rem;
    font-size: 1.25rem;
    font-weight: 600;
    color: #111;
  }

  .subtitle {
    margin: 0 0 1.5rem;
    font-size: 0.875rem;
    color: #666;
    line-height: 1.5;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-bottom: 1rem;
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

  input:focus {
    border-color: #555;
  }

  .status {
    font-size: 0.82rem;
    margin: 0.5rem 0 0.75rem;
    padding: 0.4rem 0.65rem;
    border-radius: 4px;
  }

  .status.ok {
    background: #f0faf0;
    color: #2a7a2a;
    border: 1px solid #b8e0b8;
  }

  .status.error {
    background: #fff5f5;
    color: #a00;
    border: 1px solid #f0c0c0;
  }

  .actions {
    display: flex;
    gap: 0.65rem;
    margin-top: 1.25rem;
    justify-content: flex-end;
  }

  .btn {
    padding: 0.5rem 1.1rem;
    border-radius: 5px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: background 0.15s, opacity 0.15s;
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .btn.secondary {
    background: #eee;
    color: #333;
  }

  .btn.secondary:hover:not(:disabled) {
    background: #e0e0e0;
  }

  .btn.primary {
    background: #222;
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    background: #111;
  }
</style>
