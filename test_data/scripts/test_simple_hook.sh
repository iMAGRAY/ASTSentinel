#!/bin/bash
echo "TEST HOOK EXECUTED" >> /c/Users/1/.claude/hook-test.log
echo '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"SIMPLE TEST CONTEXT"}}'