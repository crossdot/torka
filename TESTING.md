# TESTING.md

## Backend unit
Query normalization; provider registry; dedupe; partial results; timeout; malformed response; magnet validation; error mapping; session transitions; concurrency limit; cancellation/cleanup; config validation.

## Provider tests
Локальные HTML fixtures, отсутствующие поля, изменённая разметка, invalid links, oversized responses, redirects, duplicates. Live smoke tests — только opt-in.

## Streaming integration
Разрешённый test torrent/file: time to first frame, no full pre-download, seek, slow/missing peers, FFmpeg failure, invalid media, stop while buffering, shutdown while playing, cache cleanup, limits.

## Frontend
API client, loading/empty/partial failure/error, results, polling status, cancel/stop, player errors, no secret leakage.

## CI
Backend: `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`.
Frontend: install from lockfile, lint, typecheck, unit tests, production build.

## Definition of Done
Код, тесты, проверки, документация, безопасные ошибки, отсутствие секретов, clean checkout build и documented limitations.
