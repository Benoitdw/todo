<script lang="ts">
  import type { List } from '../lib/types';

  let {
    lists,
    selectedId,
    loaded,
    onSelect,
    onCreate,
    onDelete,
    onRename,
    onReorder,
    onOpenSettings,
  }: {
    lists: List[];
    selectedId: string | null;
    loaded: boolean;
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

<aside class="sidebar" class:loaded>
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
        <!-- ② Navigation — active indicator pill -->
        <div class="nav-pill"></div>

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
  /* ① App Load — sidebar */
  .sidebar {
    width: 200px;
    min-width: 200px;
    border-right: 1px solid var(--border);
    background: var(--sidebar-bg);
    display: flex;
    flex-direction: column;
    padding: 12px 8px;
    gap: 6px;
    opacity: 0;
  }

  .sidebar.loaded {
    animation: sidebarLoad 0.4s cubic-bezier(0.34, 1.1, 0.64, 1) 45ms both;
  }

  @keyframes sidebarLoad {
    from { opacity: 0; transform: translateX(-16px); }
    to   { opacity: 1; transform: translateX(0); }
  }

  .search-wrap {
    padding: 0 2px;
  }

  .search {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 5px;
    font-size: 0.82rem;
    background: var(--input-bg);
    outline: none;
  }

  .search:focus { border-color: var(--border-strong); }

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
    color: var(--text-muted);
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1.4;
  }

  .icon-btn:hover { background: var(--hover); }

  .filter-icon {
    font-size: 0.85rem;
    cursor: default;
  }

  .new-list input {
    width: 100%;
    padding: 5px 8px;
    border: 1px solid var(--border-strong);
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

  /* ② Navigation — list item with pill */
  .list-item {
    position: relative;
    display: flex;
    align-items: center;
    padding: 7px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.88rem;
    color: var(--text);
    border: 1px solid transparent;
  }

  .list-item:hover { background: var(--hover); }
  .list-item:hover .del-btn { opacity: 1; }

  .list-item.selected {
    background: var(--accent-subtle);
    font-weight: 500;
    color: var(--accent);
  }

  .list-item.drag-over {
    background: var(--hover);
    border-color: var(--border-strong);
    border-style: dashed;
  }

  .nav-pill {
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 3px;
    border-radius: 0 2px 2px 0;
    background: var(--accent);
    transform: scaleY(0);
    transform-origin: center;
    transition: transform 0.2s cubic-bezier(0.34, 1.4, 0.64, 1);
  }

  .list-item.selected .nav-pill {
    transform: scaleY(1);
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
    color: var(--text-muted);
    font-size: 1rem;
    opacity: 0;
    padding: 0 2px;
    line-height: 1;
    flex-shrink: 0;
  }

  .del-btn:hover { color: var(--danger); }

  .edit-input {
    flex: 1;
    border: 1px solid var(--border-strong);
    border-radius: 3px;
    padding: 1px 5px;
    font-size: 0.88rem;
    outline: none;
  }
</style>
