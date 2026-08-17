<script lang="ts">
  import { onMount } from 'svelte';

  let { initialSvg = null, onSave, onCancel }: {
    /** Markup of an existing sketch, reopened for further editing. */
    initialSvg?: string | null;
    onSave: (svg: string) => Promise<void> | void;
    onCancel: () => void;
  } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let saving = $state(false);
  let ready = $state(false);
  let editor: import('js-draw').Editor | null = null;

  onMount(() => {
    let disposed = false;

    // Loaded on demand: the drawing engine is dead weight for anyone who never
    // opens a sketch, and this is the only place that needs it.
    (async () => {
      // The prebuilt stylesheet, not `js-draw/styles`: that entry points at raw
      // SCSS and would drag a sass toolchain into the build for nothing.
      const [{ Editor }] = await Promise.all([
        import('js-draw'),
        import('js-draw/Editor.css'),
      ]);
      if (disposed || !host) return;

      editor = new Editor(host, {
        wheelEventsEnabled: 'only-if-focused',
      });
      editor.addToolbar();
      editor.getRootElement().style.height = '100%';

      if (initialSvg) await editor.loadFromSVG(initialSvg);
      ready = true;
      editor.focus();
    })();

    return () => {
      disposed = true;
      editor?.remove();
      editor = null;
    };
  });

  async function save() {
    if (!editor || saving) return;
    saving = true;
    try {
      await onSave(editor.toSVG().outerHTML);
    } finally {
      saving = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    // Escape belongs to the editor's own tools first (deselect, cancel a stroke);
    // only the explicit button leaves, so a sketch is never lost by reflex.
    if (e.key === 'Escape') e.stopPropagation();
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="sheet" role="dialog" aria-modal="true" aria-label="Croquis">
  <div class="bar">
    <p class="kicker">Croquis</p>
    <button class="kicker cancel" onclick={onCancel}>Annuler</button>
    <button class="btn accent" disabled={!ready || saving} onclick={save}>
      {saving ? 'Enregistrement…' : 'Enregistrer'}
    </button>
  </div>
  <div class="canvas" bind:this={host}></div>
</div>

<style>
  .sheet {
    position: fixed;
    inset: 0;
    z-index: 130;
    display: flex;
    flex-direction: column;
    background: var(--paper);

  }

  /* js-draw defines its theme variables on .imageEditorContainer itself, and
     swaps to a dark palette under prefers-color-scheme: dark. Setting them on
     the parent is not enough — they have to land on that same element to win. */
  .sheet :global(.imageEditorContainer) {
    --background-color-1: var(--paper);
    --background-color-2: var(--wash);
    --background-color-3: var(--rule);
    --background-color-transparent: rgba(255, 255, 255, 0.7);
    --foreground-color-1: var(--ink);
    --foreground-color-2: var(--ink);
    --foreground-color-3: var(--ink-mid);
    --border-color: var(--rule);
    --active-border-color: var(--accent);
    --icon-color: var(--ink);
    --primary-action-foreground-color: var(--accent);
    --selection-background-color: var(--accent);
    --selection-foreground-color: var(--paper);
    --shadow-color: rgba(17, 19, 21, 0.16);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
    height: 56px;                    /* 7 baselines */
    padding: 0 var(--margin);
    border-bottom: 1px solid var(--ink);
  }

  .bar .cancel {
    margin-left: auto;
    color: var(--ink-mid);
    padding: 0 4px;
    transition: color 0.14s ease;
  }

  .bar .cancel:hover { color: var(--accent); }

  .canvas {
    flex: 1;
    min-height: 0;
    /* js-draw paints its own surface; the app's grid has no say inside it. */
    touch-action: none;
  }
</style>
