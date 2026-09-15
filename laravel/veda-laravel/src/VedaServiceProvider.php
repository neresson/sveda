<?php

namespace Veda\Laravel;

use Illuminate\Contracts\Events\Dispatcher;
use Illuminate\Contracts\Http\Kernel as HttpKernel;
use Illuminate\Foundation\Application;
use Illuminate\Support\Facades\Route;
use Illuminate\Support\ServiceProvider;
use Laravel\Ai\Ai;
use Laravel\Ai\Events\InvokingTool;
use Laravel\Ai\Events\ToolInvoked;
use Veda\Laravel\Http\Middleware\VedaApplySettings;
use Veda\Laravel\Http\Middleware\VedaHandleCors;
use Veda\Laravel\Listeners\BroadcastVedaToolActivity;
use Veda\Laravel\Providers\VedaAnthropicProvider;
use Veda\Laravel\Providers\VedaResponsesProvider;
use Veda\Laravel\Services\VedaModelCatalog;
use Veda\Laravel\CodeIndex\CodeIndexSourcesContext;
use Veda\Laravel\Tools\GetCodeSourceOverviewTool;
use Veda\Laravel\Tools\ListCodeSourcesTool;
use Veda\Laravel\Tools\ManageMcpCatalogTool;
use Veda\Laravel\Tools\ReadCodeIndexFileTool;
use Veda\Laravel\Tools\SearchCodeTool;

class VedaServiceProvider extends ServiceProvider
{
    public const NATIVE_TOOLS = [
        ManageMcpCatalogTool::class,
        ListCodeSourcesTool::class,
        GetCodeSourceOverviewTool::class,
        SearchCodeTool::class,
        ReadCodeIndexFileTool::class,
    ];

    public function register(): void
    {
        $this->mergeConfigFrom(__DIR__.'/../config/veda.php', 'veda');

        $this->app->singleton(VedaManager::class, fn () => new VedaManager);
        $this->app->alias(VedaManager::class, 'veda');
        $this->app->singleton(Services\HostMcpCredentialStore::class);
        $this->app->singleton(Services\McpCatalog::class);
        $this->app->scoped(Services\HostMcpToolGateway::class);
        $this->app->singleton(Services\VedaSettingsRepository::class);
        $this->app->singleton(VedaModelCatalog::class);
    }

    public function boot(): void
    {
        $this->loadMigrationsFrom(__DIR__.'/../database/migrations');
        $this->loadViewsFrom(__DIR__.'/../resources/views', 'veda');

        $this->registerCorsMiddleware();
        $this->registerSettingsMiddleware();
        $this->registerAiProviders();
        $this->registerNativeTools();
        $this->registerRoutes();
        $this->registerAdminRoutes();
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
        Ai::extend('veda-responses', function (Application $app, array $config) {
            return new VedaResponsesProvider($config, $app->make(Dispatcher::class));
        });

        Ai::extend('veda-anthropic', function (Application $app, array $config) {
            return new VedaAnthropicProvider($config, $app->make(Dispatcher::class));
        });

        $this->app->make(VedaModelCatalog::class)->registerIntoAi();
    }

    protected function registerNativeTools(): void
    {
        $veda = $this->app->make(VedaManager::class);
        foreach (self::NATIVE_TOOLS as $tool) {
            $veda->toolClass($tool);
        }
        $veda->contextProvider(fn (): string => app(CodeIndexSourcesContext::class)->promptSection());
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

    protected function registerSettingsMiddleware(): void
    {
        $kernel = $this->app->make(HttpKernel::class);
        if (method_exists($kernel, 'prependMiddleware')) {
            $kernel->prependMiddleware(VedaApplySettings::class);
        }
    }

    protected function registerAdminRoutes(): void
    {
        Route::prefix(config('veda.prefix', 'veda'))->group(function () {
            $this->loadRoutesFrom(__DIR__.'/../routes/admin.php');
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
