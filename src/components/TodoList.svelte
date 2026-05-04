<script lang="ts">
  import { api } from '../lib/api';
  import type { Item, List } from '../lib/types';

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

  async function addItem() {
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
<main class="todo-list" class:nav-transition={navKey > 0}>
  <header>
    {#if isMobile}
      <button class="hamburger" onclick={onOpenSidebar} aria-label="Ouvrir le menu">☰</button>
    {/if}
    <div class="header-text">
      <h1>{list.title}</h1>
      <p class="date">{today}</p>
    </div>
    <button
      class="toggle-done"
      title={showCompleted ? 'Masquer les complétés' : 'Afficher les complétés'}
      onclick={() => showCompleted = !showCompleted}
    >
      {showCompleted ? '☑' : '☐'}
    </button>
  </header>

  {#if errorMsg}
    <div class="error-toast">{errorMsg}</div>
  {/if}

  <!-- ① App Load (items) + ⑥ Sync scan line -->
  <div class="items-container" class:loaded={loaded && navKey === 0}>
    {#key syncKey}
      {#if syncKey > 0}
        <div class="scan-line"></div>
      {/if}
    {/key}

    <ul class="items">
      {#each visible as item, i (item.id)}
        {#key syncedIds.has(item.id) ? item.id + syncKey : item.id}
          <li
            class="item"
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
            <!-- ④ Check/Uncheck — checkbox with SVG checkmark spring -->
            <button
              class="checkbox"
              class:checked={item.checked}
              onclick={() => toggleItem(item)}
              aria-label={item.checked ? 'Décocher' : 'Cocher'}
            >
              <svg class="check-svg" viewBox="0 0 10 10" fill="none">
                <polyline points="1.5,5 4,7.5 8.5,2.5" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>

            {#if editingId === item.id}
              <input
                class="item-edit"
                type="text"
                bind:value={editText}
                onblur={commitEdit}
                onkeydown={(e) => {
                  if (e.key === 'Enter') commitEdit();
                  if (e.key === 'Escape') editingId = null;
                }}
              />
            {:else}
              <!-- ④ Check/Uncheck — strikethrough via ::after -->
              <span
                class="item-text"
                class:struck={item.checked}
                role="button"
                tabindex="0"
                ondblclick={() => startEdit(item)}
                onkeydown={(e) => e.key === 'Enter' && startEdit(item)}
              >{item.text}</span>
            {/if}

            <button
              class="del-item"
              tabindex="-1"
              onclick={() => deleteItem(item.id)}
              aria-label="Supprimer"
            >
              <svg viewBox="0 0 14 14" fill="none" width="13" height="13">
                <path d="M2 3.5h10M5.5 3.5V2.5a1 1 0 0 1 1-1h1a1 1 0 0 1 1 1v1M6 6v4M8 6v4M3 3.5l.7 7.3a1 1 0 0 0 1 .9h4.6a1 1 0 0 0 1-.9L11 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
          </li>
        {/key}
      {/each}
    </ul>
  </div>

  <div class="add-item">
    <input
      type="text"
      placeholder="Nouvel item..."
      bind:value={newItemText}
      onkeydown={(e) => e.key === 'Enter' && addItem()}
    />
    <button onclick={addItem}>+</button>
  </div>
</main>

<style>
  /* ② Navigation — slide in from right on nav change */
  .todo-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 36px 48px 24px;
    overflow: hidden;
  }

  .todo-list.nav-transition {
    animation: mainSlideIn 0.28s cubic-bezier(0.34, 1.1, 0.64, 1) both;
  }

  @keyframes mainSlideIn {
    from { opacity: 0; transform: translateX(18px); }
    to   { opacity: 1; transform: translateX(0); }
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 28px;
  }

  .hamburger {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.2rem;
    color: var(--text-muted);
    padding: 2px 10px 2px 0;
    line-height: 1;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .hamburger:hover { color: var(--text); }

  h1 {
    font-size: 1.4rem;
    font-weight: 600;
    color: var(--text);
  }

  .date {
    font-size: 0.82rem;
    color: var(--text-muted);
    font-style: italic;
    margin-top: 3px;
  }

  .toggle-done {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.2rem;
    color: #bbb;
    padding: 0;
    margin-top: 2px;
    flex-shrink: 0;
  }

  .toggle-done:hover { color: var(--text-muted); }

  .error-toast {
    background: #fee2e2;
    color: #b91c1c;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
    margin-bottom: 8px;
    border: 1px solid #fca5a5;
  }

  /* ① App Load (items) + ⑥ Sync scan container */
  .items-container {
    flex: 1;
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    opacity: 0;
  }

  .items-container.loaded {
    animation: itemsLoad 0.4s ease-out 155ms both;
  }

  /* When not using the load animation (nav transitions), keep visible */
  .todo-list.nav-transition .items-container {
    opacity: 1;
  }

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
    background: linear-gradient(90deg,
      transparent 0%,
      var(--accent) 25%,
      var(--accent-light) 55%,
      transparent 100%
    );
    animation: scanLine 0.7s cubic-bezier(0.4, 0, 0.6, 1) forwards;
  }

  @keyframes scanLine {
    0%   { transform: translateY(-100%); opacity: 0; }
    8%   { opacity: 0.9; }
    100% { transform: translateY(calc(100% + 4px)); opacity: 0; }
  }

  .items {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    flex: 1;
  }

  /* ⑥ Sync — item ripple (indigo) */
  .item.synced {
    animation: rippleIn 0.9s cubic-bezier(0.34, 1.25, 0.64, 1) both;
  }

  @keyframes rippleIn {
    0%   { transform: translateX(-8px); opacity: 0; background: rgba(99, 102, 241, 0.12); }
    45%  { transform: translateX(2px);  opacity: 1; background: rgba(99, 102, 241, 0.12); }
    70%  { transform: translateX(-1px); background: rgba(99, 102, 241, 0.05); }
    100% { transform: translateX(0);    opacity: 1; background: transparent; }
  }

  /* ③ Add Item — spring entrance from above */
  .item.item-entering {
    animation: itemEnter 0.45s cubic-bezier(0.34, 1.2, 0.64, 1) both;
  }

  @keyframes itemEnter {
    0%   { opacity: 0; transform: translateY(-10px) scale(0.97); max-height: 0; padding-top: 0; padding-bottom: 0; }
    40%  { opacity: 1; transform: translateY(2px) scale(1.01); max-height: 60px; }
    70%  { transform: translateY(-1px) scale(1); }
    100% { opacity: 1; transform: translateY(0) scale(1); max-height: 60px; padding-top: 6px; padding-bottom: 6px; }
  }

  /* ⑤ Delete — slide right and collapse */
  .item.item-exiting {
    animation: itemExit 0.32s cubic-bezier(0.4, 0, 1, 1) forwards;
    pointer-events: none;
  }

  @keyframes itemExit {
    0%   { opacity: 1; transform: translateX(0);     max-height: 60px; padding-top: 6px; padding-bottom: 6px; }
    30%  { opacity: 0; transform: translateX(24px); }
    100% { opacity: 0; transform: translateX(24px);  max-height: 0;    padding-top: 0;   padding-bottom: 0; margin: 0; }
  }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: 6px;
    cursor: grab;
    border: 1px solid transparent;
  }

  .item:active { cursor: grabbing; }
  .item:hover { background: var(--hover); }
  .item:hover .del-item { opacity: 1; }

  .item.drag-over {
    background: var(--hover);
    border-color: var(--border-strong);
    border-style: dashed;
  }

  /* ④ Check/Uncheck — checkbox with transitions */
  .checkbox {
    width: 18px;
    height: 18px;
    min-width: 18px;
    border: 2px solid var(--border-strong);
    border-radius: 3px;
    background: var(--input-bg);
    cursor: pointer;
    padding: 0;
    position: relative;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.18s, border-color 0.18s, transform 0.15s;
  }

  .checkbox:hover { transform: scale(1.08); }

  .checkbox.checked {
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

  .checkbox.checked .check-svg {
    opacity: 1;
    transform: scale(1) rotate(0deg);
  }

  /* ④ Check/Uncheck — animated strikethrough */
  .item-text {
    flex: 1;
    font-size: 0.92rem;
    color: var(--text);
    cursor: text;
    line-height: 1.4;
    position: relative;
  }

  .item-text::after {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    height: 1.5px;
    background: var(--text-muted);
    width: 0;
    transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    transform: translateY(-50%);
  }

  .item-text.struck {
    color: var(--text-muted);
  }

  .item-text.struck::after {
    width: 100%;
  }

  .item-edit {
    flex: 1;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 0.92rem;
    outline: none;
  }

  .del-item {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    opacity: 0;
    padding: 2px;
    line-height: 1;
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .del-item:hover { color: var(--danger); }

  .add-item {
    display: flex;
    gap: 8px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .add-item input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.9rem;
    outline: none;
    background: var(--input-bg);
  }

  .add-item input:focus { border-color: var(--accent); }

  .add-item button {
    padding: 8px 16px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 300;
  }

  .add-item button:hover { background: var(--accent-light); }
</style>
