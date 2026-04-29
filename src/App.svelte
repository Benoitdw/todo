<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TodoList from './components/TodoList.svelte';
  import Setup from './components/Setup.svelte';
  import { api } from './lib/api';
  import type { List } from './lib/types';

  let lists = $state<List[]>([]);
  let selectedId = $state<string | null>(null);
  let hasConfig = $state<boolean | null>(null);

  const selectedList = $derived(lists.find(l => l.id === selectedId) ?? null);

  onMount(async () => {
    const result = await api.getConfig();
    hasConfig = result !== null;
    if (hasConfig) {
      await loadLists();
    }
  });

  async function loadLists() {
    lists = await api.getLists();
    if (lists.length > 0) selectedId = lists[0].id;
  }

  async function onSetupComplete() {
    hasConfig = true;
    await loadLists();
  }

  async function createList(title: string) {
    const pos = lists.length > 0 ? lists[lists.length - 1].pos + 1000 : 1000;
    const newList = await api.createList(title, pos);
    lists = [...lists, newList];
    selectedId = newList.id;
  }

  async function deleteList(id: string) {
    await api.deleteList(id);
    lists = lists.filter(l => l.id !== id);
    if (selectedId === id) selectedId = lists[0]?.id ?? null;
  }

  async function renameList(id: string, title: string) {
    await api.updateList(id, title);
    lists = lists.map(l => l.id === id ? { ...l, title } : l);
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
  <div class="app">
    <Sidebar
      {lists}
      {selectedId}
      onSelect={(id) => selectedId = id}
      onCreate={createList}
      onDelete={deleteList}
      onRename={renameList}
      onReorder={reorderLists}
    />
    {#if selectedList}
      <TodoList list={selectedList} />
    {:else}
      <div class="empty">
        <p>Crée une liste pour commencer</p>
      </div>
    {/if}
  </div>
{/if}

<style>
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
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
