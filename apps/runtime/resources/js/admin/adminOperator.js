export const ADMIN_TOOL_PREFIX = 'admin_';

export const pageFromAdminToolResult = (part) => {
    if (!part || part.type !== 'tool-result') {
        return null;
    }

    const toolName = String(part.toolName ?? '');
    if (!toolName.startsWith(ADMIN_TOOL_PREFIX)) {
        return null;
    }

    const id = String(part.toolCallId ?? '');
    if (!id) {
        return null;
    }

    const output = part.output && typeof part.output === 'object' ? part.output : {};
    if (output.success === false) {
        return null;
    }

    const page = typeof output.page === 'string' ? output.page.trim() : '';
    const data = output.data && typeof output.data === 'object' ? output.data : {};
    const url = typeof data.url === 'string' ? data.url.trim() : '';
    const reload = output.reload === true || toolName === 'admin_open_page';
    if (!reload && page === '' && url === '') {
        return null;
    }

    return { id, page, url, reload: true };
};
