<script lang="ts">
  import LinkPicker from './LinkPicker.svelte';
  import RefText from './RefText.svelte';
  import { refStore, makeToken } from '../lib/refs.svelte';
  import type { Island, Ref } from '../lib/types';

  let {
    island,
    dragOver = false,
    onEdit,
    onDelete,
    onDragStart,
    onDragOver,
    onDragLeave,
    onDrop,
    onDragEnd,
  }: {
    island: Island;
    dragOver?: boolean;
    onEdit: (id: string, text: string) => Promise<void>;
    onDelete: (id: string) => Promise<void>;
    onDragStart: (e: DragEvent, id: string) => void;
    onDragOver: (e: DragEvent, id: string) => void;
    onDragLeave: () => void;
    onDrop: (e: DragEvent, id: string) => void;
    onDragEnd: () => void;
  } = $props();

  let editing = $state(false);
  let draft = $state('');
  let fieldEl = $state<HTMLTextAreaElement | null>(null);

  // Link palette: open from the position of the "[[" that triggered it, so the
  // query is whatever has been typed since.
  let picker = $state<{ start: number } | null>(null);
  let pickerRef = $state<LinkPicker | null>(null);
  const pickerQuery = $derived(picker ? draft.slice(picker.start, caret) : '');
  let caret = $state(0);

  function startEdit() {
    draft = island.text;
    editing = true;
    refStore.load();
  }

  async function commit() {
    if (!editing) return;
    picker = null;
    const next = draft.trim();
    editing = false;
    if (next !== island.text) await onEdit(island.id, next);
  }

  function cancel() {
    picker = null;
    editing = false;
  }

  function syncCaret() {
    if (!fieldEl) return;
    caret = fieldEl.selectionStart ?? 0;
    // Typing past the token's line, or moving the caret before it, closes the
    // palette rather than leaving it matching a stale range.
    if (picker && (caret < picker.start || draft.slice(picker.start, caret).includes('\n'))) {
      picker = null;
    }
  }

  function onInput() {
    syncCaret();
    if (!picker && draft.slice(0, caret).endsWith('[[')) {
      picker = { start: caret };
    }
  }

  function insertRef(ref: Ref) {
    if (!picker) return;
    const before = draft.slice(0, picker.start - 2);   // drop the "[["
    const after = draft.slice(caret);
    const token = makeToken(ref.kind, ref.id);
    draft = before + token + after;
    picker = null;
    const pos = (before + token).length;
    requestAnimationFrame(() => {
      fieldEl?.focus();
      fieldEl?.setSelectionRange(pos, pos);
      caret = pos;
    });
  }

  function onKeydown(e: KeyboardEvent) {
    if (picker && pickerRef?.handleKey(e)) {
      e.preventDefault();
      return;
    }
    if (e.key === 'Escape') { e.preventDefault(); cancel(); }
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); commit(); }
  }

  $effect(() => {
    if (editing && fieldEl) {
      fieldEl.focus();
      fieldEl.setSelectionRange(fieldEl.value.length, fieldEl.value.length);
    }
  });
</script>

<!-- The DOM id is the anchor a todo→island link jumps to (#ilot-<id>). -->
<li
  id="ilot-{island.id}"
  class="band island"
  class:drag-over={dragOver}
  draggable={!editing}
  ondragstart={(e) => onDragStart(e, island.id)}
  ondragover={(e) => onDragOver(e, island.id)}
  ondragleave={onDragLeave}
  ondrop={(e) => onDrop(e, island.id)}
  ondragend={onDragEnd}
>
  <div class="content">
    {#if editing}
      <!-- A text island is a multi-line body, so Enter inserts a newline:
           blur commits, Ctrl/Cmd+Enter commits, Escape cancels. The raw
           [[…]] tokens stay visible here — what you edit is what is stored. -->
      <div class="editor-wrap">
        <textarea
          class="field editor"
          bind:this={fieldEl}
          bind:value={draft}
          rows="3"
          oninput={onInput}
          onclick={syncCaret}
          onkeyup={syncCaret}
          onblur={commit}
          onkeydown={onKeydown}
        ></textarea>
        {#if picker}
          <LinkPicker
            bind:this={pickerRef}
            query={pickerQuery}
            onPick={insertRef}
            onCancel={() => picker = null}
          />
        {/if}
      </div>
    {:else}
      <div
        class="body text"
        role="button"
        tabindex="0"
        ondblclick={startEdit}
        onkeydown={(e) => e.key === 'Enter' && startEdit()}
      >{#if island.text}<RefText text={island.text} />{:else}<span class="placeholder">Double-clic pour écrire</span>{/if}</div>
    {/if}
  </div>

  <button
    class="del-island"
    tabindex="-1"
    onclick={() => onDelete(island.id)}
    aria-label="Supprimer l'îlot"
  >
    <svg viewBox="0 0 14 14" fill="none" width="14" height="14">
      <path d="M2 3.5h10M5.5 3.5V2.5a1 1 0 0 1 1-1h1a1 1 0 0 1 1 1v1M6 6v4M8 6v4M3 3.5l.7 7.3a1 1 0 0 0 1 .9h4.6a1 1 0 0 0 1-.9L11 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  </button>
</li>

<style>
  .island {
    position: relative;
    padding: var(--bl) 0;
    border-bottom: 1px solid var(--rule);
    cursor: grab;
  }

  .island.drag-over { box-shadow: inset 0 -2px 0 var(--accent); }
  .island:hover .del-island { opacity: 1; }

  /* The content runs the full width up to the delete column: the kind of an
     island is already obvious from what it renders. */
  .content {
    grid-column: 1 / -2;
    min-width: 0;
  }

  .text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    cursor: text;
  }

  .placeholder { color: var(--ink-faint); }

  /* anchors the palette to the field it was opened from */
  .editor-wrap { position: relative; }

  .editor {
    width: 100%;
    height: auto;
    padding: 0 0 var(--bl);
    border: none;
    border-bottom: 1px solid var(--accent);
    background: none;
    outline: none;
    resize: vertical;
    font-family: var(--sans);
    font-size: 16px;
    line-height: var(--lh);
  }

  .del-island {
    grid-column: -2 / -1;
    justify-self: end;
    align-self: start;
    color: var(--ink-faint);
    opacity: 0;
    transition: opacity 0.14s ease, color 0.14s ease;
  }

  .del-island:hover { color: var(--accent); }

  @media (hover: none) {
    .del-island { opacity: 0.5; }
  }
</style>
