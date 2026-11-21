# Tool approvals

A `tool.call` event with `confirmation: "required"` pauses the assistant. Every
pending call supports `approve` and `deny`. The sidecar resolves exact call IDs
with arguments from its own chat history.

The sidecar offers two additional choices when the operator and the MCP host
opt in. The optional `confirmationScope` on the event and history part advertises
the choices to clients.

- **Approve all eligible actions** sends one request with an `approve_batch`
  decision for each visible eligible call in the pending assistant step.
  Every submitted call is checked again. The decision grants no future calls.
- **Approve for this chat** sends `approve_for_chat` for one call with
  `confirmationScope: "chat"`. After the call succeeds, the sidecar grants
  eligible calls on the same target for 30 minutes from that approval.
  The grant is bound to the authenticated visitor, chat, MCP endpoint, tool,
  and the target scope returned by the host. Chat deletion ends the grant.
  An authenticated `DELETE /sveda/chat-histories/{chatId}/tool-approvals`
  revokes all grants in that chat. It returns `204` on success and `409` if a
  turn is active. Use shared Redis and PostgreSQL for multi-replica deployments.

On a rejected or expired grant, the call pauses for approval again. A retry
after a lost approval response uses the checkpointed decision or reports an
unknown result if the host may already have acted. Neither client replay
arguments nor grant requests can override host authorization.

## Sidecar and MCP host settings

The `tool_approvals` map in `sveda.yaml` and the authenticated
`/admin/settings` API defaults to empty. A patch replaces the entire map.
It accepts exact tool names with these values.

- `always` forces individual confirmation.
- `batch` allows batching for tools with `_meta.confirm: "batch"`.
- `once_per_chat` allows batching for tools with `_meta.confirm: "batch"`
  and scoped approvals for tools with `_meta.confirm: "once_per_chat"`.

Host `_meta.confirm: "always"`, delete mode, and a positive MCP destructive
annotation force individual confirmation. A positive open-world annotation on
a tool that is not read-only also forces it. Tools without new metadata retain
their existing behavior through `_meta.confirmation: "required"`. Hosts should
continue emitting that legacy field for tools that require approval. A host
should use `always` for publish, unpublish, enroll, assign, issue, send, and
other externally visible or destructive operations.

For `once_per_chat`, the host implements the following authenticated MCP
JSON-RPC extension. The standard `tools/call` remains the path for individual
approvals and older sidecars.

- `sveda/approval/check` takes `name` and `arguments`. It performs no writes
  and returns `{ "scope": "opaque-stable-key" }` when eligible. Otherwise
  return `{ "scope": null }`. The key must encode the current authenticated
  account, tenant, and target, and must change if any of them changes.
- `sveda/approval/call` takes `name`, `arguments`, and `scope`. The host
  must recheck the scope and action eligibility atomically with the write,
  then return the standard `tools/call` result. A rejection returns
  `{ "allowed": false }` and must perform no write. A successful call with
  tool output `{ "success": true, ... }` can establish the chat grant.
  Use a separate RPC method so an older host cannot silently run an unchecked
  write during a rolling deployment.

The LMS grants `update_content` only for lessons in unpublished, unarchived
courses with no enrollments.
Its guarded path accepts title, description, and type edits and text or
multiple-choice block edits that retain every existing block and its type.
Published-course edits, block removals and replacements of existing block
types, and other edits outside that scope use individual confirmation. Draft
onboarding program creation supports batching. Publication, enrollment, path
assignment, and certificate issuance or preview use individual confirmation.

The wire additions are optional and keep protocol version 1.0. Legacy clients
continue sending `approve` and `deny`. The new Vue UI shows the extra buttons
only for calls that advertise the corresponding scope.
