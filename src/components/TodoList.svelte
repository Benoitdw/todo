<script lang="ts">
  import { api } from '../lib/api';
  import { optical } from '../lib/optical';
  import LinkPicker from './LinkPicker.svelte';
  import RefText from './RefText.svelte';
  import { refStore, makeToken } from '../lib/refs.svelte';
  import type { Item, List, Ref } from '../lib/types';

  let { list, isMobile = false, syncKey = 0, loaded = false, navKey = 0, onOpenSidebar }: {
    list: List;
    isMobile?: boolean;
    syncKey?: number;
    loaded?: boolean;
    navKey?: number;
    onOpenSidebar?: () => void;
  } = $props();

  let items = $state<Item[]>([]);
  let showCompleted = $state(true);
  let draggedId = $state<string | null>(null);
  let dragOverId = $state<string | null>(null);
  let editingId = $state<string | null>(null);
  let editText = $state('');
  let newItemText = $state('');
  let errorMsg = $state<string | null>(null);
  let errorTimer: ReturnType<typeof setTimeout> | null = null;

  // Sync animation state
  let syncedIds = $state(new Set<string>());

  // ③ Add item — entering IDs
  let enteringIds = $state(new Set<string>());

  // ⑤ Delete — exiting IDs (still in items array during animation)
  let exitingIds = $state(new Set<string>());

  // Non-reactive bookkeeping for diff
  let prevItems: Item[] = [];
  let prevListId = '';

  // Keep exiting items visible even when showCompleted=false
  const visible = $derived(
    showCompleted
      ? items
      : items.filter(i => !i.checked || exitingIds.has(i.id))
  );

  const remaining = $derived(items.filter(i => !i.checked).length);
  const remainingLabel = $derived(String(remaining).padStart(2, '0'));
  const doneCount = $derived(items.filter(i => i.checked).length);

  // "Nettoyer" wipes several rows at once and there is no undo, so the first
  // click only arms the button; the second one commits. It disarms itself.
  let cleanArmed = $state(false);
  let cleanTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (doneCount === 0) cleanArmed = false;
  });

  const today = new Intl.DateTimeFormat('fr-FR', {
    day: 'numeric', month: 'long', year: 'numeric'
  }).format(new Date());

  $effect(() => {
    const id = list.id;
    const _key = syncKey;
    let cancelled = false;

    api.getItems(id).then(result => {
      if (cancelled) return;

      if (id !== prevListId) {
        prevItems = [];
        prevListId = id;
      }

      const changed = new Set<string>();
      if (syncKey > 0 && prevItems.length > 0) {
        const prevMap = new Map(prevItems.map(it => [it.id, it]));
        for (const item of result) {
          const prev = prevMap.get(item.id);
          if (!prev || prev.checked !== item.checked || prev.text !== item.text) {
            changed.add(item.id);
          }
        }
      }

      prevItems = result;
      items = result;

      if (changed.size > 0) {
        syncedIds = new Set();
        queueMicrotask(() => { syncedIds = changed; });
      }
    });

    return () => { cancelled = true; };
  });

  function showError(msg: string) {
    errorMsg = msg;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => errorMsg = null, 3000);
  }

  // Link palette on the composer: "[[" opens it, exactly as in an island body.
  let picker = $state<{ start: number } | null>(null);
  let pickerRef = $state<LinkPicker | null>(null);
  let composerEl = $state<HTMLInputElement | null>(null);
  let caret = $state(0);
  const pickerQuery = $derived(picker ? newItemText.slice(picker.start, caret) : '');

  function syncCaret() {
    if (!composerEl) return;
    caret = composerEl.selectionStart ?? 0;
    if (picker && caret < picker.start) picker = null;
  }

  function onComposerInput() {
    syncCaret();
    if (!picker && newItemText.slice(0, caret).endsWith('[[')) {
      picker = { start: caret };
      refStore.load();
    }
  }

  function insertRef(ref: Ref) {
    if (!picker) return;
    const before = newItemText.slice(0, picker.start - 2);   // drop the "[["
    const after = newItemText.slice(caret);
    const token = makeToken(ref.kind, ref.id);
    newItemText = before + token + after;
    picker = null;
    const pos = (before + token).length;
    requestAnimationFrame(() => {
      composerEl?.focus();
      composerEl?.setSelectionRange(pos, pos);
      caret = pos;
    });
  }

  function onComposerKeydown(e: KeyboardEvent) {
    if (picker && pickerRef?.handleKey(e)) {
      e.preventDefault();
      return;
    }
    if (e.key === 'Enter') addItem();
  }

  async function addItem() {
    picker = null;
    const text = newItemText.trim();
    if (!text) return;
    const pos = items.length > 0 ? items[items.length - 1].pos + 1000 : 1000;
    const tempId = crypto.randomUUID();
    const tempItem: Item = { id: tempId, list_id: list.id, text, pos, checked: false };
    // ③ Add item — prepend and mark as entering
    items = [tempItem, ...items];
    enteringIds = new Set([...enteringIds, tempId]);
    setTimeout(() => {
      enteringIds = new Set([...enteringIds].filter(x => x !== tempId));
    }, 500);
    newItemText = '';
    try {
      const item = await api.createItem(list.id, text, pos);
      items = items.map(i => i.id === tempId ? item : i);
      // Keep entering animation on the real ID too
      enteringIds = new Set([...enteringIds, item.id]);
      enteringIds = new Set([...enteringIds].filter(x => x !== tempId));
      setTimeout(() => {
        enteringIds = new Set([...enteringIds].filter(x => x !== item.id));
      }, 500);
    } catch {
      items = items.filter(i => i.id !== tempId);
      enteringIds = new Set([...enteringIds].filter(x => x !== tempId));
      newItemText = text;
      showError('Erreur lors de la création');
    }
  }

  async function toggleItem(item: Item) {
    const prev = item.checked;
    items = items.map(i => i.id === item.id ? { ...i, checked: !prev } : i);
    try {
      await api.updateItem(item.id, item.text, !prev);
    } catch {
      items = items.map(i => i.id === item.id ? { ...i, checked: prev } : i);
      showError('Erreur lors de la mise à jour');
    }
  }

  async function deleteItem(id: string) {
    if (exitingIds.has(id)) return;
    const item = items.find(i => i.id === id)!;
    if (editingId === id) editingId = null;

    // ⑤ Delete — animate out, then remove
    exitingIds = new Set([...exitingIds, id]);
    try {
      await Promise.all([
        api.deleteItem(id),
        new Promise(resolve => setTimeout(resolve, 340)),
      ]);
      items = items.filter(i => i.id !== id);
    } catch {
      showError('Erreur lors de la suppression');
    } finally {
      exitingIds = new Set([...exitingIds].filter(x => x !== id));
    }
  }

  function handleClean() {
    if (doneCount === 0) return;
    if (cleanTimer) clearTimeout(cleanTimer);

    if (!cleanArmed) {
      cleanArmed = true;
      cleanTimer = setTimeout(() => { cleanArmed = false; }, 3000);
      return;
    }

    cleanArmed = false;
    clearCompleted();
  }

  async function clearCompleted() {
    const ids = items.filter(i => i.checked).map(i => i.id);
    if (ids.length === 0) return;
    if (editingId && ids.includes(editingId)) editingId = null;

    // ⑤ Delete — the whole batch animates out together, then leaves the array
    exitingIds = new Set([...exitingIds, ...ids]);
    try {
      await Promise.all([
        ...ids.map(id => api.deleteItem(id)),
        new Promise(resolve => setTimeout(resolve, 340)),
      ]);
      items = items.filter(i => !ids.includes(i.id));
    } catch {
      showError('Erreur lors du nettoyage');
    } finally {
      exitingIds = new Set([...exitingIds].filter(x => !ids.includes(x)));
    }
  }

  function startEdit(item: Item) {
    editingId = item.id;
    editText = item.text;
  }

  async function commitEdit() {
    if (!editingId) return;
    const text = editText.trim();
    if (text) {
      const item = items.find(i => i.id === editingId)!;
      await api.updateItem(editingId, text, item.checked);
      items = items.map(i => i.id === editingId ? { ...i, text } : i);
    }
    editingId = null;
  }

  function handleDragStart(e: DragEvent, id: string) {
    draggedId = id;
    e.dataTransfer!.effectAllowed = 'move';
  }

  function handleDragOver(e: DragEvent, id: string) {
    e.preventDefault();
    dragOverId = id;
  }

  async function handleDrop(e: DragEvent, targetId: string) {
    e.preventDefault();
    if (!draggedId || draggedId === targetId) {
      draggedId = null;
      dragOverId = null;
      return;
    }
    const from = items.findIndex(i => i.id === draggedId);
    const to = items.findIndex(i => i.id === targetId);
    const reordered = [...items];
    const [moved] = reordered.splice(from, 1);
    reordered.splice(to, 0, moved);
    const updated = reordered.map((item, idx) => ({ ...item, pos: (idx + 1) * 1000 }));
    items = updated;
    draggedId = null;
    dragOverId = null;
    await Promise.all(updated.map(item => api.reorderItem(item.id, item.pos)));
  }

  function handleDragEnd() {
    draggedId = null;
    dragOverId = null;
  }
</script>

<!-- ② Navigation — slides in from right on nav change; items load on first mount -->
<main class="spread" class:nav-transition={navKey > 0}>
  <div class="wrap">
    <div class="reading">
      <div class="grid">

        <!-- meta band: menu / mode -->
        <div class="band">
          <button class="kicker menu" onclick={onOpenSidebar} aria-label="Ouvrir les listes">
            ☰&nbsp;&nbsp;Listes
          </button>
          <p class="kicker section">Liste</p>
          <button
            class="kicker clean"
            class:armed={cleanArmed}
            disabled={doneCount === 0}
            title="Supprimer les items terminés"
            onclick={handleClean}
          >{cleanArmed ? 'Confirmer' : 'Nettoyer'}&nbsp;{String(doneCount).padStart(2, '0')}</button>
          <button
            class="kicker mode"
            aria-pressed={showCompleted}
            onclick={() => showCompleted = !showCompleted}
          >{showCompleted ? '☑' : '☐'}&nbsp;&nbsp;Terminés</button>
        </div>

        <div class="band"><div class="rule ink full"></div></div>

        <!-- title band: masthead + the count set large -->
        <div class="band title-band">
          <h1 class="masthead" use:optical={list.title}>{list.title}</h1>
          <div class="meta-col">
            <p class="numeral" use:optical={remainingLabel}>{remainingLabel}</p>
            <p class="kicker accent">À faire</p>
            <p class="folio">{today}</p>
          </div>
        </div>

        <div class="band"><div class="rule full"></div></div>
      </div>

      {#if errorMsg}
        <div class="grid">
          <div class="band"><p class="toast full">{errorMsg}</p></div>
        </div>
      {/if}

      <!-- ① App Load (items) + ⑥ Sync scan line -->
      <div class="items-container" class:loaded={loaded && navKey === 0}>
        {#key syncKey}
          {#if syncKey > 0}
            <div class="scan-line"></div>
          {/if}
        {/key}

        <ul class="grid items">
          {#each visible as item, i (item.id)}
            {#key syncedIds.has(item.id) ? item.id + syncKey : item.id}
              <li
                id="item-{item.id}"
                class="band item"
                class:drag-over={dragOverId === item.id}
                class:synced={syncedIds.has(item.id)}
                class:item-entering={enteringIds.has(item.id)}
                class:item-exiting={exitingIds.has(item.id)}
                style={syncedIds.has(item.id)
                  ? `animation-delay: ${80 + (i / Math.max(visible.length, 1)) * 480}ms`
                  : ''}
                draggable="true"
                ondragstart={(e) => handleDragStart(e, item.id)}
                ondragover={(e) => handleDragOver(e, item.id)}
                ondragleave={() => dragOverId = null}
                ondrop={(e) => handleDrop(e, item.id)}
                ondragend={handleDragEnd}
              >
                <!-- ④ Check/Uncheck — the cell fills its column, the mark is the ink -->
                <button
                  class="cell check"
                  class:checked={item.checked}
                  onclick={() => toggleItem(item)}
                  aria-label={item.checked ? 'Décocher' : 'Cocher'}
                >
                  <span class="box">
                    <svg class="check-svg" viewBox="0 0 10 10" fill="none">
                      <polyline points="1.5,5 4,7.5 8.5,2.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                  </span>
                </button>

                {#if editingId === item.id}
                  <input
                    class="cell item-edit"
                    type="text"
                    bind:value={editText}
                    onblur={commitEdit}
                    onkeydown={(e) => {
                      if (e.key === 'Enter') commitEdit();
                      if (e.key === 'Escape') editingId = null;
                    }}
                  />
                {:else}
                  <!-- ④ Check/Uncheck — the rule is drawn on the inner span so
                       it stops at the ink, not at the end of the column. -->
                  <span
                    class="cell item-text body"
                    role="button"
                    tabindex="0"
                    ondblclick={() => startEdit(item)}
                    onkeydown={(e) => e.key === 'Enter' && startEdit(item)}
                  ><span class="t" class:struck={item.checked}><RefText text={item.text} /></span></span>
                {/if}

                <button
                  class="cell del-item"
                  tabindex="-1"
                  onclick={() => deleteItem(item.id)}
                  aria-label="Supprimer"
                >
                  <svg viewBox="0 0 14 14" fill="none" width="14" height="14">
                    <path d="M2 3.5h10M5.5 3.5V2.5a1 1 0 0 1 1-1h1a1 1 0 0 1 1 1v1M6 6v4M8 6v4M3 3.5l.7 7.3a1 1 0 0 0 1 .9h4.6a1 1 0 0 0 1-.9L11 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </button>
              </li>
            {/key}
          {/each}

          {#if visible.length === 0}
            <li class="band empty-row">
              <p class="kicker full">Rien à faire</p>
            </li>
          {/if}
        </ul>
      </div>
    </div>

    <!-- The composer is a sibling of the scroll box, never inside it: it is
         pinned to the bottom of the dvh shell so it is reachable without
         scrolling or zooming, on every viewport. -->
    <div class="composer">
      <div class="grid">
        <div class="band">
          <div class="new-item-wrap">
            <input
              class="field new-item"
              type="text"
              placeholder="Nouvel item…"
              enterkeyhint="done"
              bind:this={composerEl}
              bind:value={newItemText}
              oninput={onComposerInput}
              onclick={syncCaret}
              onkeyup={syncCaret}
              onblur={() => picker = null}
              onkeydown={onComposerKeydown}
            />
            {#if picker}
              <LinkPicker
                bind:this={pickerRef}
                query={pickerQuery}
                placement="above"
                onPick={insertRef}
                onCancel={() => picker = null}
              />
            {/if}
          </div>
          <button class="btn accent add" onclick={addItem} aria-label="Ajouter">
            <span class="add-long">Ajouter</span>
            <span class="add-short">+</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</main>

<style>
  /* ② Navigation — slide in from right on nav change */
  .spread.nav-transition {
    animation: mainSlideIn 0.28s cubic-bezier(0.34, 1.1, 0.64, 1) both;
  }

  @keyframes mainSlideIn {
    from { opacity: 0; transform: translateX(18px); }
    to   { opacity: 1; transform: translateX(0); }
  }

  /* ---- head bands ---- */
  .menu { grid-column: 1 / 5; display: none; }
  .section { grid-column: 1 / 5; }
  .clean { grid-column: 5 / 9; }
  .mode { grid-column: 9 / 13; }

  .menu,
  .clean,
  .mode {
    letter-spacing: 0.12em;
    transition: color 0.14s ease;
  }
  .menu:hover,
  .mode:hover { color: var(--ink); }
  .mode[aria-pressed='false'] { color: var(--ink-faint); }

  .clean:hover:not(:disabled) { color: var(--ink); }
  .clean:disabled { color: var(--ink-faint); cursor: default; }
  .clean.armed { color: var(--accent); }

  .title-band { margin-top: var(--bl); }

  .masthead { grid-column: 1 / 9; }
  .meta-col { grid-column: 9 / 13; }

  .meta-col .folio { margin-top: var(--bl); }

  .toast {
    font-family: var(--mono);
    font-size: 11px;
    line-height: var(--lh);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--accent);
    border-left: 2px solid var(--accent);
    padding-left: 10px;
    margin-bottom: var(--lh);
  }

  /* ---- ① App Load (items) + ⑥ Sync scan container ---- */
  .items-container {
    position: relative;
    opacity: 0;
  }

  .items-container.loaded {
    animation: itemsLoad 0.4s ease-out 155ms both;
  }

  /* When not using the load animation (nav transitions), keep visible */
  .spread.nav-transition .items-container { opacity: 1; }

  @keyframes itemsLoad {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  /* ⑥ Sync — scan line */
  .scan-line {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 2px;
    z-index: 10;
    pointer-events: none;
    background: linear-gradient(90deg, transparent 0%, var(--accent) 30%, var(--ink) 70%, transparent 100%);
    animation: scanLine 0.7s cubic-bezier(0.4, 0, 0.6, 1) forwards;
  }

  /* `top` is a percentage of the container, so the line really does sweep the
     whole list — a percentage *transform* would only move it its own 2px. */
  @keyframes scanLine {
    0%   { top: 0;    opacity: 0; }
    8%   { opacity: 0.9; }
    100% { top: 100%; opacity: 0; }
  }

  .items { list-style: none; }

  /* ---- the row: 40px = 5 baselines, hairline ruled ---- */
  .item {
    align-items: center;
    min-height: 40px;
    padding-block: var(--bl) calc(var(--bl) - 1px);
    border-bottom: 1px solid var(--rule);
    cursor: grab;
  }

  .item:active { cursor: grabbing; }
  .item:hover { background: var(--wash); }
  .item:hover .del-item { opacity: 1; }

  .item.drag-over {
    background: var(--wash);
    box-shadow: inset 0 2px 0 var(--accent);
  }

  .empty-row {
    align-items: center;
    min-height: 40px;
    padding-block: var(--bl);
  }
  .empty-row .kicker { color: var(--ink-faint); }

  /* ⑥ Sync — item ripple */
  .item.synced {
    animation: rippleIn 0.9s cubic-bezier(0.34, 1.25, 0.64, 1) both;
  }

  @keyframes rippleIn {
    0%   { transform: translateX(-8px); opacity: 0; background: rgba(228, 0, 43, 0.10); }
    45%  { transform: translateX(2px);  opacity: 1; background: rgba(228, 0, 43, 0.10); }
    70%  { transform: translateX(-1px); background: rgba(228, 0, 43, 0.04); }
    100% { transform: translateX(0);    opacity: 1; background: transparent; }
  }

  /* ③ Add Item — spring entrance from above */
  .item.item-entering {
    animation: itemEnter 0.45s cubic-bezier(0.34, 1.2, 0.64, 1) both;
  }

  @keyframes itemEnter {
    0%   { opacity: 0; transform: translateY(-10px); max-height: 0; padding-top: 0; padding-bottom: 0; }
    40%  { opacity: 1; transform: translateY(2px);   max-height: 40px; }
    70%  { transform: translateY(-1px); }
    100% { opacity: 1; transform: translateY(0);     max-height: 40px; padding-top: 8px; padding-bottom: 7px; }
  }

  /* ⑤ Delete — slide right and collapse */
  .item.item-exiting {
    animation: itemExit 0.32s cubic-bezier(0.4, 0, 1, 1) forwards;
    pointer-events: none;
  }

  @keyframes itemExit {
    0%   { opacity: 1; transform: translateX(0);    max-height: 40px; padding-top: 8px; padding-bottom: 7px; }
    30%  { opacity: 0; transform: translateX(24px); }
    100% { opacity: 0; transform: translateX(24px); max-height: 0; padding-top: 0; padding-bottom: 0; border-bottom-width: 0; }
  }

  /* ---- cells: each one fills its column span exactly ---- */
  .cell { width: 100%; }

  .check {
    grid-column: 1 / 2;
    display: flex;
    align-items: center;
    height: var(--lh);
  }

  .box {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    border: 1.5px solid var(--ink);
    background: var(--paper);
    color: var(--paper);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.16s ease, border-color 0.16s ease;
  }

  .check:hover .box { border-color: var(--accent); }

  .check.checked .box {
    background: var(--accent);
    border-color: var(--accent);
  }

  /* ④ Check/Uncheck — SVG checkmark spring */
  .check-svg {
    width: 10px;
    height: 10px;
    opacity: 0;
    transform: scale(0.5) rotate(-10deg);
    transition: opacity 0.15s, transform 0.2s cubic-bezier(0.34, 1.5, 0.64, 1);
  }

  .check.checked .check-svg {
    opacity: 1;
    transform: scale(1) rotate(0deg);
  }

  .item-text {
    grid-column: 2 / 12;
    cursor: text;
    overflow-wrap: anywhere;
  }

  /* ④ Check/Uncheck — animated strikethrough. A painted background sweeps
     with the text box rather than a positioned pseudo-element, so it stays
     glued to the words even when an item wraps to a second line. */
  .t {
    background-image: linear-gradient(var(--ink-faint), var(--ink-faint));
    background-repeat: no-repeat;
    background-position: 0 56%;
    background-size: 0 1.5px;
    transition: background-size 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s ease;
  }

  .t.struck {
    color: var(--ink-faint);
    background-size: 100% 1.5px;
  }

  .item-edit {
    grid-column: 2 / 12;
    height: var(--lh);
    border: none;
    border-bottom: 1px solid var(--accent);
    background: none;
    outline: none;
    font-size: 16px;
  }

  .del-item {
    grid-column: 12 / 13;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    height: var(--lh);
    color: var(--ink-faint);
    opacity: 0;
    transition: opacity 0.14s ease, color 0.14s ease;
  }

  .del-item:hover { color: var(--accent); }

  /* touch devices have no hover, so the control must be permanently legible */
  @media (hover: none) {
    .del-item { opacity: 0.5; }
  }

  /* ---- the composer, pinned above the fold ---- */
  .composer {
    flex-shrink: 0;
    padding: var(--bl) var(--margin) calc(var(--lh) + env(safe-area-inset-bottom, 0px));
    border-top: 1px solid var(--ink);
    background: var(--paper);
  }

  .new-item-wrap { grid-column: 1 / 11; position: relative; }
  .new-item { width: 100%; }
  .add { grid-column: 11 / 13; width: 100%; }

  .add-short { display: none; }

  /* ===================================================================
     MOBILE — one column of reading, everything still on the same lines
     =================================================================== */
  @media (max-width: 640px) {
    .menu { display: block; }
    .section { display: none; }

    .clean,
    .mode { letter-spacing: 0.06em; }

    .mode { text-align: right; }

    .masthead { grid-column: 1 / -1; }

    .meta-col {
      grid-column: 1 / -1;
      display: flex;
      align-items: flex-end;
      gap: 12px;
      margin-top: var(--bl);
    }

    .meta-col .kicker,
    .meta-col .folio {
      line-height: 32px;          /* 4 baselines — matches the mobile numeral */
      margin-top: 0;
    }

    .meta-col .folio { margin-left: auto; }

    .check { grid-column: 1 / 3; }
    .item-text,
    .item-edit { grid-column: 3 / 11; }
    .del-item { grid-column: 11 / 13; }

    .new-item-wrap { grid-column: 1 / 10; }
    .add { grid-column: 10 / 13; }

    .add-long { display: none; }
    .add-short { display: block; font-size: 22px; letter-spacing: 0; }
  }
</style>
