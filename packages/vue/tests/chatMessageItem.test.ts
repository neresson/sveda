import { afterEach, describe, expect, it, vi } from 'vitest';
import { createApp, h, nextTick, ref } from 'vue';
import type { SvedaAssistantMessagePart } from '@sveda-ai/chat';
import ChatMessageItem from '../src/components/chat/ChatMessageItem.vue';
import { createSveda } from '../src/plugin';

const unmount: (() => void)[] = [];
afterEach(() => {
  unmount.splice(0).forEach(cleanup => cleanup());
});

function mountMessage(parts: SvedaAssistantMessagePart[]) {
  const currentParts = ref(parts);
  const disabled = ref(false);
  const confirm = vi.fn();
  const deny = vi.fn();
  const batch = vi.fn();
  const root = document.createElement('div');
  document.body.append(root);
  const app = createApp({
    render: () => h(ChatMessageItem, {
      message: { role: 'assistant', parts: currentParts.value },
      approvalDisabled: disabled.value,
      onConfirmTool: confirm,
      onDenyTool: deny,
      onApproveToolBatch: batch,
    }),
  });
  app.use(createSveda({
    endpoints: { stream: '/stream' },
    locale: 'ru',
    messages: { ru: { 'tools.publish_course': 'Опубликовать курс' } },
  }));
  app.mount(root);
  unmount.push(() => {
    app.unmount();
    root.remove();
  });
  return { root, currentParts, disabled, confirm, deny, batch };
}

function approval(id: string, scope?: SvedaAssistantMessagePart['confirmationScope']): SvedaAssistantMessagePart {
  return {
    type: 'tool-call',
    toolCallId: id,
    toolName: 'publish_course',
    input: { id: 39, internal_payload: 'raw argument' },
    confirmation: 'required',
    confirmationScope: scope,
  };
}

describe('assistant message presentation', () => {
  it('shows one reasoning preview and puts approval after the activity history', async () => {
    const pending = approval('publish');
    const { root, currentParts } = mountMessage([
      { type: 'reasoning', text: 'Предыдущие размышления' },
      { type: 'tool-result', toolCallId: 'create', toolName: 'create_course', output: { success: true } },
      { type: 'reasoning', text: 'Проверяю структуру курса. '.repeat(70) },
      { type: 'text', text: 'Черновик курса готов.' },
      pending,
    ]);

    expect(root.querySelectorAll('.reasoning-stream')).toHaveLength(1);
    expect(root.querySelector('.reasoning-stream')?.textContent?.length).toBeLessThan(620);
    expect(root.textContent).not.toContain('Предыдущие размышления');
    expect(root.querySelector('details')?.hasAttribute('open')).toBe(false);
    expect(root.querySelector('details .sveda-tool-approval')).toBeNull();
    const card = root.querySelector('.sveda-tool-approval');
    expect(card?.textContent).toContain('Опубликовать курс');
    expect(card?.textContent).toContain('Объект №39');
    expect(card?.textContent).not.toContain('raw argument');

    currentParts.value.push({ type: 'tool-result', toolCallId: pending.toolCallId, output: { success: true } });
    await nextTick();
    expect(root.querySelector('.sveda-tool-approval')).toBeNull();
    expect(root.querySelector('summary')?.textContent).toContain('Действия в чате · 2');
  });

  it('forwards the selected decision and disables choices during the request', async () => {
    const { root, confirm, deny, disabled } = mountMessage([approval('course', 'course')]);
    const [once, reject, scoped] = root.querySelectorAll<HTMLButtonElement>('.sveda-tool-approval button');
    once.click();
    reject.click();
    scoped.click();
    expect(confirm.mock.calls).toEqual([['course', 'approve'], ['course', 'approve_for_chat']]);
    expect(deny.mock.calls).toEqual([['course']]);

    disabled.value = true;
    await nextTick();
    expect([once, reject, scoped].every(button => button.disabled)).toBe(true);
  });

  it('includes only eligible approvals in the batch decision', () => {
    const { root, batch } = mountMessage([
      approval('batch', 'batch'),
      approval('chat', 'chat'),
      approval('course', 'course'),
      approval('single'),
    ]);
    const batchButton = [...root.querySelectorAll<HTMLButtonElement>('button')].find(
      button => button.textContent?.includes('Подтвердить все доступные действия (2)'),
    );
    expect(batchButton).toBeDefined();
    batchButton?.click();
    expect(batch.mock.calls).toEqual([[['batch', 'chat']]]);
  });

  it('does not count a declined action as an execution error', () => {
    const { root } = mountMessage([
      approval('declined'),
      { type: 'tool-result', toolCallId: 'declined', output: { success: false, denied: true } },
    ]);
    expect(root.querySelector('.sveda-tool-approval')).toBeNull();
    expect(root.querySelector('summary')?.textContent).not.toContain('Ошибки');
  });
});
