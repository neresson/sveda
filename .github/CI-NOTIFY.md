# Уведомления о падении CI

## Без настройки (GitHub)

1. **Watch** репозиторий → **Custom** → включите **Actions**.
2. [Notification settings](https://github.com/settings/notifications) → **Actions** → включите письма для failed workflows (или «Only for workflows I have authored»).

Письма и колокольчик в GitHub приходят автоматически, если вы смотрите репо.

## Telegram / Slack (секреты репозитория)

Workflow `CI` содержит job **Notify on failure** (только при `push`, когда упал js/rust/helm).

| Secret | Назначение |
|--------|------------|
| `TELEGRAM_BOT_TOKEN` | Токен @BotFather |
| `TELEGRAM_CHAT_ID` | id чата (личный или группы) |
| `SLACK_WEBHOOK_URL` | Incoming Webhook URL |

Достаточно одной пары (Telegram или Slack). Если секреты не заданы, job завершится без ошибки (шаги пропускаются).

Settings → Secrets and variables → Actions → **New repository secret**.
