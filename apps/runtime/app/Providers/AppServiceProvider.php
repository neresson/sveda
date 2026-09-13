<?php

namespace App\Providers;

use App\Models\User;
use Illuminate\Http\Request;
use Illuminate\Support\ServiceProvider;
use Illuminate\Support\Str;

class AppServiceProvider extends ServiceProvider
{
    public function register(): void
    {
    }

    public function boot(): void
    {
        config([
            'veda.embed.user_resolver' => function (Request $request, string $visitorId): User {
                $email = 'guest+'.substr(hash('sha256', $visitorId), 0, 24).'@veda.local';

                return User::query()->firstOrCreate(
                    ['email' => $email],
                    [
                        'name' => 'Veda guest',
                        'password' => Str::password(32),
                    ]
                );
            },
        ]);
    }
}
