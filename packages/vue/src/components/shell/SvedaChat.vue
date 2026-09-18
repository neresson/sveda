<script setup>
  import SvedaAgentCompletedNotice from './SvedaAgentCompletedNotice.vue';
  import SvedaAgentTasksPanel from './SvedaAgentTasksPanel.vue';
  import SvedaFrameTicks from './SvedaFrameTicks.vue';
  import SvedaHistorySheet from './SvedaHistorySheet.vue';
  import SvedaHistorySidebar from './SvedaHistorySidebar.vue';
  import SvedaInputSection from './SvedaInputSection.vue';
  import SvedaLandingView from './SvedaLandingView.vue';
  import SvedaMaxStepsNotice from './SvedaMaxStepsNotice.vue';
  import SvedaMinimizedTrigger from './SvedaMinimizedTrigger.vue';
  import SvedaQuickPrompts from './SvedaQuickPrompts.vue';
  import SvedaResizeHandles from './SvedaResizeHandles.vue';
  import SvedaTabs from './SvedaTabs.vue';
  import SvedaToolbar from './SvedaToolbar.vue';
  import ChatHistoryDropdown from '../chat/ChatHistoryDropdown.vue';
  import ChatMessageList from '../chat/ChatMessageList.vue';
  import { Button } from '../../ui/button';
  import { Card, CardContent } from '../../ui/card';
  import { Input } from '../../ui/input';
  import { ResponsiveDialog } from '../../ui/responsive-dialog';
  import { useToast } from '../../ui/toast';
  import { useSvedaChatPage } from '../../composables/useSvedaChatPage';
  import { useSvedaT } from '../../i18n/index';
  import { useSvedaConfig, SvedaHideLauncherKey } from '../../plugin';
  import { Plus } from 'lucide-vue-next';
  import { computed, inject, ref } from 'vue';

  const props = defineProps({
    models: { type: Array, default: undefined },
    quickPrompts: { type: Array, default: undefined },
    brandName: { type: String, default: undefined },
    brandLogo: { type: String, default: undefined },
    pageUrl: { type: String, default: undefined },
    onNavigate: { type: Function, default: undefined },
    notify: { type: Function, default: undefined },
    agentTasksSubscribe: { type: Function, default: undefined },
  });

  const config = useSvedaConfig();
  const hideLauncher = inject(SvedaHideLauncherKey, false);
  const t = useSvedaT();
  const { toast } = useToast();

  const messagesContainerRef = ref(null);
  const chatCardRef = ref(null);

  const notifyHandler = (kind, message) => {
    if (typeof props.notify === 'function') {
      props.notify(kind, message);
      return;
    }
    if (kind === 'error') {
      toast({ title: t('commonError'), description: message, variant: 'destructive' });
    }
  };

  const chat = useSvedaChatPage(
    {
      models: computed(() => props.models ?? config.models),
      quickPrompts: computed(() => props.quickPrompts ?? config.quickPrompts),
      brand: computed(() => ({
        name: props.brandName ?? config.brand.name,
        logoUrl: props.brandLogo ?? config.brand.logoUrl,
      })),
      pageUrl: computed(() => props.pageUrl ?? ''),
      onNavigate: props.onNavigate,
      notify: notifyHandler,
      agentTasksSubscribe: props.agentTasksSubscribe,
    },
    { messagesContainerRef, chatCardRef }
  );
</script>

<template>
  <div
    :class="[
      'sveda-chat',
      chat.fillHost
        ? 'relative h-full min-h-0 w-full overflow-hidden'
        : chat.isMinimized
          ? 'fixed bottom-0 right-0 z-50 min-[1872px]:bottom-4 min-[1872px]:right-4'
          : chat.isMobile
            ? 'sveda-chat-surface fixed inset-0 z-50 pb-[env(safe-area-inset-bottom)] pt-[env(safe-area-inset-top)]'
            : chat.viewMode === 'immersive'
              ? 'sveda-chat-surface fixed right-0 top-0 z-50 flex h-[100dvh] flex-col pb-[env(safe-area-inset-bottom)] pt-[env(safe-area-inset-top)]'
              : chat.viewMode === 'fixed'
                ? 'sveda-chat-surface fixed right-0 top-0 z-50 h-screen pt-[env(safe-area-inset-top)]'
                : 'fixed bottom-4 right-4 z-50',
      'flex flex-col',
      chat.fillHost || chat.isMobile || chat.viewMode === 'fixed' || chat.viewMode === 'immersive'
        ? 'items-stretch'
        : 'items-end gap-2',
      chat.isImmersiveModeTransitioning && !chat.isMobile && !chat.isMinimized ? 'overflow-hidden transition-[width,height] duration-300 ease-out' : '',
    ]"
    :style="chat.chatShellStyle"
  >
    <Transition name="chat">
      <div
        v-if="!chat.isMinimized"
        :class="[
          chat.isMobile || chat.viewMode === 'fixed' || chat.fillHost
            ? 'flex h-full min-h-0 w-full min-w-0 flex-1'
            : chat.viewMode === 'immersive'
              ? 'flex h-full min-h-0 w-full min-w-0 flex-1 flex-col'
              : 'flex gap-2',
        ]"
      >
        <div
          v-if="chat.isImmersiveDesktop"
          class="sveda-chat-surface flex h-full min-h-0 w-full flex-1 flex-col"
        >
          <SvedaToolbar
            :is-mobile="chat.isMobile"
            :view-mode="chat.viewMode"
            :is-immersive-desktop="chat.isImmersiveDesktop"
            :show-history-sidebar="chat.showHistorySidebar"
            :title="chat.headerTitle"
            :view-mode-floating-label="chat.t('viewModeFloating')"
            :view-mode-fixed-label="chat.t('viewModeFixed')"
            :immersive-enter-label="chat.t('immersiveModeEnter')"
            :immersive-exit-label="chat.t('immersiveModeExit')"
            :immersive-brand-name="chat.brandDisplayName"
            :immersive-brand-logo="chat.brandLogo"
            @toggle-history-sidebar="chat.showHistorySidebar = !chat.showHistorySidebar"
            @exit-immersive="chat.exitImmersiveMode"
            @enter-immersive="chat.enterImmersiveMode"
            @toggle-view-mode="chat.toggleViewMode"
            @new-chat="chat.handleNewChat"
            @minimize="chat.minimizeChat"
          />
          <div class="flex min-h-0 flex-1 flex-row">
            <aside class="sveda-chat-surface flex w-[min(20rem,33vw)] shrink-0 flex-col border-r border-border">
              <div class="border-b border-border/50 p-3">
                <Button
                  variant=""
                  class="w-full justify-center gap-2"
                  @click="chat.handleNewChat"
                >
                  <Plus class="h-4 w-4" />
                  {{ chat.t('newChat') }}
                </Button>
              </div>
              <div class="min-h-0 flex-1">
                <ChatHistoryDropdown
                  variant="sidebar"
                  :show="true"
                  :histories="chat.sortedHistories"
                  :current-chat-id="chat.currentChat?.id"
                  @close="chat.showHistorySidebar = false"
                  @select-chat="chat.handleSelectChat"
                  @delete-chat="chat.handleDeleteChat"
                  @rename-chat="chat.handleRenameChat"
                />
              </div>
            </aside>
            <div class="sveda-chat-surface flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
              <SvedaTabs
                :tabs="chat.openChatTabs"
                :current-chat-id="chat.currentChat?.id"
                :streaming-chat-ids="chat.streamingChatIds"
                :unread-chat-ids="chat.unreadChatIds"
                @select-chat="chat.handleSelectChat"
                @close-tab="chat.closeChatTab"
                @new-chat="chat.handleNewChat"
              />
              <div class="mx-auto flex min-h-0 w-full max-w-7xl flex-1 flex-col overflow-hidden px-4 md:px-6">
                <Transition
                  name="landing-layout-shift"
                  mode="out-in"
                >
                  <SvedaLandingView
                    v-if="chat.isImmersiveLandingLayout"
                    key="immersive-landing"
                    v-model:input-message="chat.inputMessage"
                    v-model:selected-chat-model="chat.selectedChatModel"
                    v-model:thinking-enabled="chat.thinkingEnabled"
                    v-model:pending-files="chat.pendingChatFiles"
                    :is-loading="chat.isLoading || chat.isLoadingChatHistory"
                    :has-current-chat="chat.hasCurrentChat"
                    :is-streaming="chat.isStreaming"
                    :context-window-usage-percent="chat.contextWindowUsagePercent"
                    :show-quick-prompt-buttons="chat.showQuickPromptButtons"
                    :quick-prompt-buttons="chat.quickPromptButtons"
                    :models="chat.chatModels"
                    :thinking-tooltip="chat.thinkingTooltipText"
                    :status-banner="chat.chatInputStatusBanner"
                    @send="chat.sendMessage"
                    @stop="chat.stopStreaming"
                    @send-quick-prompt="chat.sendQuickPrompt"
                  >
                    <template #top>
                      <SvedaAgentTasksPanel
                        v-if="chat.agentTasks.tasks.length > 0"
                        class="mb-4 shrink-0"
                        :phase="chat.agentTasks.phase"
                        :tasks="chat.agentTasks.tasks"
                        :show-live-status="chat.isStreaming"
                      />
                      <ChatMessageList
                        v-if="chat.messages.length > 0"
                        ref="messagesContainerRef"
                        :messages="chat.messages"
                        :is-loading="chat.isLoading || chat.isLoadingChatHistory"
                        :is-thinking="chat.isThinking"
                        :content-animations-enabled="chat.isStreaming"
                        :thinking-message="chat.thinkingMessage"
                        no-mt-auto
                        class="mb-4 max-h-[30vh] flex-none"
                        @near-bottom-change="chat.handleMessagesNearBottomChange"
                        @activity-link-click="chat.handleActivityLinkClick"
                      />
                    </template>
                  </SvedaLandingView>

                  <div
                    v-else
                    key="immersive-conversation"
                    class="flex min-h-0 flex-1 flex-col"
                  >
                    <SvedaAgentTasksPanel
                      v-if="chat.agentTasks.tasks.length > 0"
                      :phase="chat.agentTasks.phase"
                      :tasks="chat.agentTasks.tasks"
                      :show-live-status="chat.isStreaming"
                    />
                    <ChatMessageList
                      ref="messagesContainerRef"
                      :messages="chat.messages"
                      :is-loading="chat.isLoading || chat.isLoadingChatHistory"
                      :is-thinking="chat.isThinking"
                      :content-animations-enabled="chat.isStreaming"
                      :thinking-message="chat.thinkingMessage"
                      @near-bottom-change="chat.handleMessagesNearBottomChange"
                      @activity-link-click="chat.handleActivityLinkClick"
                    />
                    <SvedaMaxStepsNotice
                      v-if="chat.showMaxStepsContinueNotice"
                      :message="chat.maxStepsContinueMessage"
                      :continue-label="chat.t('maxStepsContinue')"
                      :loading="chat.isLoading || chat.isLoadingChatHistory || chat.isStreaming"
                      @continue="chat.continueAfterMaxSteps"
                    />
                    <SvedaAgentCompletedNotice
                      v-if="chat.showAgentCompletedNotice"
                      :label="chat.t('agentFinishedNotice')"
                      @scroll="chat.scrollToCompletedAnswer"
                    />
                    <SvedaInputSection
                      v-model:input-message="chat.inputMessage"
                      v-model:pending-files="chat.pendingChatFiles"
                      v-model:selected-chat-model="chat.selectedChatModel"
                      v-model:thinking-enabled="chat.thinkingEnabled"
                      :is-loading="chat.isLoading || chat.isLoadingChatHistory"
                      :has-current-chat="chat.hasCurrentChat"
                      :is-streaming="chat.isStreaming"
                      :status-banner="chat.chatInputStatusBanner"
                      :model-select-placeholder="chat.t('modelSelectPlaceholder')"
                      :models="chat.chatModels"
                      :thinking-tooltip="chat.thinkingTooltipText"
                      :context-window-usage-percent="chat.contextWindowUsagePercent"
                      :context-window-tooltip="chat.t('contextWindowPercent', { percent: chat.contextWindowUsagePercent })"
                      @send="chat.sendMessage"
                      @stop="chat.stopStreaming"
                    />
                  </div>
                </Transition>
              </div>
            </div>
          </div>
        </div>

        <div
          v-else
          :class="chat.nonImmersiveShellClass"
          :style="chat.floatNonImmersiveShellStyle"
        >
          <SvedaHistorySidebar
            v-if="chat.showHistorySidebar && !chat.isMobile"
            :surface-class="chat.historyAsideSurfaceClass"
            :histories="chat.sortedHistories"
            :current-chat-id="chat.currentChat?.id"
            :new-chat-label="chat.t('newChat')"
            @new-chat="chat.handleNewChat"
            @select-chat="chat.handleSelectChat"
            @delete-chat="chat.handleDeleteChat"
            @rename-chat="chat.handleRenameChat"
            @close="chat.showHistorySidebar = false"
          />
          <Card
            ref="chatCardRef"
            class="sveda-chat-surface relative flex min-h-0 flex-col overflow-visible"
            :class="{
              resizing: chat.isResizing || chat.isEnteringImmersiveFromDrag,
              'sveda-chat-frame': chat.viewMode === 'floating' && !chat.isMobile && !chat.fillHost,
              'border-0': chat.fillHost || chat.isMobile,
              'h-full min-w-0 flex-1 rounded-none': chat.isMobile,
              'h-full min-w-0 flex-1 rounded-none border-b-0 border-l border-r-0 border-t-0 border-border': chat.viewMode === 'fixed' && !chat.isMobile,
              'rounded-[var(--sveda-radius)]': chat.viewMode === 'floating',
              'rounded-none': chat.isMobile || chat.viewMode === 'fixed',
              'transition-[width] duration-300 ease-out': chat.isEnteringImmersiveFromDrag && chat.viewMode === 'fixed' && !chat.isMobile,
              'min-w-0 shrink-0': chat.viewMode === 'floating' && !chat.isMobile && !chat.fillHost,
              'h-full min-w-0 flex-1': chat.viewMode === 'floating' && !chat.isMobile && chat.fillHost,
            }"
            :style="chat.chatCardStyle"
          >
            <SvedaFrameTicks v-if="!chat.isMobile && (chat.viewMode === 'floating' || chat.fillHost)" />
            <SvedaResizeHandles
              :is-mobile="chat.isMobile"
              :view-mode="chat.viewMode"
              @start-resize="chat.startResize"
            />
            <SvedaToolbar
              :is-mobile="chat.isMobile"
              :view-mode="chat.viewMode"
              :is-immersive-desktop="chat.isImmersiveDesktop"
              :show-history-sidebar="chat.showHistorySidebar"
              :title="chat.headerTitle"
              :view-mode-floating-label="chat.t('viewModeFloating')"
              :view-mode-fixed-label="chat.t('viewModeFixed')"
              :immersive-enter-label="chat.t('immersiveModeEnter')"
              :immersive-exit-label="chat.t('immersiveModeExit')"
              :immersive-brand-name="chat.brandDisplayName"
              :immersive-brand-logo="chat.brandLogo"
              @toggle-history-sidebar="chat.showHistorySidebar = !chat.showHistorySidebar"
              @exit-immersive="chat.exitImmersiveMode"
              @enter-immersive="chat.enterImmersiveMode"
              @toggle-view-mode="chat.toggleViewMode"
              @new-chat="chat.handleNewChat"
              @minimize="chat.minimizeChat"
            />

            <CardContent class="sveda-chat-surface relative flex flex-1 flex-col overflow-hidden p-0">
              <div class="flex min-h-0 flex-1 flex-col">
                <SvedaTabs
                  :tabs="chat.openChatTabs"
                  :current-chat-id="chat.currentChat?.id"
                  :streaming-chat-ids="chat.streamingChatIds"
                  :unread-chat-ids="chat.unreadChatIds"
                  compact
                  @select-chat="chat.handleSelectChat"
                  @close-tab="chat.closeChatTab"
                  @new-chat="chat.handleNewChat"
                />
                <SvedaAgentTasksPanel
                  v-if="chat.agentTasks.tasks.length > 0"
                  class="mx-4 mt-3 shrink-0"
                  :phase="chat.agentTasks.phase"
                  :tasks="chat.agentTasks.tasks"
                  :show-live-status="chat.isStreaming"
                />
                <ChatMessageList
                  ref="messagesContainerRef"
                  :messages="chat.messages"
                  :is-loading="chat.isLoading || chat.isLoadingChatHistory"
                  :is-thinking="chat.isThinking"
                  :content-animations-enabled="chat.isStreaming"
                  :thinking-message="chat.thinkingMessage"
                  @near-bottom-change="chat.handleMessagesNearBottomChange"
                  @activity-link-click="chat.handleActivityLinkClick"
                />
                <SvedaMaxStepsNotice
                  v-if="chat.showMaxStepsContinueNotice"
                  :message="chat.maxStepsContinueMessage"
                  :continue-label="chat.t('maxStepsContinue')"
                  :loading="chat.isLoading || chat.isLoadingChatHistory || chat.isStreaming"
                  @continue="chat.continueAfterMaxSteps"
                />
                <SvedaAgentCompletedNotice
                  v-if="chat.showAgentCompletedNotice"
                  :label="chat.t('agentFinishedNotice')"
                  @scroll="chat.scrollToCompletedAnswer"
                />
                <SvedaQuickPrompts
                  v-if="chat.showQuickPromptButtons"
                  class="shrink-0 px-4 pb-1 pt-0"
                  :prompts="chat.quickPromptButtons"
                  :title="chat.t('quickPromptsTitle')"
                  :disabled="chat.isLoading || chat.isStreaming"
                  @select="chat.sendQuickPrompt"
                />
                <SvedaInputSection
                  v-model:input-message="chat.inputMessage"
                  v-model:pending-files="chat.pendingChatFiles"
                  v-model:selected-chat-model="chat.selectedChatModel"
                  v-model:thinking-enabled="chat.thinkingEnabled"
                  :is-loading="chat.isLoading || chat.isLoadingChatHistory"
                  :has-current-chat="chat.hasCurrentChat"
                  :is-streaming="chat.isStreaming"
                  :status-banner="chat.chatInputStatusBanner"
                  :model-select-placeholder="chat.t('modelSelectPlaceholder')"
                  :models="chat.chatModels"
                  :thinking-tooltip="chat.thinkingTooltipText"
                  :context-window-usage-percent="chat.contextWindowUsagePercent"
                  :context-window-tooltip="chat.t('contextWindowPercent', { percent: chat.contextWindowUsagePercent })"
                  @send="chat.sendMessage"
                  @stop="chat.stopStreaming"
                />
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </Transition>

    <SvedaHistorySheet
      :open="chat.isMobile && chat.showHistorySidebar"
      :title="chat.t('chatHistory')"
      :histories="chat.sortedHistories"
      :current-chat-id="chat.currentChat?.id"
      :new-chat-label="chat.t('newChat')"
      @update:open="chat.showHistorySidebar = $event"
      @new-chat="chat.handleNewChat"
      @select-chat="
        chatId => {
          chat.handleSelectChat(chatId);
          chat.showHistorySidebar = false;
        }
      "
      @delete-chat="chat.handleDeleteChat"
      @rename-chat="chat.handleRenameChat"
    />

    <SvedaMinimizedTrigger
      v-if="chat.isMinimized && !chat.isMobile && !hideLauncher"
      :label="chat.launcherLabel"
      :icon="chat.launcherIcon"
      :logo-src="chat.launcherImage"
      :logo-alt="chat.t('assistantLogoAlt')"
      @open="chat.openChat"
    />

    <ResponsiveDialog v-model:open="chat.showDeleteChatDialog">
      <template #title>{{ chat.t('commonDelete') }}</template>
      <template #description>{{ chat.t('confirmDeleteChat') }}</template>
      <template #footer>
        <Button
          variant="outline"
          @click="chat.cancelDeleteChat"
        >
          {{ chat.t('commonCancel') }}
        </Button>
        <Button
          variant="destructive"
          @click="chat.confirmDeleteChat"
        >
          {{ chat.t('commonDelete') }}
        </Button>
      </template>
    </ResponsiveDialog>

    <ResponsiveDialog v-model:open="chat.showRenameChatDialog">
      <template #title>{{ chat.t('renameChat') }}</template>
      <Input
        v-model="chat.pendingRenameChatTitle"
        :placeholder="chat.t('renameChatPlaceholder')"
        maxlength="80"
        @keydown.enter="chat.confirmRenameChat"
      />
      <template #footer>
        <Button
          variant="outline"
          @click="chat.cancelRenameChat"
        >
          {{ chat.t('commonCancel') }}
        </Button>
        <Button
          :disabled="!chat.pendingRenameChatTitle.trim()"
          @click="chat.confirmRenameChat"
        >
          {{ chat.t('commonSave') }}
        </Button>
      </template>
    </ResponsiveDialog>
  </div>
</template>

<style scoped>
  .chat-enter-active,
  .chat-leave-active {
    transition: all 0.3s ease;
  }

  .chat-enter-from,
  .chat-leave-to {
    opacity: 0;
    transform: translateY(20px) scale(0.95);
  }

  .landing-layout-shift-enter-active,
  .landing-layout-shift-leave-active {
    transition:
      opacity 0.28s ease,
      transform 0.28s ease;
  }

  .landing-layout-shift-enter-from {
    opacity: 0;
    transform: translateY(-12px);
  }

  .landing-layout-shift-leave-to {
    opacity: 0;
    transform: translateY(26px);
  }

  :deep([data-radix-card-root]) {
    user-select: none;
  }

  :deep([data-radix-card-root].resizing) {
    user-select: none;
  }

  :deep([data-radix-card-root].resizing *) {
    pointer-events: none;
  }
</style>
