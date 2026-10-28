# AGENT.md — инструкции агенту

## Обязательный этап Discovery
До написания существенного кода:
1. Изучить весь пакет.
2. Проверить актуальные официальные документы Rust/Axum/Tokio/Serde/Vue/FFmpeg и выбранного torrent engine.
3. Уточнить архитектуру целевой Synology или отметить это как блокер.
4. Подготовить ADR по torrent engine, streaming/seek, RuTracker/Rutor, Docker multi-arch, auth и раздаче frontend.
5. Не выдумывать доступность, API, HTML-селекторы или совместимость.
6. Выдать краткий отчёт: подтверждённое, неизвестное, риски, вопросы.

## Правила реализации
- Rust stable; зафиксировать toolchain.
- Axum, Tokio, Serde; tracing для логов.
- Не добавлять БД, Redis, очереди или микросервисы без ADR.
- Провайдеры — trait + статический registry; никаких динамических Rust-плагинов в MVP.
- Внешние провайдеры имеют timeout, ограничение параллелизма и частичную обработку ошибок.
- Внешние данные недоверенные.
- FFmpeg запускать отдельным процессом с фиксированными аргументами, не через shell-конкатенацию.
- Контролировать stdout/stderr, timeout, cancellation, wait/reap и очистку.
- Не логировать cookies, токены, полный magnet URI или секретные URL.
- Любой cache имеет TTL, лимит размера и безопасную очистку.
- Не обходить технические ограничения сайтов.

## RuTracker/Rutor
- Проверить текущую доступность и допустимые способы взаимодействия.
- Не обходить CAPTCHA, auth или антибот-защиту.
- Не хранить credentials без согласованного безопасного механизма.
- Если допустимая интеграция невозможна, оставить provider disabled/unavailable с диагностикой; не выдавать фиктивные результаты.
- Парсеры тестировать на локальных фикстурах.

## Проверки
Backend: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`.
Frontend: lint, typecheck, unit tests, production build.
Интеграционные тесты — на собственном или явно разрешённом тестовом материале.
Обновлять OpenAPI и документацию при изменении контрактов.

## Порядок
0 Discovery/ADR; 1 skeleton; 2 search contract; 3 providers; 4 web search UI; 5 streaming PoC; 6 playback API; 7 player; 8 Docker/DSM; 9 security/release review.

## Остановиться и спросить пользователя, если
Нужны credentials; требуется обход ограничений; решение зависит от прав/лицензий; выбранный engine не поддерживает seek; целевая архитектура не поддерживается; меняется API или product scope.
