#!/usr/bin/env bash
set -euo pipefail
SDK="${1:?sdk id required}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WORKSPACE="$(cd "$ROOT/.." && pwd)"
SDK_DIR="${SDK_DIR:-${WORKSPACE}/sdk}"
if [[ ! -d "$SDK_DIR" ]]; then
  case "$SDK" in
    android) SDK_DIR="${WORKSPACE}/sveda-android" ;;
    laravel) SDK_DIR="${WORKSPACE}/sveda-laravel-sdk" ;;
    *) SDK_DIR="${WORKSPACE}/sveda-${SDK}-sdk" ;;
  esac
fi
if [[ ! -d "$SDK_DIR" ]]; then
  echo "sdk directory not found for ${SDK}: ${SDK_DIR}" >&2
  exit 1
fi

echo "sdk-compat: running ${SDK} in ${SDK_DIR}"

case "$SDK" in
  php)
    (cd "$SDK_DIR" && composer install --no-interaction && composer test:contract && composer test:live)
    ;;
  python)
    (cd "$SDK_DIR" && python -m pip install -e . && python -m unittest discover -s tests -v)
    ;;
  node)
    (cd "$SDK_DIR" && npm test && npm run test:live)
    ;;
  go)
    (cd "$SDK_DIR" && go test ./... && go test -tags=live ./...)
    ;;
  java)
    (cd "$SDK_DIR" && mvn -q test)
    ;;
  dotnet)
    (cd "$SDK_DIR" && dotnet test tests/Sveda.Client.Tests/Sveda.Client.Tests.csproj)
    ;;
  ruby)
    (cd "$SDK_DIR" && bundle install && bundle exec rake test)
    ;;
  laravel)
    PHP_SDK_DIR="${WORKSPACE}/sveda-php-sdk"
    if [[ ! -d "$PHP_SDK_DIR" ]]; then
      PHP_SDK_DIR="${WORKSPACE}/sdk/../sveda-php-sdk"
    fi
    if [[ ! -d "$PHP_SDK_DIR" ]]; then
      git clone --depth 1 https://github.com/neresson/sveda-php-sdk.git "${WORKSPACE}/sveda-php-sdk"
      PHP_SDK_DIR="${WORKSPACE}/sveda-php-sdk"
    fi
    (cd "$SDK_DIR" && composer install --no-interaction && ./vendor/bin/phpunit)
    ;;
  android)
    (cd "$SDK_DIR" && chmod +x ./gradlew && ./gradlew :core:test --no-daemon)
    ;;
  *)
    echo "unknown sdk: ${SDK}" >&2
    exit 1
    ;;
esac
