# Rust-Meilisearch

## What

1. **Import:** Import data from a JSON file into a PostgreSQL database.
2. **Index:** Index data from the PostgreSQL database to a Meilisearch database.
3. **View:** View an example interface.

## How

1. Import data: `(cd importer && cargo run)`
2. Index data: `(cd indexer && cargo run)`
3. View example: `(cd viewer && trunk serve)`

## Etc

- **init.sql:** PostgreSQL initialization query file.
- **config.toml:** Meilisearch configuration file.
