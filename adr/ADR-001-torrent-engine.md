# ADR-001: Torrent engine — встроенная библиотека librqbit

- Status: Proposed
- Date: 2026-10-29
- Owners: ведущий инженер (Discovery)

## Context

Нужен torrent engine, который в одном Rust-процессе обеспечивает: получение metadata
(magnet/.torrent) без загрузки контента, перечисление и выбор отдельного файла,
приоритизацию выбранного файла, потоковое чтение по смещению (random access / seek)
без полной загрузки, отмену операций, ограниченный cache и multi-arch для Synology.
Лицензия должна быть совместимой с нашим кодом (не GPL-заражающей).

## Options

1. **librqbit 9.0.1** (embedded library). Apache-2.0, активен.
2. **rqbit HTTP API** (sidecar-процесс того же проекта).
3. `cratetorrent 0.1.0` — нет read/seek API, нет DHT/magnet, Linux-only, заморожен с 2021.
4. `torrent 0.1.10` (mogeko/torrent-rs) — нет выбора файла, нет read API, 3★/291 dl.
5. `bip_metainfo 0.12.0` — только парсер metainfo, релиз 2017.
6. Отдельный не-Rust движок (libtorrent/transmission) — не нужен: librqbit покрывает требования.

## Decision

**librqbit 9.0.1, встроенная библиотека** (Apache-2.0). Fallback при несовместимости API —
rqbit HTTP API как sidecar (тот же crate, без нового риска зависимостей). Остальные отклонены.

Подтверждено по исходникам и документации v9.0.1 (не предположения):

- `Session::add_torrent` принимает `http:`/`https:`/`magnet:`; magnet резолвится через
  DHT/trackers/`ut_metadata` (BEP 9).
- Metadata без контента: `AddTorrentOptions { list_only: true }` → `ListOnlyResponse`
  (список файлов + байты `.torrent`).
- Выбор файла: `only_files` (индексы) / `only_files_regex`; runtime `update_only_files`.
- Random access: `ManagedTorrent::stream(file_id) -> FileStream`, где
  `FileStream: tokio::io::AsyncRead + AsyncSeek` (приоритизация по позиции чтения, окно 32 MiB).
- Отмена: `SessionOptions.cancellation_token` + `Session::stop()`.
- Multi-arch: CI собирает linux/amd64, linux/arm64, linux/arm/v7.

## Consequences

- ⚠️ `FileStream` **не реэкспортирован публично** (модуль `torrent_state` приватный):
  нельзя писать `-> FileStream` или хранить его в поле. Хранить как
  `Box<dyn AsyncRead + AsyncSeek + Send + Unpin>`. Реальный риск связанности с API.
- ⚠️ ~8 одновременных stream'ов по умолчанию (`runtime_worker_threads`, каждый `FileStream`
  держит semaphore permit) — поднять под параллелизм HLS-сегментов.
- ⚠️ Нет вытеснения с диска «из коробки»: InMemory-хранилище жёстко 128 MiB (feature-gated),
  LRU-middleware помечен "untested". Ограниченный временный cache — собственный слой
  (директория сессии + TTL + лимит).
- ⚠️ `default-tls` тянет OpenSSL; для лёгкой сборки `default-features=false, features=["rust-tls"]`.
- `sqlx` optional (postgres) — БД не обязательна.

## Validation

- https://crates.io/crates/librqbit (9.0.1, license=Apache-2.0)
- https://docs.rs/librqbit/9.0.1/librqbit/
- https://github.com/ikatson/rqbit — `torrent_state/streaming.rs:167` (AsyncRead),
  `:246` (AsyncSeek), `:29` (`PER_STREAM_BUF_DEFAULT` 32 MiB); HTTP Range handler
  `http_api/handlers/streaming.rs`.
