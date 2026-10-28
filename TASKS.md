# TASKS.md

## Discovery
- [ ] Прочитать пакет.
- [ ] Уточнить Synology target.
- [ ] Сравнить torrent engine и лицензии.
- [ ] Проверить FFmpeg/HLS/seek PoC.
- [ ] Проверить допустимые интеграции RuTracker/Rutor.
- [ ] Подготовить ADR и вопросы пользователю.

## Foundation
- [ ] Rust/toolchain.
- [ ] Config validation.
- [ ] tracing/errors.
- [ ] health/readiness.
- [ ] Vue scaffold.
- [ ] OpenAPI.
- [ ] Docker build.

## Search
- [ ] Provider trait/registry.
- [ ] Fake provider.
- [ ] Parallel SearchService.
- [ ] Timeout/partial failures/dedupe.
- [ ] API and tests.
- [ ] Provider status.

## Integrations
- [ ] Feasibility report RuTracker.
- [ ] Parser + fixtures if appropriate.
- [ ] Feasibility report Rutor.
- [ ] Parser + fixtures if appropriate.
- [ ] Config enable/disable.
- [ ] Rate limits.

## Streaming
- [ ] Torrent engine ADR.
- [ ] Test asset/torrent.
- [ ] Metadata/file selection.
- [ ] Seek/random access.
- [ ] FFmpeg HLS.
- [ ] Session lifecycle.
- [ ] Cancellation/cleanup.
- [ ] Resource limits.
- [ ] Integration tests.

## Client and release
- [ ] Search UI.
- [ ] Partial provider errors.
- [ ] Player and stop session.
- [ ] Lint/typecheck/tests/build.
- [ ] Docker target architecture.
- [ ] Shutdown and cleanup.
- [ ] Security review.
- [ ] Docs and release checklist.
