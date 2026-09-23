<script setup>
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

defineOptions({ name: 'CodeIndexScopeTreeNode' });

const props = defineProps({
    node: { type: Object, required: true },
    csrf: { type: String, required: true },
    scopeUrl: { type: String, required: true },
    depth: { type: Number, default: 0 },
    isPathExcluded: { type: Function, required: true },
    setExcluded: { type: Function, required: true },
});

const { t } = useI18n();

const expanded = ref(false);
const children = ref(null);
const loading = ref(false);
const loadError = ref('');

const toggleExpand = async () => {
    if (props.node.kind !== 'dir') {
        return;
    }
    expanded.value = !expanded.value;
    if (!expanded.value || children.value !== null) {
        return;
    }
    loading.value = true;
    loadError.value = '';
    try {
        const url = new URL(props.scopeUrl, window.location.origin);
        url.searchParams.set('parent', props.node.path);
        const response = await fetch(url.toString(), {
            credentials: 'same-origin',
            headers: {
                Accept: 'application/json',
                'X-CSRF-TOKEN': props.csrf,
            },
        });
        const data = await response.json().catch(() => ({}));
        if (!response.ok) {
            loadError.value = data.message || t('sources.scope_load_error');
            children.value = [];
            return;
        }
        children.value = data.entries || [];
    } catch {
        loadError.value = t('sources.scope_load_error');
        children.value = [];
    } finally {
        loading.value = false;
    }
};
</script>

<template>
    <li>
        <div class="flex items-center gap-2 py-1" :style="{ paddingLeft: `${depth * 14}px` }">
            <button
                v-if="node.kind === 'dir'"
                type="button"
                class="w-5 font-mono text-xs"
                @click="toggleExpand"
            >
                {{ loading ? '…' : expanded ? '▾' : '▸' }}
            </button>
            <span v-else class="inline-block w-5" />
            <input
                type="checkbox"
                :checked="isPathExcluded(node.path)"
                @change="setExcluded(node.path, $event.target.checked)"
            >
            <span class="truncate font-mono text-sm">{{ node.name }}</span>
        </div>
        <p v-if="loadError" class="font-mono text-[11px]" :style="{ paddingLeft: `${(depth + 1) * 14}px` }">
            {{ loadError }}
        </p>
        <ul v-if="expanded && children?.length">
            <CodeIndexScopeTreeNode
                v-for="child in children"
                :key="`${child.kind}-${child.path}`"
                :node="child"
                :csrf="csrf"
                :scope-url="scopeUrl"
                :depth="depth + 1"
                :is-path-excluded="isPathExcluded"
                :set-excluded="setExcluded"
            />
        </ul>
    </li>
</template>
