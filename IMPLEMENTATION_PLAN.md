# IMPLEMENTATION_PLAN.md

Работать этапами; после каждого — тесты и отчёт.

## Phase 0 — Discovery / ADR
Определить целевую архитектуру NAS; проверить версии/лицензии; сравнить torrent engine по seek/random access; проверить допустимую интеграцию RuTracker/Rutor; решить frontend serving/HLS/auth. Результат: ADR и блокеры.

## Phase 1 — Foundation
Rust project, toolchain pin, config, tracing, Axum health/readiness, error model, Vue scaffold, OpenAPI, Docker skeleton. Критерий: сборка и запуск.

## Phase 2 — Search contracts
Модели, trait, static registry, fake providers, parallel search, timeout, partial results, dedupe, endpoint, tests. Всё работает без интернета.

## Phase 3 — Provider integrations
Feasibility check; RuTracker/Rutor только при допустимом способе; fixtures, parser tests, config enable/disable, diagnostics. Не выдавать фиктивные результаты.

## Phase 4 — Web search UI
Search, result cards, partial errors, provider status, typed client, responsive layout.

## Phase 5 — Streaming PoC
Выбрать engine; разрешённый тестовый torrent; metadata/file selection; seek/random access; buffer; FFmpeg HLS; cancellation/cleanup. Если критерии не выполняются, документировать варианты и согласовать изменение требований.

## Phase 6 — Playback API
Create/status/delete session, limits, FFmpeg lifecycle, playlist/segments, timeout, cleanup, integration tests.

## Phase 7 — Player UI
HLS, buffering/errors, session cleanup, seek/pause/fullscreen.

## Phase 8 — Docker/Synology
Target architecture, volumes/permissions, limits, graceful shutdown, healthchecks, setup/upgrade docs.

## Phase 9 — Release review
Security, tests, OpenAPI, docs, dependency/license review, release notes.

Android TV UI не входит в эти этапы; API обязан оставаться совместимым с будущим native client.
