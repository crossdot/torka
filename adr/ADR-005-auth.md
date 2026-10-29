# ADR-005: Аутентификация

- Status: Proposed
- Date: 2026-10-29
- Owners: ведущий инженер (Discovery)

## Context

Playback-сессии, HLS-артефакты и управление источниками должны быть защищены. MVP без БД
и пользовательских профилей (PRODUCT.md). Нужен минимальный механизм, «закрытый по умолчанию».

## Options

1. Один статический Bearer token из env (`APP_API_TOKEN`); API (кроме `/health`, `/ready`)
   требует `Authorization: Bearer <token>`. Без cookie, без сессий входа.
2. Пользовательские аккаунты/пароли — выходит за рамки MVP (не-цель).
3. Открытый API (без auth) + сетевой периметр — отклонено (SECURITY.md: сеть не единственная защита).

## Decision

**Модель 1.** Один секрет в env, обязателен для всех `/api/*` и playback-артефактов;
`/health`/`/ready` — публичные (liveness/readiness без секретов). Отсутствие/`change-me`
значения — сервис стартует, но помечает auth небезопасной (или отказывает в playback).
`session_id` непредсказуем; знание `session_id` не отменяет проверку токена (SECURITY.md).

## Consequences

- Токен в env, вне git/образа; redaction в логах.
- Ротация = перезапуск с новым env (приемлемо для MVP).
- Reverse proxy (если есть) должен пробрасывать `Authorization` и не кешировать playback-ответы.
- Открытый вопрос на согласование: поведение при пустом/дефолтном токене (warn vs fail-closed).

## Validation

- Требование согласуется с API.md («Bearer token; не выставлять открытый API») и SECURITY.md.
