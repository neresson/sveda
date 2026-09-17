import { useSvedaChat } from '@sveda-ai/vue';
import { onBeforeUnmount, watch } from 'vue';

const EMBED_FRAME_CHROME_PX = 24;
const EMBED_FRAME_MIN_WIDTH = 344;
const EMBED_FRAME_MAX_WIDTH = 824;
const EMBED_FRAME_MIN_HEIGHT = 424;
const EMBED_FRAME_MAX_HEIGHT = 924;

const postToParent = (data) => {
    if (typeof window === 'undefined' || !window.parent || window.parent === window) {
        return;
    }

    try {
        window.parent.postMessage(data, '*');
    } catch {}
};

export const useSvedaEmbedFrameBridge = () => {
    const chatStore = useSvedaChat();
    let frameRaf = 0;

    const frameSize = () => {
        if (chatStore.isMinimized.value) {
            return null;
        }

        const cap = (value, min, max) => Math.round(Math.max(min, Math.min(max, value)));

        return {
            frameWidth: cap(window.innerWidth + EMBED_FRAME_CHROME_PX, EMBED_FRAME_MIN_WIDTH, EMBED_FRAME_MAX_WIDTH),
            frameHeight: cap(window.innerHeight + EMBED_FRAME_CHROME_PX, EMBED_FRAME_MIN_HEIGHT, EMBED_FRAME_MAX_HEIGHT),
        };
    };

    const flushFrame = () => {
        if (chatStore.isMinimized.value) {
            postToParent({ type: 'sveda:resize', isMinimized: true });
            return;
        }

        const size = frameSize();
        if (!size) {
            return;
        }

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

    const onResize = () => {
        scheduleFrame();
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
        chatStore.minimizeChat();
        window.addEventListener('message', onParentMessage);
        window.addEventListener('resize', onResize);
        postToParent({ type: 'sveda:ready' });
    };

    const teardown = () => {
        window.removeEventListener('message', onParentMessage);
        window.removeEventListener('resize', onResize);
        if (frameRaf) {
            cancelAnimationFrame(frameRaf);
            frameRaf = 0;
        }
    };

    onBeforeUnmount(teardown);

    return { init, teardown };
};
