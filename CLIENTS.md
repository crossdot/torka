# CLIENTS.md — web и будущий Android TV

## Сейчас
Реализовать только web-клиент, но серверный API сделать независимым от Vue и браузера:
- `/api/v1`, OpenAPI, стабильные JSON ошибки;
- Bearer token;
- playback session ID/status;
- HLS playlist/segments;
- cancel/stop;
- тайм-ауты и статусы;
- не требовать cookies, DOM или browser storage;
- настроить public/base URL для reverse proxy.

## Android TV — отдельный будущий проект
Не реализовывать приложение в MVP. Отдельная спецификация позже должна определить:
- Kotlin и актуальный TV UI toolkit;
- навигацию D-pad/Select/Back и focus states;
- поиск, результаты, выбор источника/файла;
- Media3/ExoPlayer и проверку HLS/кодеков на устройствах;
- buffering, retry и ошибки сети;
- управление сессией при Back/stop/выходе;
- настройку адреса сервера и безопасный ввод токена;
- тесты на эмуляторе и реальном устройстве.

Не принимать API-решений, которые привязывают клиент к browser cookies. Проверить HLS MIME types, CORS для web, caching headers и корректность URL сегментов.
