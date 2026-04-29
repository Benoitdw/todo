# TODO app — À faire plus tard

## Phase 2 : NAS sync

- Écrire un petit serveur **Rust + Axum** qui tourne sur le NAS
  - Mêmes endpoints que les commandes Tauri actuelles, mais en REST JSON
  - SQLite côté NAS comme source de vérité
  - Authentification basique (token statique suffit pour usage perso)
- Abstraire le data access en Rust derrière un trait `Repository` :
  ```rust
  trait Repository: Send + Sync {
      fn get_lists(&self) -> Result<Vec<List>>;
      fn create_item(&self, ...) -> Result<Item>;
      // ...
  }
  ```
  - `SqliteRepository` = implémentation actuelle (locale)
  - `ApiRepository` = appels HTTP vers le NAS
- Configurer via `tauri.conf.json` ou fichier de config lequel utiliser

## Phase 3 : Android

- App **Kotlin + Jetpack Compose** qui parle à l'API NAS
- UI minimaliste identique au sketch
- Même API REST → pas de code partagé à gérer, juste un 2e client

## Features UI / UX

- [ ] Recherche dans les items (pas seulement dans les listes)
- [ ] Date d'échéance sur les items (champ `due_date TEXT` à ajouter en migration SQLite)
- [ ] Raccourcis clavier : `Ctrl+N` nouvelle liste, `N` nouvel item, `Del` supprimer
- [ ] Thème sombre
- [ ] Sous-tâches (table `items` avec colonne `parent_id`)
- [ ] Réorganiser les listes par drag & drop déjà fait — ajouter drag entre listes (déplacer un item d'une liste à l'autre)

## Technique / infra

- [ ] Migration SQLite propre (ajouter une table `migrations` et versionner le schéma)
- [ ] Icônes réelles (remplacer les PNG noirs générés)
- [ ] Build release : activer `bundle.active: true` dans tauri.conf.json + fournir vraies icônes
- [ ] `.env` ou settings UI pour l'URL du NAS quand Phase 2
