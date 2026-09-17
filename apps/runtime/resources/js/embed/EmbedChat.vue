<script setup>
import { Toaster, SvedaChat } from '@sveda-ai/vue';
import { onMounted } from 'vue';
import { useSvedaEmbedFrameBridge } from './useSvedaEmbedFrameBridge';

const props = defineProps({
    brandName: { type: String, default: 'Sveda' },
    pageUrl: { type: String, default: '' },
});

const bridge = useSvedaEmbedFrameBridge();

onMounted(() => {
    bridge.init();
});
</script>

<template>
    <div class="sveda-chat sveda-chat-host relative bg-transparent">
        <SvedaChat :brand-name="props.brandName" :page-url="props.pageUrl" />
        <Toaster />
    </div>
</template>
