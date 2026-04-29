import { invoke } from '@tauri-apps/api/core';
import type { List, Item, Config } from './types';

export const api = {
  getLists: () => invoke<List[]>('get_lists'),
  createList: (title: string, pos: number) => invoke<List>('create_list', { title, pos }),
  updateList: (id: string, title: string) => invoke<void>('update_list', { id, title }),
  deleteList: (id: string) => invoke<void>('delete_list', { id }),
  reorderList: (id: string, pos: number) => invoke<void>('reorder_list', { id, pos }),

  getItems: (listId: string) => invoke<Item[]>('get_items', { listId }),
  createItem: (listId: string, text: string, pos: number) => invoke<Item>('create_item', { listId, text, pos }),
  updateItem: (id: string, text: string, checked: boolean) => invoke<void>('update_item', { id, text, checked }),
  deleteItem: (id: string) => invoke<void>('delete_item', { id }),
  reorderItem: (id: string, pos: number) => invoke<void>('reorder_item', { id, pos }),

  getConfig: () => invoke<Config | null>('get_config'),
  saveConfig: (serverUrl: string, token: string) => invoke<void>('save_config', { serverUrl, token }),
  testConnection: (serverUrl: string, token: string) => invoke<void>('test_connection', { serverUrl, token }),
  triggerSync: () => invoke<void>('trigger_sync'),
};
