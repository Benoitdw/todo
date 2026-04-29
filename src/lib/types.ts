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
  server_url: string;
  token: string;
}
