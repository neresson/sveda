# Tool approvals

A `tool.call` event with `confirmation: "required"` pauses the assistant. Every
pending call supports `approve` and `deny`. The sidecar resolves exact call IDs
with arguments from its own chat history.

The sidecar offers additional choices when the operator and the MCP host opt in. The optional `confirmationScope` on the event and history part advertises
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
- **Approve edits to this course in this chat** sends the same decision for a
  call with `confirmationScope: "course"`. One successful approval covers
  eligible changes to the same course across tools until chat deletion or
  revocation. The host revalidates each call.

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
- `once_per_course_per_chat` allows a course-wide grant for tools marked
  `_meta.confirm: "once_per_course_per_chat"`.
- `auto` allows execution without a prompt for tools marked
  `_meta.confirm: "auto_if_host_allows"` when the host approves the draft write.

Host `_meta.confirm: "always"` forces individual confirmation. Delete mode and
a positive MCP destructive annotation also force it unless both operator and
host opt into course-scoped confirmation for that tool. A positive open-world annotation on
a tool that is not read-only also forces it. Tools without new metadata retain
their existing behavior through `_meta.confirmation: "required"`. Hosts should
continue emitting that legacy field for tools that require approval. A host
should use `always` for publish, unpublish, enroll, assign, issue, send, and
other externally visible or destructive operations.

For guarded policies, the host implements the following authenticated MCP
JSON-RPC extension. The standard `tools/call` remains the path for individual
approvals and older sidecars.

- `sveda/approval/check` takes `name` and `arguments`. It performs no writes
  and returns `{ "scope": "opaque-stable-key" }` when eligible. Otherwise
  return `{ "scope": null }`. The key must encode the current authenticated
  account, tenant, and target, and must change if any of them changes. Course
  scope keys start with `course:` and must be identical across eligible tools for
  the same course. Auto scope keys start with `auto:` and never establish grants.
- `sveda/approval/call` takes `name`, `arguments`, and `scope`. The host
  must recheck the scope and action eligibility atomically with the write,
  then return the standard `tools/call` result. A rejection returns
  `{ "allowed": false }` and must perform no write. A successful call with
  tool output `{ "success": true, ... }` can establish the chat grant.
  Use a separate RPC method so an older host cannot silently run an unchecked
  write during a rolling deployment.

The LMS verifies draft creation and permits course grants for content updates,
lesson reordering and deletion, and eligible course deletion. Full course
deletion requires a private, unpublished, unarchived course without
enrollments or learner-facing paths. Publication, enrollment, path assignment,
and certificate issuance or preview use individual confirmation.

The wire additions are optional and keep protocol version 1.0. Legacy clients
continue sending `approve` and `deny`. The new Vue UI shows the extra buttons
only for calls that advertise the corresponding scope.
