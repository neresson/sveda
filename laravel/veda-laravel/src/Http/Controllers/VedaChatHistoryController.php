<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Veda\Laravel\Models\VedaChatHistory;
use Veda\Laravel\Services\ChatHistorySyncService;
use Veda\Laravel\Services\EmbedChatScope;

class VedaChatHistoryController
{
    public function __construct(
        protected ChatHistorySyncService $syncService,
    ) {}

    protected function historyScope(Request $request): EmbedChatScope
    {
        return EmbedChatScope::fromRequest($request);
    }

    public function index(Request $request): JsonResponse
    {
        $scope = $this->historyScope($request);

        $histories = $scope->applyToHistoryQuery(VedaChatHistory::query())
            ->orderByDesc('updated_at')
            ->get()
            ->map(fn (VedaChatHistory $history) => $this->syncService->transformSummary($history))
            ->values();

        return response()->json([
            'histories' => $histories,
        ]);
    }

    public function show(Request $request, string $chatId): JsonResponse
    {
        $scope = $this->historyScope($request);

        $history = $scope->applyToHistoryQuery(VedaChatHistory::query())
            ->where('chat_id', $chatId)
            ->first();

        if (! $history) {
            return response()->json([
                'message' => 'Chat not found',
            ], 404);
        }

        return response()->json([
            'history' => $this->syncService->transformHistory($history),
        ]);
    }

    public function update(Request $request, string $chatId): JsonResponse
    {
        $validated = $request->validate([
            'title' => 'required|string|max:191',
        ]);

        $scope = $this->historyScope($request);

        $updated = $this->syncService->updateTitle($scope, $chatId, $validated['title']);

        if (! $updated) {
            return response()->json([
                'message' => 'Chat not found',
            ], 404);
        }

        return response()->json([
            'success' => true,
        ]);
    }

    public function destroy(Request $request, string $chatId): JsonResponse
    {
        $scope = $this->historyScope($request);

        $deleted = $this->syncService->deleteOne($scope, $chatId);

        if (! $deleted) {
            return response()->json([
                'message' => 'Chat not found',
            ], 404);
        }

        return response()->json([
            'success' => true,
        ]);
    }
}
