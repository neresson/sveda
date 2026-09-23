# @sveda-ai/core

## 0.3.0

### Minor Changes

- Ask the chat user to confirm opted-in tool calls before they run.

### Patch Changes

- dc18ded: Enforce admin tool policies from embed capabilities so hosts can restrict what the agent may do.
- 356467e: Replay a reasoning item for every DeepSeek assistant turn when tools are present, including text-only history without stored CoT.
- 73d375a: Keep DeepSeek parallel tool calls in one reasoning turn so thinking-mode continuations do not 400.
- 9378767: Fix DeepSeek continued chats: replay reasoning once per turn and emit function_call_output for client tool-result parts.
- dc18ded: Add an OpenAI embeddings catalog with operator keys and configurable vector dimensions, and drop Yandex.
- Updated dependencies [dc18ded]
- Updated dependencies [dc18ded]
- Updated dependencies
  - @sveda-ai/protocol@0.3.0
