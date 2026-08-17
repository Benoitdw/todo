export interface List {
  id: string;
  title: string;
  pos: number;
}

export interface Item {
  id: string;
  list_id: string;
  text: string;
  checked: boolean;
  pos: number;
}

export interface Config {
  token: string;
}

/** Which space the app is in. Derived from the URL, never persisted. */
export type Mode = 'lists' | 'notes';

/** What the sidebar needs to render a row — satisfied by both List and Note. */
export interface SidebarEntry {
  id: string;
  title: string;
  pos: number;
}

export type IslandKind = 'text' | 'photo' | 'video' | 'audio';

export interface Note {
  id: string;
  title: string;
  pos: number;
}

export interface Island {
  id: string;
  note_id: string;
  kind: IslandKind;
  /** Body when kind is 'text', caption otherwise. */
  text: string;
  pos: number;
  /** Null until a binary has been uploaded for this island. */
  media_path: string | null;
  media_mime: string | null;
  media_size: number | null;
}

export type RefKind = 'note' | 'island' | 'list' | 'item';

/** A link target, or the description of a backlink's source. */
export interface Ref {
  kind: RefKind;
  id: string;
  label: string;
  /** note_id for an island, list_id for an item, null otherwise. */
  parent_id: string | null;
  parent_label: string | null;
}
