import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { VEDA_PROTOCOL_VERSION, VEDA_STREAM_EVENTS } from '../src/index.js';

const contractPath = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../contracts/sidecar.v1.json',
);

type SidecarRoute = {
  method: string;
  path: string;
  mode: string;
};

type SidecarContract = {
  version: string;
  prefix: string;
  accept: {
    vedaStream: string;
    sse: string;
  };
  headers: {
    inbound: string[];
    streamResponse: Record<string, string>;
    mcpOutbound: string[];
  };
  cors: {
    allowedHeaders: string[];
    allowedMethods: string[];
  };
  routes: SidecarRoute[];
  streamEvents: string[];
  sseDoneLine: string;
  embedToken: {
    request: string[];
    responseRequired: string[];
    responseOptional: string[];
  };
  message: {
    responseRequired: string[];
  };
  histories: {
    listKey: string;
    detailKey: string;
    summaryRequired: string[];
    detailRequired: string[];
    patchRequest: string[];
    mutationResponseRequired: string[];
  };
  documentsExtract: {
    requestField: string;
    responseKey: string;
    itemRequired: string[];
  };
};

const requiredRoutes: SidecarRoute[] = [
  { method: 'POST', path: '/veda/stream', mode: 'sse' },
  { method: 'POST', path: '/veda/message', mode: 'json' },
  { method: 'GET', path: '/veda/chat-histories', mode: 'json' },
  { method: 'GET', path: '/veda/chat-histories/{chatId}', mode: 'json' },
  { method: 'PATCH', path: '/veda/chat-histories/{chatId}', mode: 'json' },
  { method: 'DELETE', path: '/veda/chat-histories/{chatId}', mode: 'json' },
  { method: 'POST', path: '/veda/documents/extract', mode: 'json' },
  { method: 'GET', path: '/veda/embed/config', mode: 'json' },
  { method: 'POST', path: '/veda/embed/token', mode: 'json' },
];

describe('sidecar HTTP contract v1', () => {
  it('has a sidecar.v1.json fixture', () => {
    expect(existsSync(contractPath)).toBe(true);
  });

  it('locks the host-facing HTTP surface', () => {
    const contract = JSON.parse(readFileSync(contractPath, 'utf8')) as SidecarContract;

    expect(contract.version).toBe(VEDA_PROTOCOL_VERSION);
    expect(contract.prefix).toBe('/veda');
    expect(contract.accept.vedaStream).toBe('application/vnd.veda.stream+json');
    expect(contract.accept.sse).toBe('text/event-stream');
    expect(contract.sseDoneLine).toBe('data: [DONE]');
    expect(contract.streamEvents).toEqual([...VEDA_STREAM_EVENTS]);
    expect(contract.headers.streamResponse['X-Veda-Protocol-Version']).toBe(VEDA_PROTOCOL_VERSION);
    expect(contract.headers.inbound).toEqual(
      expect.arrayContaining([
        'Authorization',
        'X-Veda-Embed-Token',
        'X-Veda-Host-Key',
        'X-Veda-Admin-Key',
        'X-Veda-Protocol',
        'Accept',
        'Content-Type',
      ]),
    );
    expect(contract.headers.mcpOutbound).toEqual(
      expect.arrayContaining(['X-Veda-Page-Context', 'X-Veda-Chat-Id']),
    );
    expect(contract.cors.allowedHeaders).toEqual(
      expect.arrayContaining(contract.headers.inbound),
    );
    expect(contract.routes).toEqual(expect.arrayContaining(requiredRoutes));
    expect(contract.embedToken.responseRequired).toEqual(
      expect.arrayContaining(['token', 'visitor_id', 'expires_in']),
    );
    expect(contract.message.responseRequired).toEqual(
      expect.arrayContaining(['explanation', 'tokens_used', 'chat_id']),
    );
    expect(contract.histories.listKey).toBe('histories');
    expect(contract.histories.detailKey).toBe('history');
    expect(contract.histories.summaryRequired).toEqual(
      expect.arrayContaining(['chatId', 'title']),
    );
    expect(contract.histories.detailRequired).toEqual(
      expect.arrayContaining(['chatId', 'messages']),
    );
    expect(contract.documentsExtract.responseKey).toBe('items');
    expect(contract.documentsExtract.itemRequired).toEqual(
      expect.arrayContaining(['filename', 'ok']),
    );
  });
});
