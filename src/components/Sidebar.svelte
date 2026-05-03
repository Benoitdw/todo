<script lang="ts">
  import type { List } from '../lib/types';

  let {
    lists,
    selectedId,
    onSelect,
    onCreate,
    onDelete,
    onRename,
    onReorder,
    onOpenSettings,
  }: {
    lists: List[];
    selectedId: string | null;
    onSelect: (id: string) => void;
    onCreate: (title: string) => Promise<void>;
    onDelete: (id: string) => Promise<void>;
    onRename: (id: string, title: string) => Promise<void>;
    onReorder: (reordered: List[]) => Promise<void>;
    onOpenSettings: () => void;
  } = $props();

  let search = $state('');
  let addingList = $state(false);
  let newTitle = $state('');
  let editingId = $state<string | null>(null);
  let editTitle = $state('');
  let draggedId = $state<string | null>(null);
  let dragOverId = $state<string | null>(null);

  const filtered = $derived(
    search.trim()
      ? lists.filter(l => l.title.toLowerCase().includes(search.toLowerCase()))
      : lists
  );

  async function handleCreate() {
    const t = newTitle.trim();
    addingList = false;
    newTitle = '';
    if (t) await onCreate(t);
  }

  function startEdit(list: List) {
    editingId = list.id;
    editTitle = list.title;
  }

  async function commitEdit() {
    if (editingId && editTitle.trim()) {
      await onRename(editingId, editTitle.trim());
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
    const from = lists.findIndex(l => l.id === draggedId);
    const to = lists.findIndex(l => l.id === targetId);
    const reordered = [...lists];
    const [moved] = reordered.splice(from, 1);
    reordered.splice(to, 0, moved);
    draggedId = null;
    dragOverId = null;
    await onReorder(reordered);
  }

  function handleDragEnd() {
    draggedId = null;
    dragOverId = null;
  }

  let newInputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (addingList && newInputEl) newInputEl.focus();
  });
</script>

<aside class="sidebar">
  <div class="search-wrap">
    <input
      class="search"
      type="text"
      placeholder="Rechercher..."
      bind:value={search}
    />
  </div>

  <div class="toolbar">
    <button class="icon-btn" title="Nouvelle liste" onclick={() => { addingList = true; newTitle = ''; }}>+</button>
    <span class="icon-btn filter-icon">▽</span>
    <button class="icon-btn settings-btn" title="Paramètres" onclick={onOpenSettings}>⚙</button>
  </div>

  {#if addingList}
    <div class="new-list">
      <input
        bind:this={newInputEl}
        type="text"
        placeholder="Nom de la liste"
        bind:value={newTitle}
        onkeydown={(e) => {
          if (e.key === 'Enter') handleCreate();
          if (e.key === 'Escape') { addingList = false; newTitle = ''; }
        }}
        onblur={handleCreate}
      />
    </div>
  {/if}

  <ul class="list-nav">
    {#each filtered as list (list.id)}
      <li
        class="list-item"
        class:selected={selectedId === list.id}
        class:drag-over={dragOverId === list.id}
        draggable="true"
        role="button"
        tabindex="0"
        ondragstart={(e) => handleDragStart(e, list.id)}
        ondragover={(e) => handleDragOver(e, list.id)}
        ondragleave={() => dragOverId = null}
        ondrop={(e) => handleDrop(e, list.id)}
        ondragend={handleDragEnd}
        onclick={() => onSelect(list.id)}
        ondblclick={() => startEdit(list)}
        onkeydown={(e) => e.key === 'Enter' && onSelect(list.id)}
      >
        {#if editingId === list.id}
          <input
            class="edit-input"
            type="text"
            bind:value={editTitle}
            onblur={commitEdit}
            onkeydown={(e) => {
              if (e.key === 'Enter') commitEdit();
              if (e.key === 'Escape') editingId = null;
            }}
          />
        {:else}
          <span class="list-name">{list.title}</span>
          <button
            class="del-btn"
            tabindex="-1"
            title="Supprimer"
            onclick={(e) => { e.stopPropagation(); onDelete(list.id); }}
          >×</button>
        {/if}
      </li>
    {/each}
  </ul>
</aside>

<style>
  .sidebar {
    width: 200px;
    min-width: 200px;
    border-right: 1px solid #e0e0e0;
    background: #f5f5f5;
    display: flex;
    flex-direction: column;
    padding: 12px 8px;
    gap: 6px;
  }

  .search-wrap {
    padding: 0 2px;
  }

  .search {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid #ddd;
    border-radius: 5px;
    font-size: 0.82rem;
    background: white;
    outline: none;
  }

  .search:focus { border-color: #aaa; }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 2px;
  }

  .settings-btn {
    margin-left: auto;
    font-size: 1rem;
  }

  .icon-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.1rem;
    color: #666;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1.4;
  }

  .icon-btn:hover { background: #e0e0e0; }

  .filter-icon {
    font-size: 0.85rem;
    cursor: default;
  }

  .new-list input {
    width: 100%;
    padding: 5px 8px;
    border: 1px solid #bbb;
    border-radius: 5px;
    font-size: 0.85rem;
    outline: none;
  }

  .list-nav {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow-y: auto;
    flex: 1;
  }

  .list-item {
    display: flex;
    align-items: center;
    padding: 7px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.88rem;
    color: #333;
    border: 1px solid transparent;
  }

  .list-item:hover { background: #eaeaea; }
  .list-item:hover .del-btn { opacity: 1; }

  .list-item.selected {
    background: #e0e0e0;
    font-weight: 500;
  }

  .list-item.drag-over {
    background: #d8d8d8;
    border-color: #bbb;
    border-style: dashed;
  }

  .list-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .del-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: #bbb;
    font-size: 1rem;
    opacity: 0;
    padding: 0 2px;
    line-height: 1;
    flex-shrink: 0;
  }

  .del-btn:hover { color: #e33; }

  .edit-input {
    flex: 1;
    border: 1px solid #bbb;
    border-radius: 3px;
    padding: 1px 5px;
    font-size: 0.88rem;
    outline: none;
  }
</style>
