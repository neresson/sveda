export function isToolInvocationComplete(invocation) {
  return ['result', 'output-available', 'output-error', 'output-denied'].includes(invocation.state);
}

export function needsToolConfirmation(invocation) {
  return invocation.confirmation === 'required' && !isToolInvocationComplete(invocation);
}

export function translateToolName(t, toolName) {
  if (!toolName) {
    return '';
  }
  const key = `tools.${toolName}`;
  const translated = t(key);
  return translated === key ? toolName.replace(/[_-]+/g, ' ') : translated;
}
