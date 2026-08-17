import { api } from './api';
import type { Ref, RefKind } from './types';

const KINDS: RefKind[] = ['note', 'island', 'list', 'item'];
const UUID = '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}';
const TOKEN = new RegExp(`\\[\\[(${KINDS.join('|')}):(${UUID})\\]\\]`, 'g');

export const KIND_LABELS: Record<RefKind, string> = {
  note: 'Note',
  island: 'Îlot',
  list: 'Liste',
  item: 'Item',
};

export function makeToken(kind: RefKind, id: string): string {
  return `[[${kind}:${id}]]`;
}

/** The URL a link opens, anchor included. */
export function refHref(ref: Ref): string {
  switch (ref.kind) {
    case 'note': return `/notes/${ref.id}`;
    case 'island': return `/notes/${ref.parent_id}#ilot-${ref.id}`;
    case 'list': return `/lists/${ref.id}`;
    case 'item': return `/lists/${ref.parent_id}#item-${ref.id}`;
  }
}

export type Segment =
  | { type: 'text'; value: string }
  | { type: 'ref'; kind: RefKind; id: string };

/** Splits a stored text into plain runs and link tokens, in order. */
export function parseSegments(text: string): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  TOKEN.lastIndex = 0;
  for (const m of text.matchAll(TOKEN)) {
    if (m.index! > last) out.push({ type: 'text', value: text.slice(last, m.index) });
    out.push({ type: 'ref', kind: m[1] as RefKind, id: m[2] });
    last = m.index! + m[0].length;
  }
  if (last < text.length) out.push({ type: 'text', value: text.slice(last) });
  return out;
}

/**
 * The catalogue of link targets, shared by the palette and by label resolution.
 * One fetch, invalidated by the same SSE tick that refreshes everything else —
 * a stale label is corrected on the next sync rather than on every keystroke.
 */
class RefStore {
  refs = $state<Ref[]>([]);
  private loading: Promise<void> | null = null;

  async load(force = false): Promise<void> {
    if (this.loading && !force) return this.loading;
    this.loading = api.getRefs()
      .then(r => { this.refs = r; })
      .catch(() => { /* the palette simply stays empty */ })
      .finally(() => { this.loading = null; });
    return this.loading;
  }

  find(kind: RefKind, id: string): Ref | null {
    return this.refs.find(r => r.kind === kind && r.id === id) ?? null;
  }

  /** Fuzzy-ish search across all four kinds at once, best matches first. */
  search(query: string, limit = 8): Ref[] {
    const q = query.trim().toLowerCase();
    if (!q) return this.refs.slice(0, limit);
    const scored: { ref: Ref; score: number }[] = [];
    for (const ref of this.refs) {
      const label = ref.label.toLowerCase();
      const parent = (ref.parent_label ?? '').toLowerCase();
      let score = -1;
      if (label.startsWith(q)) score = 0;
      else if (label.includes(q)) score = 1;
      else if (parent.includes(q)) score = 2;
      if (score >= 0) scored.push({ ref, score });
    }
    scored.sort((a, b) => a.score - b.score || a.ref.label.length - b.ref.label.length);
    return scored.slice(0, limit).map(s => s.ref);
  }
}

export const refStore = new RefStore();
