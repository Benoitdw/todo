<script lang="ts">
  import type { List } from '../lib/types';

  let {
    lists,
    selectedId,
    loaded,
    isMobile = false,
    onSelect,
    onCreate,
    onDelete,
    onRename,
    onReorder,
    onOpenSettings,
    onClose,
  }: {
    lists: List[];
    selectedId: string | null;
    loaded: boolean;
    isMobile?: boolean;
    onSelect: (id: string) => void;
    onCreate: (title: string) => Promise<void>;
    onDelete: (id: string) => Promise<void>;
    onRename: (id: string, title: string) => Promise<void>;
    onReorder: (reordered: List[]) => Promise<void>;
    onOpenSettings: () => void;
    onClose?: () => void;
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
  <div class="head">
    <p class="kicker">Listes</p>
    <span class="folio count">{String(lists.length).padStart(2, '0')}</span>
    {#if isMobile}
      <button class="close" onclick={onClose} aria-label="Fermer">×</button>
    {/if}
  </div>

  <input
    class="search"
    type="text"
    placeholder="Rechercher"
    bind:value={search}
  />

  {#if addingList}
    <input
      class="search new"
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
  {/if}

  <ul class="nav">
    {#each filtered as list (list.id)}
      <li
        class="row"
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
        <!-- ② Navigation — active indicator bar -->
        <span class="bar"></span>

        {#if editingId === list.id}
          <input
            class="edit"
            type="text"
            bind:value={editTitle}
            onblur={commitEdit}
            onkeydown={(e) => {
              if (e.key === 'Enter') commitEdit();
              if (e.key === 'Escape') editingId = null;
            }}
          />
        {:else}
          <span class="name">{list.title}</span>
          <button
            class="del"
            tabindex="-1"
            title="Supprimer"
            onclick={(e) => { e.stopPropagation(); onDelete(list.id); }}
            aria-label="Supprimer la liste"
          >×</button>
        {/if}
      </li>
    {/each}
  </ul>

  <div class="foot">
    <button class="action" onclick={() => { addingList = true; newTitle = ''; }}>
      + &nbsp;Nouvelle liste
    </button>
    <div class="foot-row">
      <button class="action" onclick={onOpenSettings}>Réglages</button>
    </div>
  </div>
</aside>

<style>
  /* ① App Load — sidebar */
  .sidebar {
    width: var(--sidebar);
    min-width: var(--sidebar);
    border-right: 1px solid var(--ink);
    background: var(--paper);
    display: flex;
    flex-direction: column;
    padding: var(--lh) 24px calc(var(--lh) + env(safe-area-inset-bottom, 0px));
    opacity: 0;
  }

  .sidebar.loaded {
    animation: sidebarLoad 0.4s cubic-bezier(0.34, 1.1, 0.64, 1) 45ms both;
  }

  @keyframes sidebarLoad {
    from { opacity: 0; transform: translateX(-16px); }
    to   { opacity: 1; transform: translateX(0); }
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    height: var(--lh);
    margin-bottom: var(--lh);
  }

  .count { margin-left: auto; }

  .close {
    font-size: 22px;
    line-height: var(--lh);
    color: var(--ink);
    padding-left: 8px;
  }

  .search {
    height: 32px;                    /* 4 baselines */
    padding: 0 8px;
    margin-bottom: var(--bl);
    border: 1px solid var(--rule);
    background: var(--paper);
    outline: none;
    font-size: 16px;
    flex-shrink: 0;
  }

  .search::placeholder { color: var(--ink-faint); }
  .search:focus { border-color: var(--ink); }
  .search.new { border-color: var(--accent); }

  .nav {
    list-style: none;
    overflow-y: auto;
    flex: 1;
    margin-top: var(--bl);
    padding-top: var(--bl);
    border-top: 1px solid var(--rule);
    scrollbar-width: none;
  }
  .nav::-webkit-scrollbar { width: 0; }

  /* ② Navigation — 32px rows, one accent bar for the active list */
  .row {
    position: relative;
    display: flex;
    align-items: center;
    height: 32px;                    /* 4 baselines */
    padding-left: 12px;
    cursor: pointer;
    font-size: 15px;
    color: var(--ink-mid);
    transition: color 0.14s ease, background 0.14s ease;
  }

  .row:hover { background: var(--wash); color: var(--ink); }
  .row:hover .del { opacity: 1; }

  .row.selected {
    color: var(--ink);
    font-weight: 600;
  }

  .row.drag-over { box-shadow: inset 0 -2px 0 var(--accent); }

  .bar {
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 3px;
    background: var(--accent);
    transform: scaleY(0);
    transition: transform 0.2s cubic-bezier(0.34, 1.4, 0.64, 1);
  }

  .row.selected .bar { transform: scaleY(1); }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .del {
    font-size: 18px;
    line-height: 1;
    color: var(--ink-faint);
    opacity: 0;
    padding: 0 2px;
    flex-shrink: 0;
    transition: opacity 0.14s ease, color 0.14s ease;
  }

  .del:hover { color: var(--accent); }

  @media (hover: none) {
    .del { opacity: 0.5; }
  }

  .edit {
    flex: 1;
    height: 24px;
    border: none;
    border-bottom: 1px solid var(--accent);
    background: none;
    outline: none;
    font-size: 15px;
  }

  .foot {
    flex-shrink: 0;
    margin-top: var(--lh);
    padding-top: var(--bl);
    border-top: 1px solid var(--ink);
  }

  .foot-row {
    display: flex;
    gap: 16px;
    margin-top: var(--bl);
  }

  .action {
    font-family: var(--mono);
    font-size: 11px;
    line-height: var(--lh);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--ink-mid);
    transition: color 0.14s ease;
  }

  .action:hover { color: var(--accent); }

  /* ===================================================================
     MOBILE — the sidebar is a drawer over the content, never a column
     that squeezes it.
     =================================================================== */
  @media (max-width: 640px) {
    .sidebar {
      position: fixed;
      z-index: 80;
      top: 0;
      left: 0;
      bottom: 0;
      width: min(300px, 84vw);
      min-width: 0;
      padding: var(--lh) 20px calc(var(--lh) + env(safe-area-inset-bottom, 0px));
      box-shadow: 1px 0 0 var(--ink);
      animation: drawerIn 0.24s cubic-bezier(0.34, 1.1, 0.64, 1) both;
      opacity: 1;
    }

    /* the drawer must open even before the shell finished its load animation */
    .sidebar.loaded { animation: drawerIn 0.24s cubic-bezier(0.34, 1.1, 0.64, 1) both; }

    @keyframes drawerIn {
      from { opacity: 0; transform: translateX(-100%); }
      to   { opacity: 1; transform: translateX(0); }
    }
  }
</style>
