<?php

return [
    'prefix' => env('VEDA_ROUTE_PREFIX', 'veda'),

    'middleware' => array_values(array_filter(array_map('trim', explode(',', (string) env('VEDA_MIDDLEWARE', 'api'))))),

    'provider' => env('VEDA_PROVIDER', 'veda-openai'),

    'model' => env('VEDA_MODEL'),

    'failover' => array_values(array_filter(explode(',', (string) env('VEDA_FAILOVER', 'veda-openai')))),

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
    ],

    'providers' => [
        'veda-openai' => [
            'driver' => 'veda-openai',
            'key' => env('OPENAI_API_KEY'),
            'url' => env('OPENAI_URL', 'https://api.openai.com/v1'),
        ],
        'veda-deepseek' => [
            'driver' => 'veda-deepseek',
            'key' => env('DEEPSEEK_API_KEY'),
        ],
        'veda-yandex' => [
            'driver' => 'veda-yandex',
            'key' => env('VEDA_YANDEX_API_KEY', ''),
            'url' => env('VEDA_YANDEX_CHAT_ENDPOINT', 'https://ai.api.cloud.yandex.net/v1/responses'),
            'folder_id' => env('VEDA_YANDEX_FOLDER_ID', ''),
            'use_iam_bearer' => filter_var(env('VEDA_YANDEX_USE_IAM_BEARER', false), FILTER_VALIDATE_BOOLEAN),
            'embedding_endpoint' => env('VEDA_YANDEX_EMBEDDING_ENDPOINT', 'https://llm.api.cloud.yandex.net/foundationModels/v1/textEmbedding'),
            'embedding_model_uri' => env('VEDA_YANDEX_EMBEDDING_MODEL_URI', ''),
            'embedding_query_model_uri' => env('VEDA_YANDEX_EMBEDDING_QUERY_MODEL_URI', ''),
            'models' => [
                'text' => [
                    'default' => env('VEDA_YANDEX_TEXT_MODEL', 'qwen3.6-35b-a3b/latest'),
                    'cheapest' => env('VEDA_YANDEX_TEXT_MODEL', 'qwen3.6-35b-a3b/latest'),
                    'smartest' => env('VEDA_YANDEX_TEXT_MODEL', 'qwen3.6-35b-a3b/latest'),
                ],
                'embedding' => [
                    'default' => env('VEDA_YANDEX_EMBEDDING_MODEL', 'text-search-doc/latest'),
                    'dimensions' => (int) env('VEDA_YANDEX_EMBEDDING_DIMENSIONS', 256),
                ],
            ],
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
        'allowed_origins' => array_values(array_filter(array_map('trim', explode(',', (string) env('VEDA_CORS_ORIGINS', 'http://localhost:8001'))))),
        'allowed_headers' => [
            'Accept',
            'Authorization',
            'Content-Type',
            'X-Requested-With',
            'X-Veda-Embed-Token',
            'X-Veda-Host-Key',
            'X-Veda-Protocol',
        ],
        'allowed_methods' => ['GET', 'POST', 'PATCH', 'DELETE', 'OPTIONS'],
    ],

    'tool_defer' => [
        'enabled' => filter_var(env('VEDA_TOOL_DEFER_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
        'min_pool' => (int) env('VEDA_TOOL_DEFER_MIN_POOL', 14),
        'always_loaded' => array_values(array_filter(array_map('trim', explode(',', (string) env('VEDA_TOOL_DEFER_ALWAYS_LOADED', 'spawn_tasks'))))),
    ],

    'tool_catalog' => [
        'embeddings_enabled' => filter_var(env('VEDA_TOOL_CATALOG_EMBEDDINGS_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
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

    'vision_markers' => ['yandex', 'timeweb', 'gemini', 'qwen'],

    'safety_tokens_per_request' => (int) env('VEDA_SAFETY_TOKENS_PER_REQUEST', 40000),

    'compaction' => [
        'enabled' => filter_var(env('VEDA_COMPACTION_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
        'transcript_enabled' => filter_var(env('VEDA_COMPACTION_TRANSCRIPT_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
        'lock_seconds' => (int) env('VEDA_COMPACTION_LOCK_SECONDS', 180),
        'min_messages' => (int) env('VEDA_COMPACTION_MIN_MESSAGES', 40),
        'keep_tail_messages' => (int) env('VEDA_COMPACTION_KEEP_TAIL_MESSAGES', 20),
        'provider' => env('VEDA_COMPACTION_PROVIDER'),
        'model' => env('VEDA_COMPACTION_MODEL'),
        'disk' => env('VEDA_COMPACTION_DISK', 'local'),
    ],

    'chat_turns' => [
        'enabled' => filter_var(env('VEDA_CHAT_TURNS_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
    ],

    'title_generation' => [
        'enabled' => filter_var(env('VEDA_TITLE_GENERATION_ENABLED', true), FILTER_VALIDATE_BOOLEAN),
        'provider' => env('VEDA_TITLE_PROVIDER'),
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
