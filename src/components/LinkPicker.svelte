<script lang="ts">
  import { refStore, KIND_LABELS } from '../lib/refs.svelte';
  import type { Ref } from '../lib/types';

  let { query, placement = 'below', onPick, onCancel }: {
    query: string;
    /** 'above' for the composer, which is pinned to the bottom of the shell. */
    placement?: 'above' | 'below';
    onPick: (ref: Ref) => void;
    onCancel: () => void;
  } = $props();

  const results = $derived(refStore.search(query));
  let active = $state(0);

  // A new query means a new result set: never leave the cursor past its end.
  $effect(() => {
    query;
    active = 0;
  });

  /** Called by the host input's keydown, so the caret never leaves the field. */
  export function handleKey(e: KeyboardEvent): boolean {
    if (e.key === 'ArrowDown') {
      active = results.length ? (active + 1) % results.length : 0;
      return true;
    }
    if (e.key === 'ArrowUp') {
      active = results.length ? (active - 1 + results.length) % results.length : 0;
      return true;
    }
    if (e.key === 'Enter' || e.key === 'Tab') {
      if (results[active]) { onPick(results[active]); return true; }
      return false;
    }
    if (e.key === 'Escape') { onCancel(); return true; }
    return false;
  }
</script>

<div class="picker" class:above={placement === 'above'} role="listbox" aria-label="Choisir une cible">
  {#if results.length === 0}
    <p class="kicker none">Aucune cible</p>
  {:else}
    {#each results as ref, i (ref.kind + ref.id)}
      <button
        class="option"
        class:active={i === active}
        role="option"
        aria-selected={i === active}
        onmouseenter={() => active = i}
        onmousedown={(e) => { e.preventDefault(); onPick(ref); }}
      >
        <span class="kicker badge">{KIND_LABELS[ref.kind]}</span>
        <span class="label">
          {#if ref.parent_label}<span class="parent">{ref.parent_label} ›</span>{/if}
          {ref.label}
        </span>
      </button>
    {/each}
  {/if}
</div>

<style>
  .picker {
    position: absolute;
    z-index: 90;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    max-height: 240px;
    overflow-y: auto;
    background: var(--paper);
    border: 1px solid var(--ink);
    box-shadow: 0 2px 0 var(--ink);
    scrollbar-width: none;
  }

  .picker::-webkit-scrollbar { width: 0; }

  .picker.above {
    top: auto;
    bottom: calc(100% + 4px);
    box-shadow: 0 -2px 0 var(--ink);
  }

  .none { padding: var(--bl) 10px; color: var(--ink-faint); }

  .option {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    padding: 0 10px;
    height: 32px;                    /* 4 baselines */
    text-align: left;
    font-size: 14px;
    color: var(--ink-mid);
  }

  .option.active { background: var(--wash); color: var(--ink); }

  .badge {
    flex-shrink: 0;
    width: 44px;
    color: var(--accent);
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .parent { color: var(--ink-faint); }
</style>
