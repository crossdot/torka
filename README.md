# Movie Streamer — пакет спецификации для агента

Версия: 0.1.0. Статус: стартовая спецификация; перед реализацией нужен Discovery/ADR-этап.

## Использование
1. Передайте архив агенту Oh My Pi.
2. Сначала прочитать `AGENT.md`, `PRODUCT.md`, `ARCHITECTURE.md`, `API.md`, `IMPLEMENTATION_PLAN.md`.
3. Сначала проверить зависимости, лицензии, целевую архитектуру Synology, torrent engine и допустимые способы интеграции RuTracker/Rutor.
4. Не выдумывать HTML-структуру, API или доступность сайтов. Не обходить CAPTCHA, авторизацию или технические ограничения.
5. Использовать собственный или иной явно разрешённый тестовый контент для PoC.

## Файлы
- `PRODUCT.md` — продуктовые цели и границы.
- `AGENT.md` — правила автономного агента.
- `ARCHITECTURE.md` — модули и потоки данных.
- `API.md` — предварительный HTTP API.
- `WEB_UI.md` — веб-интерфейс.
- `CLIENTS.md` — веб, Android TV и другие клиенты.
- `PROVIDERS.md` — RuTracker/Rutor и провайдерная архитектура.
- `STREAMING.md` — torrent, FFmpeg и HLS.
- `DOCKER_SYNOLOGY.md` — Docker/DSM.
- `SECURITY.md` — безопасность.
- `TESTING.md` — тестовая стратегия.
- `IMPLEMENTATION_PLAN.md` — фазы реализации.
- `DECISIONS.md` — исходные решения и открытые вопросы.
- `TASKS.md` — рабочий чек-лист.
- `ENV.example`, `compose.yaml.example` — примеры конфигурации.
- `ADR_TEMPLATE.md`, `GLOSSARY.md`, `examples/` — вспомогательные материалы.
