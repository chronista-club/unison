/**
 * Lightweight connection handling for MCP servers in TypeScript
 */

import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { SSEClientTransport } from '@modelcontextprotocol/sdk/client/sse.js';

export interface Tool {
  name: string;
  description?: string;
  inputSchema: any;
}

export interface MCPConnectionOptions {
  transport: 'stdio' | 'sse' | 'http';
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  url?: string;
  headers?: Record<string, string>;
}

export abstract class MCPConnection {
  protected client: Client | null = null;

  abstract connect(): Promise<void>;

  async disconnect(): Promise<void> {
    if (this.client) {
      await this.client.close();
      this.client = null;
    }
  }

  async listTools(): Promise<Tool[]> {
    if (!this.client) {
      throw new Error('Client not connected');
    }

    const response = await this.client.listTools();
    return response.tools;
  }

  async callTool(toolName: string, args: Record<string, any>): Promise<any> {
    if (!this.client) {
      throw new Error('Client not connected');
    }

    const result = await this.client.callTool({
      name: toolName,
      arguments: args
    });

    // Extract content from the result
    if (result.content && result.content.length > 0) {
      // If content is an array with text items, join them
      const textContent = result.content
        .filter((item: any) => item.type === 'text')
        .map((item: any) => item.text)
        .join('\n');

      // Try to parse as JSON if possible
      try {
        return JSON.parse(textContent);
      } catch {
        return textContent;
      }
    }

    return result;
  }
}

export class MCPConnectionStdio extends MCPConnection {
  private command: string;
  private args: string[];
  private env?: Record<string, string>;

  constructor(command: string, args: string[] = [], env?: Record<string, string>) {
    super();
    this.command = command;
    this.args = args;
    this.env = env;
  }

  async connect(): Promise<void> {
    const transport = new StdioClientTransport({
      command: this.command,
      args: this.args,
      env: this.env
    });

    this.client = new Client({
      name: 'mcp-evaluation-client',
      version: '1.0.0'
    }, {
      capabilities: {}
    });

    await this.client.connect(transport);
  }
}

export class MCPConnectionSSE extends MCPConnection {
  private url: string;
  private headers: Record<string, string>;

  constructor(url: string, headers: Record<string, string> = {}) {
    super();
    this.url = url;
    this.headers = headers;
  }

  async connect(): Promise<void> {
    const transport = new SSEClientTransport(
      new URL(this.url),
      { headers: this.headers }
    );

    this.client = new Client({
      name: 'mcp-evaluation-client',
      version: '1.0.0'
    }, {
      capabilities: {}
    });

    await this.client.connect(transport);
  }
}

export class MCPConnectionHTTP extends MCPConnection {
  private url: string;
  private headers: Record<string, string>;

  constructor(url: string, headers: Record<string, string> = {}) {
    super();
    this.url = url;
    this.headers = headers;
  }

  async connect(): Promise<void> {
    // For HTTP transport, we'll use fetch-based implementation
    // Note: The MCP SDK might not have built-in HTTP transport,
    // so this is a simplified implementation
    throw new Error('HTTP transport not yet implemented in TypeScript SDK');
  }
}

export function createConnection(options: MCPConnectionOptions): MCPConnection {
  const { transport } = options;

  switch (transport.toLowerCase()) {
    case 'stdio':
      if (!options.command) {
        throw new Error('Command is required for stdio transport');
      }
      return new MCPConnectionStdio(
        options.command,
        options.args,
        options.env
      );

    case 'sse':
      if (!options.url) {
        throw new Error('URL is required for SSE transport');
      }
      return new MCPConnectionSSE(
        options.url,
        options.headers
      );

    case 'http':
      if (!options.url) {
        throw new Error('URL is required for HTTP transport');
      }
      return new MCPConnectionHTTP(
        options.url,
        options.headers
      );

    default:
      throw new Error(`Unsupported transport type: ${transport}. Use 'stdio', 'sse', or 'http'`);
  }
}