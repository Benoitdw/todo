import { invoke } from '@tauri-apps/api/core';
import type { List, Item, Config } from './types';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
  return text ? JSON.parse(text) : (undefined as T);
}

export const api = {
  getLists: () => isTauri
    ? invoke<List[]>('get_lists')
    : http<List[]>('GET', '/lists'),

  createList: (title: string, pos: number) => isTauri
    ? invoke<List>('create_list', { title, pos })
    : http<List>('POST', '/lists', { title, pos }),

  updateList: (id: string, title: string) => isTauri
    ? invoke<void>('update_list', { id, title })
    : http<void>('PUT', `/lists/${id}`, { title }),

  deleteList: (id: string) => isTauri
    ? invoke<void>('delete_list', { id })
    : http<void>('DELETE', `/lists/${id}`),

  reorderList: (id: string, pos: number) => isTauri
    ? invoke<void>('reorder_list', { id, pos })
    : http<void>('PUT', `/lists/${id}`, { pos }),

  getItems: (listId: string) => isTauri
    ? invoke<Item[]>('get_items', { listId })
    : http<Item[]>('GET', `/lists/${listId}/items`),

  createItem: (listId: string, text: string, pos: number) => isTauri
    ? invoke<Item>('create_item', { listId, text, pos })
    : http<Item>('POST', '/items', { list_id: listId, text, pos }),

  updateItem: (id: string, text: string, checked: boolean) => isTauri
    ? invoke<void>('update_item', { id, text, checked })
    : http<void>('PUT', `/items/${id}`, { text, checked }),

  deleteItem: (id: string) => isTauri
    ? invoke<void>('delete_item', { id })
    : http<void>('DELETE', `/items/${id}`),

  reorderItem: (id: string, pos: number) => isTauri
    ? invoke<void>('reorder_item', { id, pos })
    : http<void>('PUT', `/items/${id}`, { pos }),

  // Config — web mode uses localStorage, Tauri uses invoke
  getConfig: () => isTauri
    ? invoke<Config | null>('get_config')
    : Promise.resolve(localStorage.getItem('api_token') ? { server_url: '', token: localStorage.getItem('api_token')! } as Config : null),

  saveConfig: (serverUrl: string, token: string) => isTauri
    ? invoke<void>('save_config', { serverUrl, token })
    : (localStorage.setItem('api_token', token), Promise.resolve()),

  testConnection: (serverUrl: string, token: string) => isTauri
    ? invoke<void>('test_connection', { serverUrl, token })
    : fetch('/health', { headers: { Authorization: `Bearer ${token}` } }).then(r => { if (!r.ok) throw new Error('Connexion échouée'); }),

  triggerSync: () => isTauri
    ? invoke<void>('trigger_sync')
    : Promise.resolve(),

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
