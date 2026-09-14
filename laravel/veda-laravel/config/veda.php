<?php

return [
    'prefix' => env('VEDA_ROUTE_PREFIX', 'veda'),

    'middleware' => array_values(array_filter(array_map('trim', explode(',', (string) env('VEDA_MIDDLEWARE', 'web,auth'))))),

    'default_model' => env('VEDA_DEFAULT_MODEL', env('VEDA_MODEL', 'deepseek-v4-flash-responses')),

    'model' => env('VEDA_MODEL', env('VEDA_DEFAULT_MODEL', 'deepseek-v4-flash-responses')),

    'failover' => array_values(array_filter(explode(',', (string) env('VEDA_FAILOVER', 'deepseek-v4-flash-anthropic')))),

    'stream_timeout' => (int) env('VEDA_STREAM_TIMEOUT', 1800),

    'max_steps' => (int) env('VEDA_AGENT_MAX_STEPS', 30),

    'protocol' => env('VEDA_STREAM_PROTOCOL', 'veda'),

    'user_model' => env('VEDA_USER_MODEL', 'App\\Models\\User'),

    'authorize' => null,

    'tables' => [
        'conversations' => env('VEDA_TABLE_CONVERSATIONS'),
        'messages' => env('VEDA_TABLE_MESSAGES'),
        'chat_histories' => 'veda_chat_histories',
        'chat_turns' => 'veda_chat_turns',
        'chat_compactions' => 'veda_chat_compactions',
        'generations' => 'veda_generations',
        'settings' => 'veda_settings',
    ],

    'admin' => [
        'api_key' => env('VEDA_ADMIN_API_KEY', ''),
    ],

    'welcome_message' => env('VEDA_WELCOME_MESSAGE', ''),

    'system_prompt' => env('VEDA_SYSTEM_PROMPT', ''),

    'deepseek' => [
        'key' => env('DEEPSEEK_API_KEY'),
    ],

    'models' => [
        [
            'id' => 'deepseek-v4-flash-responses',
            'label' => 'DeepSeek V4 Flash (Responses)',
            'protocol' => 'responses',
            'api_model' => 'deepseek-v4-flash',
            'url' => 'https://api.deepseek.com',
            'thinking' => true,
            'vision' => false,
            'preset' => 'deepseek',
            'aliases' => ['deepseek-v4-flash'],
        ],
        [
            'id' => 'deepseek-v4-flash-anthropic',
            'label' => 'DeepSeek V4 Flash (Anthropic)',
            'protocol' => 'anthropic',
            'api_model' => 'deepseek-v4-flash',
            'url' => 'https://api.deepseek.com/anthropic/v1',
            'thinking' => true,
            'vision' => false,
            'preset' => 'deepseek',
        ],
        [
            'id' => 'deepseek-v4-pro',
            'label' => 'DeepSeek V4 Pro',
            'protocol' => 'responses',
            'api_model' => 'deepseek-v4-pro',
            'url' => 'https://api.deepseek.com',
            'thinking' => true,
            'vision' => false,
            'preset' => 'deepseek',
        ],
    ],

    'broadcasting' => [
        'enabled' => env('VEDA_BROADCASTING_ENABLED', true),
        'channel_prefix' => env('VEDA_BROADCAST_CHANNEL_PREFIX', 'veda'),
        'authorize' => null,
    ],

    'embed' => [
        'enabled' => env('VEDA_EMBED_ENABLED', false),
        'write_tools_enabled' => env('VEDA_EMBED_WRITE_TOOLS_ENABLED', false),
        'token_ttl_seconds' => (int) env('VEDA_EMBED_TOKEN_TTL', 3600),
        'throttle' => env('VEDA_EMBED_TOKEN_THROTTLE', '30,1'),
        'host_api_key' => env('VEDA_EMBED_HOST_API_KEY', ''),
        'authorize' => null,
        'user_resolver' => null,
    ],

    'host' => [
        'mcp' => [
            'timeout' => (int) env('VEDA_HOST_MCP_TIMEOUT', 30),
        ],
    ],

    'cors' => [
        'allowed_origins' => array_values(array_filter(array_map('trim', explode(',', (string) env('VEDA_CORS_ORIGINS', ''))))),
        'allowed_headers' => [
            'Accept',
            'Authorization',
            'Content-Type',
            'X-Requested-With',
            'X-Veda-Embed-Token',
            'X-Veda-Host-Key',
            'X-Veda-Admin-Key',
            'X-Veda-Protocol',
        ],
        'allowed_methods' => ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'],
    ],

    'tool_defer' => [
        'enabled' => filter_var(env('VEDA_TOOL_DEFER_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
        'min_pool' => (int) env('VEDA_TOOL_DEFER_MIN_POOL', 14),
        'always_loaded' => array_values(array_filter(array_map('trim', explode(',', (string) env('VEDA_TOOL_DEFER_ALWAYS_LOADED', 'spawn_tasks'))))),
    ],

    'tool_catalog' => [
        'embeddings_enabled' => filter_var(env('VEDA_TOOL_CATALOG_EMBEDDINGS_ENABLED', false), FILTER_VALIDATE_BOOLEAN),
        'embeddings_online_fill' => filter_var(env('VEDA_TOOL_CATALOG_EMBEDDINGS_ONLINE_FILL', false), FILTER_VALIDATE_BOOLEAN),
        'embedding_cache_days' => (int) env('VEDA_TOOL_CATALOG_EMBEDDING_CACHE_DAYS', 90),
        'embedding_keyword_weight' => (float) env('VEDA_TOOL_CATALOG_EMBEDDING_KEYWORD_WEIGHT', 0.25),
        'embedding_min_score' => (float) env('VEDA_TOOL_CATALOG_EMBEDDING_MIN_SCORE', 0.12),
    ],

    'embeddings' => [
        'provider' => env('VEDA_EMBEDDINGS_PROVIDER'),
        'model' => env('VEDA_EMBEDDINGS_MODEL'),
        'dimensions' => (int) env('VEDA_EMBEDDINGS_DIMENSIONS', 256),
        'max_input_chars' => (int) env('VEDA_EMBEDDINGS_MAX_INPUT_CHARS', 2000),
    ],

    'context_max_tokens' => (int) env('VEDA_CONTEXT_MAX_TOKENS', 128000),

    'safety_tokens_per_request' => (int) env('VEDA_SAFETY_TOKENS_PER_REQUEST', 40000),

    'compaction' => [
        'enabled' => filter_var(env('VEDA_COMPACTION_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
        'transcript_enabled' => filter_var(env('VEDA_COMPACTION_TRANSCRIPT_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
        'lock_seconds' => (int) env('VEDA_COMPACTION_LOCK_SECONDS', 180),
        'min_messages' => (int) env('VEDA_COMPACTION_MIN_MESSAGES', 40),
        'keep_tail_messages' => (int) env('VEDA_COMPACTION_KEEP_TAIL_MESSAGES', 20),
        'model' => env('VEDA_COMPACTION_MODEL'),
        'disk' => env('VEDA_COMPACTION_DISK', 'local'),
    ],

    'chat_turns' => [
        'enabled' => filter_var(env('VEDA_CHAT_TURNS_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
    ],

    'title_generation' => [
        'enabled' => filter_var(env('VEDA_TITLE_GENERATION_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
        'model' => env('VEDA_TITLE_MODEL'),
    ],

    'preflight' => [
        'enabled' => filter_var(env('VEDA_PREFLIGHT_ENABLED', false), FILTER_VALIDATE_BOOLEAN),
        'max_parallel' => (int) env('VEDA_PREFLIGHT_MAX_PARALLEL', 4),
    ],

    'tasks' => [
        'max_parallel' => (int) env('VEDA_TASK_MAX_PARALLEL', 4),
        'max_per_turn' => (int) env('VEDA_TASK_MAX_PER_TURN', 6),
        'timeout' => (int) env('VEDA_TASK_TIMEOUT', 300),
    ],

    'documents' => [
        'max_files' => (int) env('VEDA_DOCUMENT_MAX_FILES', 5),
        'max_file_kb' => (int) env('VEDA_DOCUMENT_MAX_FILE_KB', 40960),
        'max_total_chars' => (int) env('VEDA_DOCUMENT_MAX_TOTAL_CHARS', 150000),
    ],

    'prompts' => [
        'path' => null,
    ],
];
