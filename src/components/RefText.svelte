<script lang="ts">
  import { refStore, refHref, parseSegments, KIND_LABELS } from '../lib/refs.svelte';

  let { text }: { text: string } = $props();

  const segments = $derived(parseSegments(text));
</script><!--
  Chips are plain anchors carrying data-ref-link; App.svelte intercepts the
  click once for the whole document, so no navigation callback has to be
  threaded through every component that can hold text.
--><!--
-->{#each segments as seg, i (i)}{#if seg.type === 'text'}{seg.value}{:else}{@const ref = refStore.find(seg.kind, seg.id)}{#if ref}<a
  class="ref-chip"
  data-ref-link
  href={refHref(ref)}
><span class="ref-kind">{KIND_LABELS[ref.kind]}</span>{ref.label}</a>{:else}<span
  class="ref-chip dead"
  title="La cible de ce lien n'existe plus"
><span class="ref-kind">{KIND_LABELS[seg.kind]}</span>cible introuvable</span>{/if}{/if}{/each}
