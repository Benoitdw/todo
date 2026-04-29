# NAS Sync — Design v2

## Architecture

```
[Tauri app]
  └── SQLite local (toujours actif, source de vérité locale)
        └── Sync engine (background, Rust)
              └── HTTP + Bearer token ──► [Axum NAS] ──► [SQLite NAS]
```

- Toutes les lectures/écritures passent par le SQLite local en premier
- La sync est bidirectionnelle et se fait en arrière-plan
- L'app fonctionne entièrement offline, la sync rattrape quand la connexion revient

## Schéma SQLite — ajouts

Chaque table gagne deux colonnes :

```sql
updated_at  TEXT  -- timestamp ISO8601, mis à jour à chaque write
deleted_at  TEXT  -- soft delete (NULL = vivant)
```

Pas de suppression physique immédiate — on marque `deleted_at`, on propage lors de la sync, puis on purge localement après confirmation.

## Sync engine

Deux déclencheurs :
- Au lancement de l'app
- Quand la connexion revient (polling toutes les 30s sur `GET /health`)

Algorithme (last-write-wins sur `updated_at`) :

```
1. GET /sync?since=<last_sync_at>  → récupère les changements NAS
2. Pour chaque item reçu : si updated_at NAS > local → écrase local
3. Collecter les changements locaux (updated_at > last_sync_at)
4. PUT /sync  → envoie les changements locaux
5. Pour chaque item reçu : si updated_at local > NAS → local gagne
6. Sauvegarder last_sync_at = now
```

Les ajouts distincts (ex : deux personnes ajoutent des items différents offline) ne se marchent jamais dessus — last-write-wins ne s'applique qu'en cas de modification du **même item** par deux appareils.

## Serveur Axum (NAS)

- Middleware Bearer token sur toutes les routes
- `GET /health` — 200 si token valide (utilisé pour tester la connexion)
- `GET /sync?since=<timestamp>` — retourne tous les items modifiés après `since`
- `PUT /sync` — reçoit un batch de changements locaux, applique last-write-wins
- SQLite dans un volume Docker (`/data/todo.db`)

## Sécurité

Token statique configuré via variable d'environnement sur le NAS. Pas de système de login/utilisateur.

## Déploiement Synology

`docker-compose.yml` sur le NAS :

```yaml
services:
  todo:
    image: ghcr.io/ton-user/todo-server:latest
    ports:
      - "8080:8080"
    environment:
      - BEARER_TOKEN=ton-token-secret-ici
    volumes:
      - ./data:/data
    restart: unless-stopped
```

Mise à jour manuelle :

```bash
docker-compose pull && docker-compose up -d
```

## CI/CD — GitHub Actions

Workflow sur push `main` :
- Build image Rust multi-arch (`linux/amd64` + `linux/arm64`)
- Push sur `ghcr.io/ton-user/todo-server:latest`

Build ~10 min sur GitHub, pas en local.

## Flow premier lancement Tauri

Si aucune config détectée au démarrage → écran de setup :

```
┌─────────────────────────────────┐
│  Connexion au serveur           │
│                                 │
│  URL   [http://192.168.1.x:8080]│
│  Token [••••••••••••••••••••]   │
│                                 │
│  [Tester la connexion]          │
│  ✓ Connexion réussie            │
│                                 │
│  [Continuer]                    │
└─────────────────────────────────┘
```

- "Tester" → `GET /health` avec le token
- "Continuer" actif seulement après test réussi
- Config sauvée dans `~/.config/todo/config.toml`
- Modifiable plus tard depuis les Settings

## Hors scope v2

- Watchtower (mises à jour auto du container)
- Résolution de conflits manuelle
- Multi-utilisateurs
