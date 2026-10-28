# DECISIONS.md

## Исходные решения
1. Rust stable + Axum + Tokio + Serde.
2. Vue 3 + TypeScript + Vite.
3. Docker на Synology DSM.
4. Модульный монолит.
5. Нет обязательной БД/Redis в MVP; sessions in-memory.
6. Providers через trait и static registry.
7. Первые кандидаты: RuTracker, Rutor.
8. FFmpeg как отдельный процесс.
9. HLS — основной кандидат для web.
10. Android TV отдельно, API клиент-независимый.

## Требуют ADR
- ADR-001 Torrent engine: Rust integration, seek, cache, license, multi-arch.
- ADR-002 Streaming: temp cache vs seekable adapter vs remux/range.
- ADR-003 RuTracker/Rutor: доступность и допустимый способ.
- ADR-004 Frontend serving: backend static vs reverse proxy.
- ADR-005 Auth: token provisioning/rotation, закрытое поведение по умолчанию.
- ADR-006 Synology: модель, CPU, DSM, RAM.
- ADR-007 Metadata: нужен ли внешний metadata catalog в MVP.

Не принимать молча решения, которые меняют scope, обходят ограничения, добавляют обязательную инфраструктуру или ломают API.
