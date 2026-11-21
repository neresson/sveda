# @sveda-ai/chat

Headless chat store, streaming, appearance presets, and shared helpers used by the Sveda UI packages.

- Site: [sveda.dev](https://sveda.dev)
- Docs: [sveda.dev/docs/chat-ui](https://sveda.dev/docs/chat-ui)

```bash
npm install @sveda-ai/chat
```

Install [`@sveda-ai/vue`](https://sveda.dev/docs/chat-ui/vue), [`@sveda-ai/react`](https://sveda.dev/docs/chat-ui/react), [`@sveda-ai/svelte`](https://sveda.dev/docs/chat-ui/svelte), or [`@sveda-ai/solid`](https://sveda.dev/docs/chat-ui/solid) when you want the widget. Those packages depend on this one.

## License

Apache License 2.0. See [LICENSE](LICENSE).

Host SDKs live in sibling repositories and use the same license. Sveda Cloud is a separate proprietary control plane.

### Tool approvals

Keep the individual approval and denial controls for every pending tool call.
If the sidecar emits `confirmationScope: "chat"`, call
`resolveToolConfirmation(toolCallId, "approve_for_chat")` to approve the call
and grant eligible changes to that target in the chat for 30 minutes. Call
`approveToolBatch(toolCallIds)` with the visible, eligible tool call IDs from
the same assistant step to approve them together. Both `"batch"` and `"chat"`
offers qualify for a batch. To revoke existing grants, call
`client.revokeToolApprovals(chatId)`.

[Sidecar settings and approval contract](../../docs/tool-approvals.md)
