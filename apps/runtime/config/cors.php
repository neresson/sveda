<?php

return [

    'paths' => ['veda/*', 'up'],

    'allowed_methods' => ['*'],

    'allowed_origins' => array_values(array_filter(array_map('trim', explode(',', (string) env('VEDA_CORS_ORIGINS', 'http://localhost:8001'))))),

    'allowed_origins_patterns' => [],

    'allowed_headers' => ['*'],

    'exposed_headers' => [],

    'max_age' => 0,

    'supports_credentials' => false,

];
