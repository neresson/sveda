<?php

namespace Veda\Laravel;

use Illuminate\Contracts\Events\Dispatcher;
use Illuminate\Contracts\Http\Kernel as HttpKernel;
use Illuminate\Foundation\Application;
use Illuminate\Support\Facades\Route;
use Illuminate\Support\ServiceProvider;
use Veda\Laravel\Http\Middleware\VedaHandleCors;
use Laravel\Ai\Ai;
use Laravel\Ai\Events\InvokingTool;
use Laravel\Ai\Events\ToolInvoked;
use Veda\Laravel\Listeners\BroadcastVedaToolActivity;
use Veda\Laravel\Providers\VedaDeepSeekProvider;
use Veda\Laravel\Providers\VedaOpenAiProvider;
use Veda\Laravel\Providers\YandexTextProvider;

class VedaServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->mergeConfigFrom(__DIR__.'/../config/veda.php', 'veda');

        $this->app->singleton(VedaManager::class, fn () => new VedaManager);
        $this->app->alias(VedaManager::class, 'veda');
        $this->app->singleton(Services\HostMcpCredentialStore::class);
        $this->app->scoped(Services\HostMcpToolGateway::class);
    }

    public function boot(): void
    {
        $this->loadMigrationsFrom(__DIR__.'/../database/migrations');

        $this->registerCorsMiddleware();
        $this->registerAiProviders();
        $this->registerRoutes();
        $this->registerBroadcasting();
        $this->registerEventListeners();

        if ($this->app->runningInConsole()) {
            $this->publishes([
                __DIR__.'/../config/veda.php' => config_path('veda.php'),
            ], 'veda-config');
        }
    }

    protected function registerCorsMiddleware(): void
    {
        $kernel = $this->app->make(HttpKernel::class);
        if (method_exists($kernel, 'prependMiddleware')) {
            $kernel->prependMiddleware(VedaHandleCors::class);
        }
    }

    protected function registerAiProviders(): void
    {
        Ai::extend('veda-openai', function (Application $app, array $config) {
            return new VedaOpenAiProvider($config, $app->make(Dispatcher::class));
        });

        Ai::extend('veda-deepseek', function (Application $app, array $config) {
            return new VedaDeepSeekProvider($config, $app->make(Dispatcher::class));
        });

        Ai::extend('veda-yandex', function (Application $app, array $config) {
            return new YandexTextProvider($config, $app->make(Dispatcher::class));
        });

        $providers = (array) config('veda.providers', []);
        if ($providers !== []) {
            $existing = (array) config('ai.providers', []);
            config(['ai.providers' => array_merge($existing, $providers)]);
        }

        $failover = array_values(array_filter((array) config('veda.failover', [])));
        if ($failover !== []) {
            config(['ai.failover' => $failover]);
        }

        $defaultProvider = config('veda.provider');
        if (is_string($defaultProvider) && $defaultProvider !== '') {
            config(['ai.default' => $defaultProvider]);
        }
    }

    protected function registerRoutes(): void
    {
        Route::group([
            'prefix' => config('veda.prefix', 'veda'),
            'middleware' => config('veda.middleware', ['web']),
        ], function () {
            $this->loadRoutesFrom(__DIR__.'/../routes/web.php');
        });
    }

    protected function registerBroadcasting(): void
    {
        if (! (bool) config('veda.broadcasting.enabled', true)) {
            return;
        }

        $this->loadRoutesFrom(__DIR__.'/../routes/channels.php');
    }

    protected function registerEventListeners(): void
    {
        if (! (bool) config('veda.broadcasting.enabled', true)) {
            return;
        }

        $events = $this->app->make(Dispatcher::class);
        $events->listen(InvokingTool::class, [BroadcastVedaToolActivity::class, 'handle']);
        $events->listen(ToolInvoked::class, [BroadcastVedaToolActivity::class, 'handle']);
    }
}
