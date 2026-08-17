<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TodoList from './components/TodoList.svelte';
  import NotesSpace from './components/NotesSpace.svelte';
  import Setup from './components/Setup.svelte';
  import Settings from './components/Settings.svelte';
  import { api } from './lib/api';
  import { refStore } from './lib/refs.svelte';
  import type { List, Mode, Note, SidebarEntry } from './lib/types';

  let lists = $state<List[]>([]);
  let selectedId = $state<string | null>(null);
  let notes = $state<Note[]>([]);
  let selectedNoteId = $state<string | null>(null);
  let notesLoaded = false;

  // The mode is derived from the URL and never persisted, so "/" always opens
  // on the lists exactly as it did before notes existed.
  let mode = $state<Mode>('lists');
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
  const selectedNote = $derived(notes.find(n => n.id === selectedNoteId) ?? null);
  const isMobile = $derived(windowWidth <= 640);

  const entries = $derived<SidebarEntry[]>(mode === 'lists' ? lists : notes);
  const sidebarSelectedId = $derived(mode === 'lists' ? selectedId : selectedNoteId);

  // --- routing ------------------------------------------------------------
  // Three routes, no library: "/" (lists), "/notes", "/notes/:noteId".

  function parseRoute(): { mode: Mode; noteId: string | null; listId: string | null } {
    const path = location.pathname;
    if (path === '/notes' || path.startsWith('/notes/')) {
      const rest = path.slice('/notes'.length).replace(/^\//, '');
      return { mode: 'notes', noteId: rest ? decodeURIComponent(rest) : null, listId: null };
    }
    if (path.startsWith('/lists/')) {
      const rest = path.slice('/lists/'.length);
      return { mode: 'lists', noteId: null, listId: rest ? decodeURIComponent(rest) : null };
    }
    // "/" stays the untouched entry point: it opens the first list and writes
    // nothing to the history.
    return { mode: 'lists', noteId: null, listId: null };
  }

  /// Brings the #ilot-<id> / #item-<id> target of a link into view once rendered.
  function scrollToHash() {
    const hash = location.hash;
    if (!hash) return;
    requestAnimationFrame(() => {
      const el = document.querySelector(hash);
      el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
      el?.classList.add('link-target');
      setTimeout(() => el?.classList.remove('link-target'), 1600);
    });
  }

  function navigate(path: string, replace = false) {
    if (location.pathname === path) return;
    if (replace) history.replaceState({}, '', path);
    else history.pushState({}, '', path);
  }

  async function applyRoute() {
    const route = parseRoute();
    mode = route.mode;
    if (route.mode === 'notes') {
      await ensureNotes();
      // An unknown note id falls back to the list of notes rather than a blank
      // screen, and the URL is corrected so a reload lands somewhere real.
      if (route.noteId && !notes.some(n => n.id === route.noteId)) {
        selectedNoteId = null;
        navigate('/notes', true);
      } else {
        selectedNoteId = route.noteId;
      }
    } else if (route.listId) {
      if (lists.some(l => l.id === route.listId)) {
        selectedId = route.listId;
      } else {
        navigate('/', true);
      }
    }
    navKey++;
    scrollToHash();
  }

  async function ensureNotes() {
    if (notesLoaded) return;
    try {
      notes = await api.getNotes();
      notesLoaded = true;
    } catch {
      showError('Chargement des notes impossible');
    }
  }

  async function selectMode(next: Mode) {
    if (next === mode) return;
    if (next === 'notes') {
      await ensureNotes();
      mode = 'notes';
      navigate(selectedNoteId ? `/notes/${selectedNoteId}` : '/notes');
    } else {
      mode = 'lists';
      navigate('/');
    }
    if (isMobile) sidebarOpen = false;
  }

  onMount(async () => {
    const handler = () => { windowWidth = window.innerWidth; };
    window.addEventListener('resize', handler);

    const onPopState = () => { applyRoute(); };
    window.addEventListener('popstate', onPopState);

    // One document-level handler for every link chip, wherever it is rendered —
    // cheaper than threading a navigation callback through every component.
    const onRefClick = (e: MouseEvent) => {
      const anchor = (e.target as HTMLElement)?.closest?.('a[data-ref-link]');
      if (!anchor) return;
      // Modified clicks stay the browser's business (new tab, new window).
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
      e.preventDefault();
      const href = anchor.getAttribute('href');
      if (!href) return;
      history.pushState({}, '', href);
      applyRoute();
    };
    document.addEventListener('click', onRefClick);

    const result = await api.getConfig();
    hasConfig = result !== null;
    if (hasConfig) {
      // Never let a failed load leave the shell invisible — the app fades in on
      // `loaded`, so throwing here used to render a blank page with no clue why.
      try {
        await loadLists();
      } catch {
        showError('Serveur injoignable');
      }
      await applyRoute();
    }
    setTimeout(() => { loaded = true; }, 60);

    let unlisten: (() => void) | undefined;
    if (hasConfig) {
      unlisten = api.connectEvents(async () => {
        lists = await api.getLists();
        if (notesLoaded) notes = await api.getNotes();
        // Labels resolve from this catalogue, so a rename anywhere refreshes
        // every chip pointing at it.
        refStore.load(true);
        syncKey++;
      });
      refStore.load();
    }

    return () => {
      window.removeEventListener('resize', handler);
      window.removeEventListener('popstate', onPopState);
      document.removeEventListener('click', onRefClick);
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
    if (mode === 'notes') {
      handleSelectNote(id);
      return;
    }
    if (id === selectedId && !isMobile) return;
    navFading = true;
    setTimeout(() => {
      selectedId = id;
      navKey++;
      navFading = false;
      navigate(`/lists/${id}`);
      if (isMobile) sidebarOpen = false;
    }, 140);
  }

  function handleSelectNote(id: string) {
    if (id === selectedNoteId && !isMobile) return;
    navFading = true;
    setTimeout(() => {
      selectedNoteId = id;
      navKey++;
      navFading = false;
      navigate(`/notes/${id}`);
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
    if (selectedId === id) {
      selectedId = lists[0]?.id ?? null;
      if (location.pathname.startsWith('/lists/')) {
        navigate(selectedId ? `/lists/${selectedId}` : '/', true);
      }
    }
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

  async function reorderLists(reordered: SidebarEntry[]) {
    const updated = reordered.map((l, i) => ({ ...l, pos: (i + 1) * 1000 })) as List[];
    lists = updated;
    await Promise.all(updated.map(l => api.reorderList(l.id, l.pos)));
  }

  // --- notes: same optimistic patterns as the lists above -----------------

  async function createNote(title: string) {
    const pos = notes.length > 0 ? notes[notes.length - 1].pos + 1000 : 1000;
    const tempId = crypto.randomUUID();
    notes = [...notes, { id: tempId, title, pos }];
    selectedNoteId = tempId;
    if (isMobile) sidebarOpen = false;
    try {
      const created = await api.createNote(title, pos);
      notes = notes.map(n => n.id === tempId ? created : n);
      selectedNoteId = created.id;
      navigate(`/notes/${created.id}`);
    } catch {
      notes = notes.filter(n => n.id !== tempId);
      if (selectedNoteId === tempId) selectedNoteId = notes[0]?.id ?? null;
      showError('Erreur lors de la création de la note');
    }
  }

  async function deleteNote(id: string) {
    const note = notes.find(n => n.id === id)!;
    const prevSelected = selectedNoteId;
    notes = notes.filter(n => n.id !== id);
    if (selectedNoteId === id) {
      selectedNoteId = notes[0]?.id ?? null;
      navigate(selectedNoteId ? `/notes/${selectedNoteId}` : '/notes', true);
    }
    try {
      await api.deleteNote(id);
    } catch {
      notes = [...notes, note].sort((a, b) => a.pos - b.pos);
      selectedNoteId = prevSelected;
      showError('Erreur lors de la suppression de la note');
    }
  }

  async function renameNote(id: string, title: string) {
    const prevTitle = notes.find(n => n.id === id)?.title ?? '';
    notes = notes.map(n => n.id === id ? { ...n, title } : n);
    try {
      await api.updateNote(id, title);
    } catch {
      notes = notes.map(n => n.id === id ? { ...n, title: prevTitle } : n);
      showError('Erreur lors du renommage de la note');
    }
  }

  async function reorderNotes(reordered: SidebarEntry[]) {
    const updated = reordered.map((n, i) => ({ ...n, pos: (i + 1) * 1000 })) as Note[];
    notes = updated;
    await Promise.all(updated.map(n => api.reorderNote(n.id, n.pos)));
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
    {#if isMobile && sidebarOpen}
      <button class="scrim" onclick={() => sidebarOpen = false} aria-label="Fermer les listes"></button>
    {/if}

    {#if sidebarOpen || !isMobile}
      <Sidebar
        {mode}
        {entries}
        selectedId={sidebarSelectedId}
        {loaded}
        {isMobile}
        onSelectMode={selectMode}
        onSelect={handleSelect}
        onCreate={mode === 'lists' ? createList : createNote}
        onDelete={mode === 'lists' ? deleteList : deleteNote}
        onRename={mode === 'lists' ? renameList : renameNote}
        onReorder={mode === 'lists' ? reorderLists : reorderNotes}
        onOpenSettings={() => showSettings = true}
        onClose={() => sidebarOpen = false}
      />
    {/if}

    <div class="main-area">
      <div class="nav-overlay" class:fading={navFading}></div>
      {#if showSettings}
        <Settings onClose={() => showSettings = false} />
      {/if}
      {#key navKey}
        {#if mode === 'notes'}
          {#if selectedNote}
            <NotesSpace
              note={selectedNote}
              {syncKey}
              {loaded}
              {navKey}
              onOpenSidebar={() => sidebarOpen = true}
            />
          {:else}
            <div class="empty">
              <p class="kicker">Crée une note pour commencer</p>
            </div>
          {/if}
        {:else if selectedList}
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
            <p class="kicker">Crée une liste pour commencer</p>
          </div>
        {/if}
      {/key}
    </div>
  </div>
{/if}

<style>
  .error-toast {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 120;
    background: var(--accent);
    color: var(--paper);
    padding: var(--bl) var(--margin) calc(var(--bl) + env(safe-area-inset-bottom, 0px));
    font-family: var(--mono);
    font-size: 11px;
    line-height: var(--lh);
    letter-spacing: 0.12em;
    text-transform: uppercase;
    pointer-events: none;
  }

  /* ① App Load — shell */
  .app {
    display: flex;
    height: 100vh;
    height: 100dvh;
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

  /* mobile drawer scrim */
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgba(17, 19, 21, 0.32);
    border: none;
    cursor: pointer;
    animation: scrimIn 0.24s ease both;
  }

  @keyframes scrimIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  /* ② Navigation — main area wrapper + overlay */
  .main-area {
    flex: 1;
    position: relative;
    display: flex;
    min-width: 0;
    overflow: hidden;
  }

  .nav-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 20;
    background: var(--paper);
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
  }
</style>
