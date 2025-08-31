# XAI Grok Code Fast 1 — техдокументация (актуально на 30.08.2025)

## 1) Назначение
- Индекс: модель для агентного кодинга (pair-programmer, tool calling, IDE/CLI-агенты).
- Цель: максимальная скорость отклика при достаточной точности для практических задач разработки.
- Использование: генерация патчей, навигация по проекту, вызовы инструментов (lint/test/build), структурированные ответы.

## 2) Идентификатор и базовые свойства
- `model`: `grok-code-fast-1`
- Поддержка длинного контекста (под большие диффы/журналы инструментов).
- Режимы вывода: обычный текст, стриминг, структурированный JSON.
- Поддержка рассуждений в стриме (reasoning-дельты) — см. §5.2.
- Оптимально для коротких итераций и многошаговых агентных сценариев.

## 3) Совместимость и эндпоинты API
- Базовый хост: `https://api.x.ai`
- OpenAI-совместимый маршрут: `POST /v1/chat/completions`
- Anthropic-совместимый маршрут: `POST /v1/messages`
- Дополнительно: `/v1/models` (перечень моделей), `/v1/tokenize-text` (планирование затрат/контекста).
- Региональные хосты поддерживаются (для требований суверенитета/латентности).

## 4) Быстрый старт

### 4.1 cURL (OpenAI-совместимый)
```bash
curl https://api.x.ai/v1/chat/completions   -H "Authorization: Bearer $XAI_API_KEY"   -H "Content-Type: application/json"   -d '{
    "model": "grok-code-fast-1",
    "messages": [
      {"role": "system", "content": "Ты — помощник по коду. Отвечай кратко, давай патчи."},
      {"role": "user", "content": "Оптимизируй функцию сортировки по памяти."}
    ],
    "temperature": 0.2,
    "stream": true
  }'
```

### 4.2 Node.js (OpenAI SDK; поменять baseURL)
```ts
import OpenAI from "openai";

const client = new OpenAI({
  apiKey: process.env.XAI_API_KEY,
  baseURL: "https://api.x.ai/v1"
});

const resp = await client.chat.completions.create({
  model: "grok-code-fast-1",
  messages: [
    { role: "system", content: "Коротко. Возвращай unified-diff без преамбулы." },
    { role: "user", content: "Обнови функцию parse() под новый формат." }
  ],
  stream: true
});

// В стриме агрегируйте resp.choices[0].delta.content
// и resp.choices[0].delta.reasoning_content (если присутствует).
```

### 4.3 Anthropic-совместимый вызов
```bash
curl https://api.x.ai/v1/messages   -H "Authorization: Bearer $XAI_API_KEY"   -H "Content-Type: application/json"   -d '{
    "model":"grok-code-fast-1",
    "messages":[{"role":"user","content":"Сгенерируй shell-скрипт deploy.sh с проверками."}],
    "stream": true
  }'
```

## 5) Потоковый режим и reasoning-дельты
- Рекомендуется всегда включать `stream: true` для интерактивности в IDE/агенте.
- В потоке могут приходить дельты обычного вывода и dельты рассуждений (reasoning).
- Показывать reasoning пользователю — по политике продукта (можно скрывать, логировать, обрезать PII).
- Агрегируйте дельты в том порядке, в котором они приходят.

## 6) Вызов инструментов (tool calling)
- Формат — совместимый с function calling.
- Описывайте инструменты JSON-схемой в поле `tools`.
- Управление:
  - `tool_choice: "auto"` — модель сама решает, вызывать ли инструмент.
  - `tool_choice: "required"` — принудительный вызов (возможна «догадка» по аргументам).
  - `tool_choice: {"type":"function","function":{"name":"..."}}` — форсировать конкретную функцию.
  - Параллельные вызовы: включены по умолчанию; можно ограничить флагом/параметром интеграции.

**Скелет запроса:**
```json
{
  "model": "grok-code-fast-1",
  "messages": [{"role":"user","content":"Сгенерируй миграции БД"}],
  "tools": [{
    "type": "function",
    "function": {
      "name": "apply_migration",
      "description": "Apply SQL migration safely",
      "parameters": {
        "type":"object",
        "properties":{"sql":{"type":"string"}},
        "required":["sql"]
      }
    }
  }],
  "tool_choice": "auto"
}
```

## 7) Структурированные ответы
- Поддерживается строгий JSON по заданной схеме.
- Типы: `string`, `number`, `integer`, `float`, `object`, `array`, `boolean`, `enum`, `anyOf`.
- Ограничения наподобие `minLength/maxLength`, `minItems/maxItems`, `allOf` — могут игнорироваться.
- Рекомендация: валидируйте объект на стороне клиента (Zod/Pydantic/JSON Schema).

## 8) Паттерны промптинга (для кодинга)
- Давайте точный контекст: пути файлов, версии зависимостей, цель правки.
- Формулируйте критерии готовности (компилируется, тесты зелёные, стилевые правила).
- Просите **unified-diff** для прямого применения патча.
- Разбивайте задачи на короткие шаги для лучшего скоринга и латентности.
- Для глубоких расследований комбинируйте с более «мыслящей» моделью и/или внешним поиском/инструментами.

## 9) Производительность, кеш, лимиты
- Кеш входа снижает стоимость и ускоряет серии шагов — не меняйте общий префикс без надобности.
- Биллинг по токенам (prompt/completion; при наличии — reasoning).
- Обрабатывайте ошибки `429` (бекофф, джиттер, идемпотентные повторы).
- Следите за `usage` в ответе для планирования бюджета.

## 10) Интеграция с IDE/агентами
- Поддерживается в популярных агентных IDE и плагинообразных средах.
- BYOK: используйте собственный xAI API key и `baseURL` `https://api.x.ai/v1`.
- Для мульти-маршрутизации возможно применение прокси-провайдеров.

## 11) Диагностика и устойчивость
- Логируйте: вход, выход, tool_calls, метаданные `usage`, коды ошибок.
- Стратегии повторов: экспоненциальный бекофф с пределом попыток.
- Длинные задачи — используйте отложенные завершения/асинхронные шаблоны.
- Мониторинг инцидентов — по статус-странице провайдера (если используется).

## 12) Шаблоны использования

**A. Генерация патча (diff)**
1. Дайте путь и минимальный фрагмент.
2. Попросите unified-diff без преамбулы.
3. Примените через инструмент `apply_patch`.

**B. Агентная диагностика (многошаговая)**
- Цель в system-промпте (ветка `temp/*`, правила безопасности).
- Инструменты: `read_files`, `ripgrep`, `tests.run`, `apply_patch`.
- Включите параллельные tool calls; стримьте reasoning в UI (если нужно).

**C. Структурированный ответ**
- Задайте JSON-схему (пример: `{ "function": "string", "files": "string[]", "risk": "string" }`).
- Включите строгий вывод; валидируйте на клиенте.

## 13) Ограничения
- Нет встроенного Live Search: используйте отдельный инструмент поиска.
- Reasoning-след доступен в стриме; хранение/отображение — по политике приватности.
- Для очень глубокого анализа кода лучше комбинировать с более «тяжёлой» моделью.

---

### Приложение A — SDK-скетчи

**Node (OpenAI SDK)**
```ts
import OpenAI from "openai";
const client = new OpenAI({ apiKey: process.env.XAI_API_KEY, baseURL: "https://api.x.ai/v1" });

const stream = await client.chat.completions.create({
  model: "grok-code-fast-1",
  messages: [
    { role: "system", content: "Коротко. Дай готовый diff." },
    { role: "user", content: "Исправь баг в validateEmail()" }
  ],
  stream: true
});

// Обрабатывайте delta.content и delta.reasoning_content
```

**cURL — строгий JSON-ответ**
```bash
curl https://api.x.ai/v1/chat/completions   -H "Authorization: Bearer $XAI_API_KEY"   -H "Content-Type: application/json"   -d '{
    "model": "grok-code-fast-1",
    "messages": [{"role":"user","content":"Верни JSON с полями {function, files, risk}."}],
    "response_format": {
      "type": "json_schema",
      "json_schema": {
        "name": "ActionPlan",
        "schema": {
          "type": "object",
          "properties": {
            "function": {"type": "string"},
            "files": { "type": "array", "items": {"type": "string"} },
            "risk": {"type": "string"}
          },
          "required": ["function", "files", "risk"],
          "additionalProperties": false
        }
      }
    }
  }'
```
