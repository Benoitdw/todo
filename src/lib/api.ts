import type { List, Item, Config } from './types';

function getToken(): string {
  return localStorage.getItem('api_token') ?? '';
}

async function http<T>(method: string, path: string, body?: unknown): Promise<T> {
  const resp = await fetch(`/api${path}`, {
    method,
    headers: {
      Authorization: `Bearer ${getToken()}`,
      ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!resp.ok) throw new Error(`${resp.status}`);
  const text = await resp.text();
  if (!text) return undefined as T;
  const type = resp.headers.get('Content-Type') ?? '';
  // A non-JSON 200 means the request never reached the API (dev server SPA
  // fallback, misconfigured proxy) — say so instead of dying in JSON.parse.
  if (!type.includes('application/json')) {
    throw new Error(`Réponse non-JSON depuis /api${path} (${type || 'type inconnu'})`);
  }
  return JSON.parse(text);
}

export const api = {
  getLists: () => http<List[]>('GET', '/lists'),

  createList: (title: string, pos: number) =>
    http<List>('POST', '/lists', { title, pos }),

  updateList: (id: string, title: string) =>
    http<void>('PUT', `/lists/${id}`, { title }),

  deleteList: (id: string) => http<void>('DELETE', `/lists/${id}`),

  reorderList: (id: string, pos: number) =>
    http<void>('PUT', `/lists/${id}`, { pos }),

  getItems: (listId: string) => http<Item[]>('GET', `/lists/${listId}/items`),

  createItem: (listId: string, text: string, pos: number) =>
    http<Item>('POST', '/items', { list_id: listId, text, pos }),

  updateItem: (id: string, text: string, checked: boolean) =>
    http<void>('PUT', `/items/${id}`, { text, checked }),

  deleteItem: (id: string) => http<void>('DELETE', `/items/${id}`),

  reorderItem: (id: string, pos: number) =>
    http<void>('PUT', `/items/${id}`, { pos }),

  // Config lives in localStorage — the server is the single source of truth
  // for data, so the only thing we persist locally is the access token.
  getConfig: (): Promise<Config | null> => {
    const token = localStorage.getItem('api_token');
    return Promise.resolve(token ? { token } : null);
  },

  saveConfig: (token: string): Promise<void> => {
    localStorage.setItem('api_token', token);
    return Promise.resolve();
  },

  testConnection: (token: string): Promise<void> =>
    fetch('/health', { headers: { Authorization: `Bearer ${token}` } }).then(r => {
      if (!r.ok) throw new Error('Connexion échouée');
    }),

  connectEvents: (onInvalidate: () => void): (() => void) => {
    let closed = false;
    let current: EventSource | null = null;

    function connect() {
      if (closed) return;
      const token = getToken();
      const es = new EventSource(`/events?token=${encodeURIComponent(token)}`);
      es.onmessage = () => onInvalidate();
      es.onerror = () => { es.close(); if (!closed) setTimeout(connect, 5000); };
      current = es;
    }

    connect();
    return () => { closed = true; current?.close(); };
  },
};
