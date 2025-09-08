#!/usr/bin/env python3
import json
import sys

def main():
    # Читаем входные данные от Claude Code
    try:
        input_data = json.load(sys.stdin)
        # Проверяем корректность входных данных
        if not isinstance(input_data, dict):
            print("Error: Invalid input format", file=sys.stderr)
            sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Failed to parse JSON input: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: Failed to read input: {e}", file=sys.stderr)
        sys.exit(1)

    # Создаем корректный ответ для UserPromptSubmit хука
    output = {
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit", 
            "additionalContext": "ЗАРАБОТАЛ - UserPromptSubmit хук успешно вызван и обработал входные данные"
        }
    }

    try:
        print(json.dumps(output))
        sys.exit(0)
    except Exception as e:
        print(f"Error: Failed to output JSON: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()