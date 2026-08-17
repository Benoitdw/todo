<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let token = $state('');
  let testStatus = $state<'idle' | 'testing' | 'ok' | 'error'>('idle');
  let testError = $state('');
  let saving = $state(false);

  onMount(async () => {
    const cfg = await api.getConfig();
    if (cfg) {
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
      await api.testConnection(token);
      testStatus = 'ok';
    } catch (e) {
      testStatus = 'error';
      testError = typeof e === 'string' ? e : 'Connexion échouée';
    }
  }

  async function handleSave() {
    saving = true;
    try {
      await api.saveConfig(token);
      onClose();
    } catch {
      testStatus = 'error';
      testError = 'Erreur lors de la sauvegarde';
    } finally {
      saving = false;
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
          disabled={testStatus === 'testing' || !token}
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
</style>
