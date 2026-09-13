<?php

namespace Veda\Laravel\Services;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Http\Request;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;

class EmbedChatScope
{
    public function __construct(
        public ?Authenticatable $user,
        public bool $embedVisitorMode,
        public string $visitorId = '',
        public int|string|null $userId = null,
    ) {
        if ($this->userId === null && $this->user !== null) {
            $this->userId = $this->user->getAuthIdentifier();
        }
    }

    public static function fromRequest(Request $request): self
    {
        $user = $request->user();
        $embedVisitorMode = $request->attributes->get(VedaEmbedAuth::ATTRIBUTE_EMBED_GUEST) === true;
        $visitorId = '';

        if ($embedVisitorMode) {
            $visitorId = (string) ($request->attributes->get(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID) ?? '');
        }

        return new self($user, $embedVisitorMode, $visitorId);
    }

    public function applyToHistoryQuery(Builder $query): Builder
    {
        if ($this->embedVisitorMode) {
            return $query->where('visitor_id', $this->visitorId);
        }

        return $query
            ->where('user_id', $this->userId)
            ->where('visitor_id', '');
    }

    /**
     * @return array<string, int|string>
     */
    public function uniqueKeysForChat(string $chatId): array
    {
        if ($this->embedVisitorMode) {
            return [
                'user_id' => $this->userId ?? 0,
                'visitor_id' => $this->visitorId,
                'chat_id' => $chatId,
            ];
        }

        return [
            'user_id' => $this->userId ?? 0,
            'visitor_id' => '',
            'chat_id' => $chatId,
        ];
    }

    /**
     * @return array<string, mixed>
     */
    public function attributesForCreate(string $chatId): array
    {
        return $this->uniqueKeysForChat($chatId);
    }
}
