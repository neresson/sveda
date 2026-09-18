<script setup>
import { SvedaChat, SvedaFillHostKey, Toaster } from '@sveda-ai/vue';
import { onMounted, provide } from 'vue';
import { useSvedaEmbedFrameBridge } from './useSvedaEmbedFrameBridge';

const props = defineProps({
    brandName: { type: String, default: 'Sveda' },
    pageUrl: { type: String, default: '' },
});

provide(SvedaFillHostKey, true);

const bridge = useSvedaEmbedFrameBridge();

onMounted(() => {
    bridge.init();
});
</script>

<template>
    <div class="sveda-chat sveda-chat-host relative h-full min-h-0 w-full bg-transparent">
        <SvedaChat :brand-name="props.brandName" :page-url="props.pageUrl" />
        <Toaster />
    </div>
</template>
