import { useSvedaChat } from '@sveda-ai/vue';
import { onBeforeUnmount, watch } from 'vue';

const DEFAULT_FRAME_WIDTH = 384;
const DEFAULT_FRAME_HEIGHT = 600;

const postToParent = (data) => {
    if (typeof window === 'undefined' || !window.parent || window.parent === window) {
        return;
    }

    try {
        window.parent.postMessage(data, '*');
    } catch {}
};

const readHostSize = () => {
    const width = Number.parseInt(
        document.documentElement.style.getPropertyValue('--sveda-embed-width'),
        10,
    );
    const height = Number.parseInt(
        document.documentElement.style.getPropertyValue('--sveda-embed-height'),
        10,
    );

    return {
        frameWidth: Number.isFinite(width) && width > 0 ? width : DEFAULT_FRAME_WIDTH,
        frameHeight: Number.isFinite(height) && height > 0 ? height : DEFAULT_FRAME_HEIGHT,
    };
};

export const useSvedaEmbedFrameBridge = () => {
    const chatStore = useSvedaChat();
    let frameRaf = 0;

    const flushFrame = () => {
        if (chatStore.isMinimized.value) {
            postToParent({ type: 'sveda:resize', isMinimized: true });
            return;
        }

        const size = readHostSize();
        postToParent({
            type: 'sveda:resize',
            isMinimized: false,
            frameWidth: size.frameWidth,
            frameHeight: size.frameHeight,
        });
    };

    const scheduleFrame = () => {
        if (frameRaf) {
            cancelAnimationFrame(frameRaf);
        }

        frameRaf = requestAnimationFrame(() => {
            frameRaf = 0;
            flushFrame();
        });
    };

    watch(
        () => chatStore.isMinimized.value,
        () => {
            scheduleFrame();
        },
        { immediate: true, flush: 'post' },
    );

    const onParentMessage = (event) => {
        const data = event.data;
        if (!data || typeof data !== 'object') {
            return;
        }

        if (data.type === 'sveda:open') {
            chatStore.maximizeChat();
            return;
        }

        if (data.type === 'sveda:close') {
            chatStore.minimizeChat();
        }
    };

    const init = () => {
        window.addEventListener('message', onParentMessage);
        window.addEventListener('sveda:embed-host-size', scheduleFrame);
        postToParent({ type: 'sveda:ready' });
        scheduleFrame();
    };

    const teardown = () => {
        window.removeEventListener('message', onParentMessage);
        window.removeEventListener('sveda:embed-host-size', scheduleFrame);
        if (frameRaf) {
            cancelAnimationFrame(frameRaf);
            frameRaf = 0;
        }
    };

    onBeforeUnmount(teardown);

    return { init, teardown };
};
