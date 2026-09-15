<?php

namespace Veda\Laravel\Mcp;

use Laravel\Mcp\Client\Transport\StdioTransport;
use Laravel\Mcp\Exceptions\ClientException;
use Symfony\Component\Process\Exception\ExceptionInterface;
use Symfony\Component\Process\InputStream;
use Symfony\Component\Process\Process;

class EnvStdioTransport extends StdioTransport
{
    /**
     * @param  array<int, string>  $args
     * @param  array<string, string>  $env
     */
    public function __construct(
        string $command,
        array $args = [],
        protected array $env = [],
        protected string $cwd = '',
    ) {
        parent::__construct($command, $args);
    }

    public function connect(): void
    {
        if ($this->process?->isRunning()) {
            return;
        }

        $this->input = new InputStream;
        $this->process = new Process(
            [$this->command, ...$this->args],
            $this->cwd !== '' ? $this->cwd : null,
        );
        if ($this->env !== []) {
            $this->process->setEnv($this->env);
        }
        $this->process->setInput($this->input);
        $this->process->setTimeout(null);

        try {
            $this->process->start();
        } catch (ExceptionInterface) {
            throw new ClientException("Failed to start process [{$this->command}]. Make sure the command exists.");
        }
    }
}
