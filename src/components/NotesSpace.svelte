<script lang="ts">
  import { api } from '../lib/api';
  import { optical } from '../lib/optical';
  import Island from './Island.svelte';
  import type { Island as IslandType, Note } from '../lib/types';

  let { note, syncKey = 0, loaded = false, navKey = 0, onOpenSidebar }: {
    note: Note;
    syncKey?: number;
    loaded?: boolean;
    navKey?: number;
    onOpenSidebar?: () => void;
  } = $props();

  let islands = $state<IslandType[]>([]);
  let draggedId = $state<string | null>(null);
  let dragOverId = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);
  let errorTimer: ReturnType<typeof setTimeout> | null = null;

  const countLabel = $derived(String(islands.length).padStart(2, '0'));

  const today = new Intl.DateTimeFormat('fr-FR', {
    day: 'numeric', month: 'long', year: 'numeric'
  }).format(new Date());

  function showError(msg: string) {
    errorMsg = msg;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => errorMsg = null, 3000);
  }

  $effect(() => {
    const id = note.id;
    const _key = syncKey;
    let cancelled = false;

    api.getIslands(id)
      .then(result => { if (!cancelled) islands = result; })
      .catch(() => { if (!cancelled) showError('Chargement de la note impossible'); });

    return () => { cancelled = true; };
  });

  async function addIsland() {
    const pos = islands.length > 0 ? islands[islands.length - 1].pos + 1000 : 1000;
    const tempId = crypto.randomUUID();
    const temp: IslandType = {
      id: tempId,
      note_id: note.id,
      kind: 'text',
      text: '',
      pos,
      media_path: null,
      media_mime: null,
      media_size: null,
    };
    islands = [...islands, temp];
    try {
      const created = await api.createIsland(note.id, 'text', '', pos);
      islands = islands.map(i => i.id === tempId ? created : i);
    } catch {
      islands = islands.filter(i => i.id !== tempId);
      showError("Erreur lors de la création de l'îlot");
    }
  }

  async function editIsland(id: string, text: string) {
    const prev = islands.find(i => i.id === id)?.text ?? '';
    islands = islands.map(i => i.id === id ? { ...i, text } : i);
    try {
      await api.updateIsland(id, text);
    } catch {
      islands = islands.map(i => i.id === id ? { ...i, text: prev } : i);
      showError("Erreur lors de la modification de l'îlot");
    }
  }

  async function deleteIsland(id: string) {
    const removed = islands.find(i => i.id === id)!;
    islands = islands.filter(i => i.id !== id);
    try {
      await api.deleteIsland(id);
    } catch {
      islands = [...islands, removed].sort((a, b) => a.pos - b.pos);
      showError("Erreur lors de la suppression de l'îlot");
    }
  }

  function handleDragStart(e: DragEvent, id: string) {
    draggedId = id;
    e.dataTransfer!.effectAllowed = 'move';
  }

  function handleDragOver(e: DragEvent, id: string) {
    e.preventDefault();
    dragOverId = id;
  }

  function handleDragEnd() {
    draggedId = null;
    dragOverId = null;
  }

  async function handleDrop(e: DragEvent, targetId: string) {
    e.preventDefault();
    if (!draggedId || draggedId === targetId) {
      handleDragEnd();
      return;
    }
    const from = islands.findIndex(i => i.id === draggedId);
    const to = islands.findIndex(i => i.id === targetId);
    const reordered = [...islands];
    const [moved] = reordered.splice(from, 1);
    reordered.splice(to, 0, moved);
    handleDragEnd();

    const previous = islands;
    // Rewrite every position on the note, same strategy as the todo list.
    const updated = reordered.map((i, index) => ({ ...i, pos: (index + 1) * 1000 }));
    islands = updated;
    try {
      await Promise.all(updated.map(i => api.reorderIsland(i.id, i.pos)));
    } catch {
      islands = previous;
      showError('Erreur lors du déplacement');
    }
  }
</script>

<main class="spread" class:nav-transition={navKey > 0}>
  <div class="wrap">
    <div class="reading">
      <div class="grid">

        <div class="band">
          <button class="kicker menu" onclick={onOpenSidebar} aria-label="Ouvrir les notes">
            ☰&nbsp;&nbsp;Notes
          </button>
          <p class="kicker section">Note</p>
        </div>

        <div class="band"><div class="rule ink full"></div></div>

        <div class="band title-band">
          <h1 class="masthead" use:optical={note.title}>{note.title}</h1>
          <div class="meta-col">
            <p class="numeral" use:optical={countLabel}>{countLabel}</p>
            <p class="kicker accent">Îlots</p>
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

      <div class="islands-container" class:loaded={loaded && navKey === 0}>
        <ul class="grid islands">
          {#each islands as island (island.id)}
            <Island
              {island}
              dragOver={dragOverId === island.id}
              onEdit={editIsland}
              onDelete={deleteIsland}
              onDragStart={handleDragStart}
              onDragOver={handleDragOver}
              onDragLeave={() => dragOverId = null}
              onDrop={handleDrop}
              onDragEnd={handleDragEnd}
            />
          {/each}

          {#if islands.length === 0}
            <li class="band empty-row">
              <p class="kicker full">Note vide</p>
            </li>
          {/if}
        </ul>
      </div>
    </div>

    <div class="composer">
      <div class="grid">
        <div class="band">
          <button class="btn accent add" onclick={addIsland}>+&nbsp;&nbsp;Texte</button>
        </div>
      </div>
    </div>
  </div>
</main>

<style>
  .spread.nav-transition {
    animation: mainSlideIn 0.28s cubic-bezier(0.34, 1.1, 0.64, 1) both;
  }

  @keyframes mainSlideIn {
    from { opacity: 0; transform: translateX(12px); }
    to   { opacity: 1; transform: translateX(0); }
  }

  .menu {
    grid-column: 1 / 4;
    text-align: left;
    padding: 0;
    transition: color 0.14s ease;
  }

  .menu:hover { color: var(--accent); }

  .section { grid-column: 4 / 8; }

  .title-band { margin-top: var(--bl); }

  .masthead { grid-column: 1 / 9; }

  .meta-col { grid-column: 9 / 13; }

  .meta-col .folio { margin-top: var(--bl); }

  .toast {
    font-family: var(--mono);
    font-size: 11px;
    line-height: var(--lh);
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--accent);
    border-left: 2px solid var(--accent);
    padding-left: 10px;
    margin-bottom: var(--lh);
  }

  /* Sibling of the scroll box, pinned to the bottom of the dvh shell —
     same construction as the todo composer. */
  .composer {
    flex-shrink: 0;
    padding: var(--bl) var(--margin) calc(var(--lh) + env(safe-area-inset-bottom, 0px));
    border-top: 1px solid var(--ink);
    background: var(--paper);
  }

  .islands-container { opacity: 1; }

  .islands-container.loaded {
    animation: islandsLoad 0.45s cubic-bezier(0.34, 1.1, 0.64, 1) 120ms both;
  }

  @keyframes islandsLoad {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .islands {
    list-style: none;
    margin-top: var(--bl);
  }

  .empty-row { padding: var(--lh) 0; }

  .add { grid-column: 1 / 4; }

  @media (max-width: 640px) {
    .menu { grid-column: 1 / 7; }
    .section { display: none; }
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

    .add { grid-column: 1 / -1; }
  }
</style>
