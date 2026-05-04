<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TodoList from './components/TodoList.svelte';
  import Setup from './components/Setup.svelte';
  import Settings from './components/Settings.svelte';
  import { api } from './lib/api';
  import type { List } from './lib/types';

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  let lists = $state<List[]>([]);
  let selectedId = $state<string | null>(null);
  let hasConfig = $state<boolean | null>(null);
  let showSettings = $state(false);
  let windowWidth = $state(window.innerWidth);
  let sidebarOpen = $state(true);
  let errorMsg = $state<string | null>(null);
  let errorTimer: ReturnType<typeof setTimeout> | null = null;
  let syncKey = $state(0);
  let loaded = $state(false);
  let navKey = $state(0);
  let navFading = $state(false);

  function showError(msg: string) {
    errorMsg = msg;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => errorMsg = null, 3000);
  }

  const selectedList = $derived(lists.find(l => l.id === selectedId) ?? null);
  const isMobile = $derived(windowWidth <= 640);

  onMount(async () => {
    const handler = () => { windowWidth = window.innerWidth; };
    window.addEventListener('resize', handler);

    const result = await api.getConfig();
    hasConfig = result !== null;
    if (hasConfig) {
      await loadLists();
    }
    setTimeout(() => { loaded = true; }, 60);

    let unlisten: (() => void) | undefined;
    if (isTauri) {
      const { listen } = await import('@tauri-apps/api/event');
      unlisten = await listen('sync:completed', async () => {
        lists = await api.getLists();
        syncKey++;
      });
    } else if (hasConfig) {
      unlisten = api.connectEvents(async () => {
        lists = await api.getLists();
        syncKey++;
      });
    }

    return () => {
      window.removeEventListener('resize', handler);
      unlisten?.();
    };
  });

  async function loadLists() {
    lists = await api.getLists();
    if (lists.length > 0) {
      selectedId = lists[0].id;
      if (isMobile) sidebarOpen = false;
    }
  }

  async function onSetupComplete() {
    hasConfig = true;
    await loadLists();
    setTimeout(() => { loaded = true; }, 60);
  }

  function handleSelect(id: string) {
    if (id === selectedId && !isMobile) return;
    navFading = true;
    setTimeout(() => {
      selectedId = id;
      navKey++;
      navFading = false;
      if (isMobile) sidebarOpen = false;
    }, 140);
  }

  async function createList(title: string) {
    const pos = lists.length > 0 ? lists[lists.length - 1].pos + 1000 : 1000;
    const tempId = crypto.randomUUID();
    const tempList: List = { id: tempId, title, pos };
    lists = [...lists, tempList];
    selectedId = tempId;
    if (isMobile) sidebarOpen = false;
    try {
      const newList = await api.createList(title, pos);
      lists = lists.map(l => l.id === tempId ? newList : l);
      selectedId = newList.id;
    } catch {
      lists = lists.filter(l => l.id !== tempId);
      if (selectedId === tempId) selectedId = lists[0]?.id ?? null;
      showError('Erreur lors de la création de la liste');
    }
  }

  async function deleteList(id: string) {
    const list = lists.find(l => l.id === id)!;
    const prevSelectedId = selectedId;
    lists = lists.filter(l => l.id !== id);
    if (selectedId === id) selectedId = lists[0]?.id ?? null;
    try {
      await api.deleteList(id);
    } catch {
      lists = [...lists, list].sort((a, b) => a.pos - b.pos);
      selectedId = prevSelectedId;
      showError('Erreur lors de la suppression de la liste');
    }
  }

  async function renameList(id: string, title: string) {
    const prevTitle = lists.find(l => l.id === id)?.title ?? '';
    lists = lists.map(l => l.id === id ? { ...l, title } : l);
    try {
      await api.updateList(id, title);
    } catch {
      lists = lists.map(l => l.id === id ? { ...l, title: prevTitle } : l);
      showError('Erreur lors du renommage de la liste');
    }
  }

  async function reorderLists(reordered: List[]) {
    const updated = reordered.map((l, i) => ({ ...l, pos: (i + 1) * 1000 }));
    lists = updated;
    await Promise.all(updated.map(l => api.reorderList(l.id, l.pos)));
  }
</script>

{#if hasConfig === null}
  <div></div>
{:else if !hasConfig}
  <Setup onComplete={onSetupComplete} />
{:else}
  {#if errorMsg}
    <div class="error-toast">{errorMsg}</div>
  {/if}
  <div class="app" class:loaded>
    {#if sidebarOpen || !isMobile}
      <Sidebar
        {lists}
        {selectedId}
        {loaded}
        onSelect={handleSelect}
        onCreate={createList}
        onDelete={deleteList}
        onRename={renameList}
        onReorder={reorderLists}
        onOpenSettings={() => showSettings = true}
      />
    {/if}
    <div class="main-area">
      <div class="nav-overlay" class:fading={navFading}></div>
      {#if showSettings}
        <Settings onClose={() => showSettings = false} />
      {/if}
      {#key navKey}
        {#if selectedList}
          <TodoList
            list={selectedList}
            {isMobile}
            {syncKey}
            {loaded}
            {navKey}
            onOpenSidebar={() => sidebarOpen = true}
          />
        {:else}
          <div class="empty">
            <p>Crée une liste pour commencer</p>
          </div>
        {/if}
      {/key}
    </div>
  </div>
{/if}

<style>
  .error-toast {
    position: fixed;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    background: #fee2e2;
    color: #b91c1c;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.85rem;
    border: 1px solid #fca5a5;
    z-index: 100;
    pointer-events: none;
  }

  /* ① App Load — shell */
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
    opacity: 0;
  }

  .app.loaded {
    animation: shellLoad 0.5s cubic-bezier(0.34, 1.1, 0.64, 1) both;
  }

  @keyframes shellLoad {
    from { opacity: 0; transform: translateY(10px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  /* ② Navigation — main area wrapper + overlay */
  .main-area {
    flex: 1;
    position: relative;
    display: flex;
    overflow: hidden;
  }

  .nav-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 20;
    background: white;
    opacity: 0;
    transition: opacity 0.28s ease-out;
  }

  .nav-overlay.fading {
    opacity: 0.6;
    transition: opacity 0.14s ease-in;
  }

  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #aaa;
    font-size: 0.9rem;
  }
</style>
