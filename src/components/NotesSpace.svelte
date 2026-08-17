<script lang="ts">
  import { api } from '../lib/api';
  import { optical } from '../lib/optical';
  import Island from './Island.svelte';
  import SketchEditor from './SketchEditor.svelte';
  import { AudioRecorder, audioSupport, formatDuration } from '../lib/recorder.svelte';
  import type { Island as IslandType, IslandKind, Note } from '../lib/types';

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

  // Per-island upload state, keyed by island id. The blob is kept so "réessayer"
  // does not force the user to pick the file again within the session.
  type Upload = { progress: number; error: string | null; blob: Blob | null; mime: string };
  let uploads = $state<Record<string, Upload>>({});

  const recorder = new AudioRecorder();
  const audioOk = audioSupport();
  let fileInput = $state<HTMLInputElement | null>(null);
  let pendingKind: IslandKind = 'photo';

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

  function nextPos() {
    return islands.length > 0 ? islands[islands.length - 1].pos + 1000 : 1000;
  }

  async function addIsland(kind: IslandKind = 'text') {
    const pos = nextPos();
    const tempId = crypto.randomUUID();
    const temp: IslandType = {
      id: tempId,
      note_id: note.id,
      kind,
      text: '',
      pos,
      media_path: null,
      media_mime: null,
      media_size: null,
    };
    islands = [...islands, temp];
    try {
      const created = await api.createIsland(note.id, kind, '', pos);
      islands = islands.map(i => i.id === tempId ? created : i);
      return created;
    } catch {
      islands = islands.filter(i => i.id !== tempId);
      showError("Erreur lors de la création de l'îlot");
      return null;
    }
  }

  const MAX_BYTES: Record<string, number> = {
    photo: 15 * 1024 * 1024,
    audio: 25 * 1024 * 1024,
    video: 200 * 1024 * 1024,
    sketch: 5 * 1024 * 1024,
  };

  // Which sketch the editor is open on: an existing island, or null for a new one.
  let sketching = $state<{ islandId: string | null; svg: string | null } | null>(null);

  async function openSketch(islandId: string | null) {
    if (!islandId) {
      sketching = { islandId: null, svg: null };
      return;
    }
    // Reopening: the stored SVG is fetched back and handed to the editor, which
    // is the whole point of keeping sketches as markup rather than pixels.
    try {
      const resp = await fetch(api.islandMediaUrl(islandId));
      sketching = { islandId, svg: resp.ok ? await resp.text() : null };
    } catch {
      showError('Croquis introuvable');
    }
  }

  async function saveSketch(svg: string) {
    const blob = new Blob([svg], { type: 'image/svg+xml' });
    const target = sketching?.islandId ?? null;
    sketching = null;
    if (target) {
      await upload(target, blob, 'image/svg+xml');
      // The <img> keeps the old bytes on the same URL, so force a reload.
      islands = islands.map(i => i.id === target ? { ...i } : i);
      mediaVersion += 1;
    } else {
      await addMediaIsland('sketch', blob, 'image/svg+xml');
    }
  }

  /** Bumped on every sketch resave to bust the browser's image cache. */
  let mediaVersion = $state(0);

  /** Creates the island first — its id is what the file path and links hang off. */
  async function addMediaIsland(kind: IslandKind, blob: Blob, mime: string) {
    if (blob.size > MAX_BYTES[kind]) {
      // Refused here, before any request leaves the browser.
      showError(`Fichier trop volumineux (max ${Math.round(MAX_BYTES[kind] / 1024 / 1024)} Mo)`);
      return;
    }
    const created = await addIsland(kind);
    if (!created) return;
    await upload(created.id, blob, mime);
  }

  async function upload(islandId: string, blob: Blob, mime: string) {
    uploads = { ...uploads, [islandId]: { progress: 0, error: null, blob, mime } };
    try {
      const done = await api.uploadIslandMedia(islandId, blob, mime, (ratio) => {
        const current = uploads[islandId];
        if (current) uploads = { ...uploads, [islandId]: { ...current, progress: ratio } };
      });
      islands = islands.map(i => i.id === islandId ? { ...done, text: i.text } : i);
      const { [islandId]: _gone, ...rest } = uploads;
      uploads = rest;
    } catch (e) {
      const current = uploads[islandId];
      uploads = {
        ...uploads,
        [islandId]: { ...(current ?? { progress: 0, blob, mime }), error: (e as Error).message },
      };
    }
  }

  function retryUpload(islandId: string) {
    const pending = uploads[islandId];
    if (pending?.blob) {
      upload(islandId, pending.blob, pending.mime);
    } else {
      // The blob is gone (page reloaded since) — ask for the file again.
      const island = islands.find(i => i.id === islandId);
      if (island) pickFileFor(island.kind as IslandKind, islandId);
    }
  }

  let refillTarget: string | null = null;

  function pickFileFor(kind: IslandKind, islandId: string | null = null) {
    if (!fileInput) return;
    pendingKind = kind;
    refillTarget = islandId;
    fileInput.accept = kind === 'photo' ? 'image/*' : 'video/*';
    fileInput.value = '';
    fileInput.click();
  }

  async function onFileChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    const mime = file.type || (pendingKind === 'photo' ? 'image/jpeg' : 'video/mp4');
    if (refillTarget) {
      const target = refillTarget;
      refillTarget = null;
      await upload(target, file, mime);
    } else {
      await addMediaIsland(pendingKind, file, mime);
    }
  }

  async function recordAudio() {
    // Recorded first, island created on stop: a refused permission must leave
    // no orphan behind.
    const result = await recorder.start();
    if (!result) {
      if (recorder.error) showError(recorder.error);
      return;
    }
    await addMediaIsland('audio', result.blob, result.mime);
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
              upload={uploads[island.id] ?? null}
              {mediaVersion}
              dragOver={dragOverId === island.id}
              onRetry={retryUpload}
              onEditSketch={openSketch}
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
          {#if recorder.state === 'recording'}
            <div class="recording">
              <span class="dot"></span>
              <span class="numeral timer">{formatDuration(recorder.seconds)}</span>
              <span class="kicker">Enregistrement…</span>
              <button class="btn stop" onclick={() => recorder.stop()}>Arrêter</button>
              <button class="kicker cancel" onclick={() => recorder.cancel()}>Annuler</button>
            </div>
          {:else}
            <div class="kinds">
              <button class="btn accent" onclick={() => addIsland('text')}>+&nbsp;&nbsp;Texte</button>
              <button class="btn" onclick={() => pickFileFor('photo')}>Photo</button>
              <button class="btn" onclick={() => pickFileFor('video')}>Vidéo</button>
              <button class="btn" onclick={() => openSketch(null)}>Croquis</button>
              <button
                class="btn"
                disabled={!audioOk.ok || recorder.state === 'requesting'}
                title={audioOk.ok ? 'Enregistrer un mémo' : audioOk.reason}
                onclick={recordAudio}
              >{recorder.state === 'requesting' ? 'Micro…' : 'Audio'}</button>
              {#if !audioOk.ok}
                <span class="kicker unavailable">{audioOk.reason}</span>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- On a phone `capture` hands over to the native camera app, which is far
         more reliable than MediaRecorder for video. -->
    <input
      class="hidden-file"
      type="file"
      capture="environment"
      bind:this={fileInput}
      onchange={onFileChosen}
    />
  </div>
</main>

<!-- Outside <main>: .spread animates a transform, which would make it the
     containing block of a position:fixed child and clip the editor to the
     content column instead of the viewport. -->
{#if sketching}
  <SketchEditor
    initialSvg={sketching.svg}
    onSave={saveSketch}
    onCancel={() => sketching = null}
  />
{/if}

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

  .kinds,
  .recording {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .hidden-file { display: none; }

  .unavailable { color: var(--ink-faint); margin-left: 8px; }

  .recording .timer { font-size: 22px; line-height: var(--lh); }

  .recording .cancel {
    color: var(--ink-mid);
    padding: 0 4px;
    transition: color 0.14s ease;
  }

  .recording .cancel:hover { color: var(--accent); }

  .btn.stop { margin-left: auto; }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.25; }
  }

  @media (max-width: 640px) {
    .menu { grid-column: 1 / 7; }
    .section { display: none; }
    .masthead { grid-column: 1 / -1; }
    .kinds { flex-wrap: wrap; }
    .unavailable { flex-basis: 100%; margin-left: 0; }

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
  }
</style>
