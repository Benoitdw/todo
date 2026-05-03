<script lang="ts">
  import { api } from '../lib/api';
  import type { Item, List } from '../lib/types';

  let { list, isMobile = false, onOpenSidebar }: {
    list: List;
    isMobile?: boolean;
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

  const visible = $derived(showCompleted ? items : items.filter(i => !i.checked));

  const today = new Intl.DateTimeFormat('fr-FR', {
    day: 'numeric', month: 'long', year: 'numeric'
  }).format(new Date());

  $effect(() => {
    const id = list.id;
    let cancelled = false;
    api.getItems(id).then(result => {
      if (!cancelled) items = result;
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
    items = [...items, tempItem];
    newItemText = '';
    try {
      const item = await api.createItem(list.id, text, pos);
      items = items.map(i => i.id === tempId ? item : i);
    } catch {
      items = items.filter(i => i.id !== tempId);
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
    const item = items.find(i => i.id === id)!;
    items = items.filter(i => i.id !== id);
    if (editingId === id) editingId = null;
    try {
      await api.deleteItem(id);
    } catch {
      items = [...items, item].sort((a, b) => a.pos - b.pos);
      showError('Erreur lors de la suppression');
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

<main class="todo-list">
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

  <ul class="items">
    {#each visible as item (item.id)}
      <li
        class="item"
        class:drag-over={dragOverId === item.id}
        draggable="true"
        ondragstart={(e) => handleDragStart(e, item.id)}
        ondragover={(e) => handleDragOver(e, item.id)}
        ondragleave={() => dragOverId = null}
        ondrop={(e) => handleDrop(e, item.id)}
        ondragend={handleDragEnd}
      >
        <button
          class="checkbox"
          class:checked={item.checked}
          onclick={() => toggleItem(item)}
          aria-label={item.checked ? 'Décocher' : 'Cocher'}
        ></button>

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
          <span
            class="item-text"
            class:done={item.checked}
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
        >×</button>
      </li>
    {/each}
  </ul>

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
  .todo-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 36px 48px 24px;
    overflow: hidden;
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
    color: #666;
    padding: 2px 10px 2px 0;
    line-height: 1;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .hamburger:hover { color: #333; }

  h1 {
    font-size: 1.4rem;
    font-weight: 600;
    color: #1a1a1a;
  }

  .date {
    font-size: 0.82rem;
    color: #999;
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

  .toggle-done:hover { color: #888; }

  .error-toast {
    background: #fee2e2;
    color: #b91c1c;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
    margin-bottom: 8px;
    border: 1px solid #fca5a5;
  }

  .items {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    flex: 1;
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
  .item:hover { background: #f0f0f0; }
  .item:hover .del-item { opacity: 1; }

  .item.drag-over {
    background: #eaeaea;
    border-color: #ccc;
    border-style: dashed;
  }

  .checkbox {
    width: 18px;
    height: 18px;
    min-width: 18px;
    border: 2px solid #ccc;
    border-radius: 3px;
    background: white;
    cursor: pointer;
    padding: 0;
    position: relative;
    flex-shrink: 0;
  }

  .checkbox.checked {
    background: #3b82f6;
    border-color: #3b82f6;
  }

  .checkbox.checked::after {
    content: '';
    position: absolute;
    left: 3px;
    top: 0px;
    width: 5px;
    height: 9px;
    border: 2px solid white;
    border-top: none;
    border-left: none;
    transform: rotate(45deg);
  }

  .item-text {
    flex: 1;
    font-size: 0.92rem;
    color: #222;
    cursor: text;
    line-height: 1.4;
  }

  .item-text.done {
    text-decoration: line-through;
    color: #bbb;
  }

  .item-edit {
    flex: 1;
    border: 1px solid #bbb;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 0.92rem;
    outline: none;
  }

  .del-item {
    background: none;
    border: none;
    cursor: pointer;
    color: #ccc;
    font-size: 1rem;
    opacity: 0;
    padding: 0 2px;
    line-height: 1;
    flex-shrink: 0;
  }

  .del-item:hover { color: #e33; }

  .add-item {
    display: flex;
    gap: 8px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid #eee;
  }

  .add-item input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 0.9rem;
    outline: none;
    background: white;
  }

  .add-item input:focus { border-color: #aaa; }

  .add-item button {
    padding: 8px 16px;
    background: #1a1a1a;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 300;
  }

  .add-item button:hover { background: #333; }
</style>
