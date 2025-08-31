{
  "document":"Claude Code Hooks — Machine Guide",
  "version":"2025-08-28+v3.2",
  "normative":true,
  "purpose":"Deterministic, machine-checkable contract for implementing, validating, and operating Claude Code hooks with clear gating semantics.",
  "audience":["ai_agent","hook_scripts","ci_pipelines","security_review"],
  "encoding":{"charset":"UTF-8","newline":"\n","bom":false,"surrogates_forbidden":true},
  "references":{"anthropic_docs":["https://docs.anthropic.com/en/docs/claude-code/hooks","https://docs.anthropic.com/en/docs/claude-code/hooks-guide"]},
  "global_invariants":[
    "Emit exactly ONE JSON object on stdout (no banners/markdown).",
    "Use double quotes only; no comments; no trailing commas.",
    "Top-level allowed keys by event only; unknown keys are rejected.",
    "reason<=300 chars; hookSpecificOutput.additionalContext<=4000 chars.",
    "Stdout is NOT model-visible at exit 0, EXCEPT for UserPromptSubmit and SessionStart where stdout is added to context.",
    "Log to stderr only; never mix logs into stdout JSON.",
    "Prefer JSON contracts; use exit code 2 only for hard block/error.",
    "Sort JSON keys for deterministic output.",
    "additionalContext MUST be either 'OK' or JSON SoftFeedback encoded as a string for PostToolUse; for UserPromptSubmit/SessionStart it MUST be plain string (no markdown fences/backticks).",
    "If both JSON and exit code semantics are present, JSON output takes precedence; exit codes still apply for gating."
  ],
  "events":{
    "PreToolUse":{
      "when":"after tool args are created and before the call executes",
      "matcher_applicable":true,
      "allowed_top_keys":["hookSpecificOutput"],
      "visibility_routing":{
        "allow":{"to_user":true,"to_model":false},
        "ask":{"to_user":true,"to_model":false},
        "deny":{"to_user":false,"to_model":true}
      },
      "contracts":{
        "allow":{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}},
        "ask":{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"<<=300 chars>"}},
        "deny":{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"<<=300 chars>"}}
      },
      "deprecated":{"decision_reason":"Top-level decision='approve'|'block' and reason are deprecated in favor of hookSpecificOutput.permissionDecision."}
    },
    "PostToolUse":{
      "when":"immediately after a tool completes successfully",
      "matcher_applicable":true,
      "allowed_top_keys":["decision","reason","hookSpecificOutput"],
      "contracts":{
        "block":{"decision":"block","reason":"<<=300 chars>","hookSpecificOutput":{"hookEventName":"PostToolUse"}},
        "soft":{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"<<=4000 chars | 'OK' | JSON(SoftFeedback) string>"}}
      }
    },
    "UserPromptSubmit":{
      "when":"on user prompt submission, before processing",
      "matcher_applicable":false,
      "allowed_top_keys":["decision","reason","hookSpecificOutput"],
      "contracts":{
        "block":{"decision":"block","reason":"<<=300 chars>"},
        "add_context":{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"<<=4000 chars plain string (no code fences)>"}}
      },
      "stdout_at_exit0_is_context":true
    },
    "Stop":{
      "when":"after main agent turn",
      "matcher_applicable":false,
      "allowed_top_keys":["decision","reason","hookSpecificOutput"],
      "contracts":{
        "block":{"decision":"block","reason":"Explain what to fix before stopping","hookSpecificOutput":{"hookEventName":"Stop"}}
      }
    },
    "SubagentStop":{
      "when":"after subagent (Task) turn",
      "matcher_applicable":false,
      "allowed_top_keys":["decision","reason","hookSpecificOutput"],
      "contracts":{
        "block":{"decision":"block","reason":"Explain what to fix before stopping","hookSpecificOutput":{"hookEventName":"SubagentStop"}}
      }
    },
    "Notification":{"when":"on Claude Code notification (permission needed; idle etc.)","matcher_applicable":false,"allowed_top_keys":[],"contracts":{}},
    "PreCompact":{"when":"before compact","matcher_applicable":true,"allowed_top_keys":[],"contracts":{}},
    "SessionStart":{
      "when":"when session starts or resumes",
      "matcher_applicable":true,
      "allowed_top_keys":["hookSpecificOutput"],
      "contracts":{
        "add_context":{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"<<=4000 chars plain string (no code fences)>"}}
      },
      "stdout_at_exit0_is_context":true
    }
  },
  "json_schemas":{
    "$schema":"https://json-schema.org/draft/2020-12/schema",
    "$defs":{
      "SoftFeedback":{
        "type":"object",
        "required":["summary"],
        "additionalProperties":false,
        "properties":{
          "summary":{"type":"string","maxLength":280},
          "files":{
            "type":"array","maxItems":25,
            "items":{
              "type":"object","additionalProperties":false,
              "required":["path","issues"],
              "properties":{
                "path":{"type":"string"},
                "issues":{
                  "type":"array","maxItems":3,
                  "items":{
                    "type":"object","additionalProperties":false,
                    "required":["sev","msg","loc"],
                    "properties":{
                      "sev":{"enum":["info","warn","error"]},
                      "msg":{"type":"string","maxLength":200},
                      "loc":{"type":"object","required":["line"],"additionalProperties":false,"properties":{"line":{"type":["integer","null"]}}}
                    }
                  }
                }
              }
            }
          }
        }
      }
    },
    "PreToolUseAllow":{"type":"object","additionalProperties":false,"required":["hookSpecificOutput"],"properties":{"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName","permissionDecision"],"properties":{"hookEventName":{"const":"PreToolUse"},"permissionDecision":{"const":"allow"}}}}},
    "PreToolUseAsk":{"type":"object","additionalProperties":false,"required":["hookSpecificOutput"],"properties":{"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName","permissionDecision","permissionDecisionReason"],"properties":{"hookEventName":{"const":"PreToolUse"},"permissionDecision":{"const":"ask"},"permissionDecisionReason":{"type":"string","maxLength":300}}}}},
    "PreToolUseDeny":{"type":"object","additionalProperties":false,"required":["hookSpecificOutput"],"properties":{"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName","permissionDecision","permissionDecisionReason"],"properties":{"hookEventName":{"const":"PreToolUse"},"permissionDecision":{"const":"deny"},"permissionDecisionReason":{"type":"string","maxLength":300}}}}},
    "PostToolUseBlock":{"type":"object","additionalProperties":false,"required":["decision","reason","hookSpecificOutput"],"properties":{"decision":{"const":"block"},"reason":{"type":"string","maxLength":300},"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName"],"properties":{"hookEventName":{"const":"PostToolUse"},"additionalContext":{"type":"string"}}}}},
    "PostToolUseSoft":{"type":"object","additionalProperties":false,"required":["hookSpecificOutput"],"properties":{"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName","additionalContext"],"properties":{"hookEventName":{"const":"PostToolUse"},"additionalContext":{"anyOf":[{"const":"OK"},{"type":"string","contentMediaType":"application/json","contentSchema":{"$ref":"#/$defs/SoftFeedback"}}],"maxLength":4000,"not":{"pattern":"```"}}}}}},
    "UserPromptSubmitBlock":{"type":"object","additionalProperties":false,"required":["decision","reason"],"properties":{"decision":{"const":"block"},"reason":{"type":"string","maxLength":300}}},
    "UserPromptSubmitAddContext":{"type":"object","additionalProperties":false,"required":["hookSpecificOutput"],"properties":{"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName","additionalContext"],"properties":{"hookEventName":{"const":"UserPromptSubmit"},"additionalContext":{"type":"string","maxLength":4000,"not":{"pattern":"```"}}}}}},
    "SessionStartAddContext":{"type":"object","additionalProperties":false,"required":["hookSpecificOutput"],"properties":{"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName","additionalContext"],"properties":{"hookEventName":{"const":"SessionStart"},"additionalContext":{"type":"string","maxLength":4000,"not":{"pattern":"```"}}}}}},
    "StopBlock":{"type":"object","additionalProperties":false,"required":["decision","reason","hookSpecificOutput"],"properties":{"decision":{"const":"block"},"reason":{"type":"string","maxLength":300},"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName"],"properties":{"hookEventName":{"const":"Stop"}}}}},
    "SubagentStopBlock":{"type":"object","additionalProperties":false,"required":["decision","reason","hookSpecificOutput"],"properties":{"decision":{"const":"block"},"reason":{"type":"string","maxLength":300},"hookSpecificOutput":{"type":"object","additionalProperties":false,"required":["hookEventName"],"properties":{"hookEventName":{"const":"SubagentStop"}}}}}
  },
  "implementation_snippets":{
    "python_safe_dump":"def safe_dump(o):\n  import json, sys\n  s = json.dumps(o, ensure_ascii=False, separators=(\",\",\":\"), sort_keys=True)\n  try:\n    s.encode(\"utf-8\")\n  except UnicodeEncodeError:\n    print(\"Invalid UTF-8 in payload\", file=sys.stderr)\n    raise SystemExit(2)\n  if '\"additionalContext\"' in s and \"```\" in s:\n    print(\"Backticks/markdown fences forbidden in additionalContext\", file=sys.stderr)\n    raise SystemExit(2)\n  return s",
    "pretooluse_allow":"{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"allow\"}}",
    "pretooluse_ask":"{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"ask\",\"permissionDecisionReason\":\"<why>\"}}",
    "pretooluse_deny":"{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"<why>\"}}",
    "posttooluse_block":"{\"decision\":\"block\",\"reason\":\"<why>\",\"hookSpecificOutput\":{\"hookEventName\":\"PostToolUse\"}}",
    "posttooluse_soft_ok":"{\"hookSpecificOutput\":{\"hookEventName\":\"PostToolUse\",\"additionalContext\":\"OK\"}}",
    "posttooluse_soft_payload":"{\"hookSpecificOutput\":{\"hookEventName\":\"PostToolUse\",\"additionalContext\":\"{\\\"summary\\\":\\\"minor lint\\\",\\\"files\\\":[{\\\"path\\\":\\\"app.ts\\\",\\\"issues\\\":[{\\\"sev\\\":\\\"warn\\\",\\\"msg\\\":\\\"Remove dead code\\\",\\\"loc\\\":{\\\"line\\\":42}}]}]}\"}}",
    "userprompt_block":"{\"decision\":\"block\",\"reason\":\"Sensitive content detected\"}",
    "userprompt_add_context":"{\"hookSpecificOutput\":{\"hookEventName\":\"UserPromptSubmit\",\"additionalContext\":\"Context injected\"}}",
    "sessionstart_add_context":"{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"Bootstrapped context\"}}",
    "stop_block":"{\"decision\":\"block\",\"reason\":\"Fix remaining issues then continue.\",\"hookSpecificOutput\":{\"hookEventName\":\"Stop\"}}"
  },
  "merge_rules":{
    "PreToolUse":"If multiple outputs, most restrictive wins: deny > ask > allow. Concatenate non-empty permissionDecisionReason with '; ' then truncate to 300 chars.",
    "PostToolUse":"If any hook emits decision=block, final decision is block with first reason. For soft feedback, join additionalContext payloads with '\\n---\\n' then truncate to 4000 chars.",
    "UserPromptSubmit/SessionStart additionalContext":"Join with '\\n---\\n' then truncate to 4000 chars."
  },
  "setup":{
    "files":[".claude/settings.json",".claude/settings.local.json","/hooks/*"],
    "checklist":[
      "Define hooks in .claude/settings.json; project-specific scripts live under .claude/hooks/.",
      "Use matcher '*' to target all tools or omit matcher for all.",
      "PreToolUse only gates; PostToolUse always returns feedback (soft or block).",
      "UserPromptSubmit/SessionStart: stdout at exit 0 is injected into context; prefer JSON for deterministic control.",
      "One JSON object on stdout; logs only to stderr.",
      "Validate lengths; if exceeding, put full payload into artifact and reference it (see length_and_chunking).",
      "Validate schemas in CI before enabling hooks."
    ],
    "settings_json_example":{
      "hooks":{
        "PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"$CLAUDE_PROJECT_DIR/.claude/hooks/validate-bash.py"}]}],
        "PostToolUse":[{"matcher":"Write|Edit","hooks":[{"type":"command","command":"$CLAUDE_PROJECT_DIR/.claude/hooks/check-style.sh"}]}],
        "UserPromptSubmit":[{"hooks":[{"type":"command","command":"$CLAUDE_PROJECT_DIR/.claude/hooks/prompt-guard.py"}]}],
        "SessionStart":[{"hooks":[{"type":"command","command":"$CLAUDE_PROJECT_DIR/.claude/hooks/bootstrap-context.py"}]}]
      }
    }
  },
  "length_and_chunking":{
    "truncate":"Hard-truncate to limits with ellipsis '…'",
    "artifact_path":".claude/artifacts/last_validation.json",
    "reference_format":"artifact://.claude/artifacts/last_validation.json",
    "rule":"If additionalContext would exceed 4000 chars, keep summary/top-K; write full payload to artifact and reference it."
  },
  "bug_workarounds":[
    "If PreToolUse sometimes does not fire: verify matcher; re-apply hooks via /hooks; restart session. As fallback, enforce deny for critical paths.",
    "If PostToolUse is skipped due to tool crash: Stop/SubagentStop hooks must block with generic reason to prevent silent success.",
    "Avoid exit code 1 ambiguity; prefer JSON decisions or exit 2 for hard block.",
    "Remember stdout visibility exceptions (UserPromptSubmit, SessionStart); do not assume others will see stdout at exit 0.",
    "Windows quoting: prefer external scripts over complex inline one-liners; escape backslashes carefully."
  ],
  "conformance":{
    "must_pass":[
      {"name":"pre_allow","event":"PreToolUse","output":{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}},"schema":"PreToolUseAllow"},
      {"name":"pre_ask","event":"PreToolUse","output":{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"Need confirmation for billable API call."}},"schema":"PreToolUseAsk"},
      {"name":"pre_deny","event":"PreToolUse","output":{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Production file write outside allowlist."}},"schema":"PreToolUseDeny"},
      {"name":"post_block","event":"PostToolUse","output":{"decision":"block","reason":"Critical: unsafe command construction.","hookSpecificOutput":{"hookEventName":"PostToolUse"}},"schema":"PostToolUseBlock"},
      {"name":"post_soft_ok","event":"PostToolUse","output":{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"OK"}},"schema":"PostToolUseSoft"},
      {"name":"userprompt_block","event":"UserPromptSubmit","output":{"decision":"block","reason":"Sensitive content"},"schema":"UserPromptSubmitBlock"},
      {"name":"userprompt_add","event":"UserPromptSubmit","output":{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"seed ctx"}},"schema":"UserPromptSubmitAddContext"},
      {"name":"sessionstart_add","event":"SessionStart","output":{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"boot ctx"}},"schema":"SessionStartAddContext"},
      {"name":"stop_block","event":"Stop","output":{"decision":"block","reason":"Fix tests before stopping","hookSpecificOutput":{"hookEventName":"Stop"}},"schema":"StopBlock"},
      {"name":"subagentstop_block","event":"SubagentStop","output":{"decision":"block","reason":"Follow-up tasks required","hookSpecificOutput":{"hookEventName":"SubagentStop"}},"schema":"SubagentStopBlock"}
    ],
    "must_fail":[
      {"name":"markdown_in_additionalContext","why":"Backticks not allowed","bad":{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"```json {\"a\":1} ```"}}},
      {"name":"trailing_comma","why":"Invalid JSON","bad":"{\"decision\":\"block\",}"},
      {"name":"pre_permission_block_value","why":"Invalid permissionDecision value","bad":{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"block"}}},
      {"name":"unknown_top_key","why":"Unknown top-level key","bad":{"unexpectedKey":1}},
      {"name":"userprompt_ctx_not_string","why":"additionalContext must be plain string","bad":{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":{"foo":1}}}}
    ]
  },
  "ci":{
    "python":{
      "deps":["jsonschema"],
      "validate_script":"import sys, json, jsonschema\nguide = json.load(sys.stdin)\nschemas = guide[\"json_schemas\"]\n\ndef check(schema_name, instance):\n  jsonschema.validate(instance=instance, schema=schemas[schema_name])\nfor case in guide[\"conformance\"][\"must_pass\"]:\n  check(case[\"schema\"], case[\"output\"])\nfailed = 0\nfor case in guide[\"conformance\"][\"must_fail\"]:\n  try:\n    bad = case[\"bad\"]\n    hs = bad.get(\"hookSpecificOutput\",{}).get(\"hookEventName\")\n    if bad.get(\"decision\") == \"block\" and hs == \"PostToolUse\":\n      check(\"PostToolUseBlock\", bad)\n    elif hs == \"PostToolUse\":\n      check(\"PostToolUseSoft\", bad)\n    elif hs == \"PreToolUse\":\n      check(\"PreToolUseDeny\", bad)\n    elif hs == \"UserPromptSubmit\":\n      check(\"UserPromptSubmitAddContext\", bad)\n    elif hs == \"SessionStart\":\n      check(\"SessionStartAddContext\", bad)\n    elif bad.get(\"decision\") == \"block\" and hs == \"Stop\":\n      check(\"StopBlock\", bad)\n    elif bad.get(\"decision\") == \"block\" and hs == \"SubagentStop\":\n      check(\"SubagentStopBlock\", bad)\n    else:\n      jsonschema.validate(instance=bad, schema={\"type\":\"object\",\"required\":[\"__never__\"]})\n    print(\"UNEXPECTED_PASS:\", case[\"name\"], file=sys.stderr)\n  except Exception:\n    failed += 1\nif failed < len(guide[\"conformance\"][\"must_fail\"]):\n  raise SystemExit(2)\nprint(\"OK\")"
    }
  },
  "meta":{"generated_at":"2025-08-28T00:00:00Z","source":"v3.1 base improved to v3.2 with visibility routing, new schemas, stricter key sets, and SessionStart support.","hash":null}
}
