# DOCKER_SYNOLOGY.md

## Перед сборкой
Уточнить модель NAS, CPU architecture, DSM/Container Manager, RAM, свободный диск, доступность аппаратного ускорения, firewall и исходящие peer-соединения.

## Образ
- multi-stage build Rust + frontend;
- фиксированные версии base images/dependencies;
- FFmpeg с проверенными muxer/demuxer/codecs;
- непривилегированный пользователь, если возможно;
- не использовать `latest` в production;
- не монтировать Docker socket;
- не включать `privileged: true` без документированной необходимости;
- корректная обработка SIGTERM.

## Volumes/network
- `/cache` — ограниченный временный cache;
- `/config` — опциональная конфигурация;
- постоянная БД в MVP не нужна;
- публиковать только web/API port;
- не открывать peer ports без явного решения;
- для удалённого доступа рекомендовать VPN или защищённый reverse proxy с TLS/auth, а не открытый API.

## Compose
- подготовить `compose.yaml` для запуска сервиса

## Release checks
Clean build для целевых архитектур, права volume, memory limits, shutdown при активной сессии, очистка кэша, upgrade/rollback docs.
