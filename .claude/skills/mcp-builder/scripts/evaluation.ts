/**
 * MCP Server Evaluation Harness in TypeScript
 *
 * This script evaluates MCP servers by running test questions against them using Claude.
 */

import { readFileSync, writeFileSync } from 'fs';
import { parseString } from 'xml2js';
import { promisify } from 'util';
import { Command } from 'commander';
import Anthropic from '@anthropic-ai/sdk';
import type { MessageParam, Tool as AnthropicTool, ToolUseBlock } from '@anthropic-ai/sdk/resources/messages.js';
import { createConnection, MCPConnection, Tool } from './connections.js';

const parseXML = promisify(parseString);

const EVALUATION_PROMPT = `You are an AI assistant with access to tools.

When given a task, you MUST:
1. Use the available tools to complete the task
2. Provide summary of each step in your approach, wrapped in <summary> tags
3. Provide feedback on the tools provided, wrapped in <feedback> tags
4. Provide your final response, wrapped in <response> tags

Summary Requirements:
- In your <summary> tags, you must explain:
  - The steps you took to complete the task
  - Which tools you used, in what order, and why
  - The inputs you provided to each tool
  - The outputs you received from each tool
  - A summary for how you arrived at the response

Feedback Requirements:
- In your <feedback> tags, provide constructive feedback on the tools:
  - Comment on tool names: Are they clear and descriptive?
  - Comment on input parameters: Are they well-documented? Are required vs optional parameters clear?
  - Comment on descriptions: Do they accurately describe what the tool does?
  - Comment on any errors encountered during tool usage: Did the tool fail to execute? Did the tool return too many tokens?
  - Identify specific areas for improvement and explain WHY they would help
  - Be specific and actionable in your suggestions

Response Requirements:
- Your response should be concise and directly address what was asked
- Always wrap your final response in <response> tags
- If you cannot solve the task return <response>NOT_FOUND</response>
- For numeric responses, provide just the number
- For IDs, provide just the ID
- For names or text, provide the exact text requested
- Your response should go last`;

interface QAPair {
  question: string;
  answer: string;
}

interface ToolMetrics {
  [toolName: string]: {
    count: number;
    durations: number[];
  };
}

interface EvaluationResult {
  question: string;
  expected: string;
  actual: string | null;
  score: number;
  totalDuration: number;
  toolCalls: ToolMetrics;
  numToolCalls: number;
  summary: string | null;
  feedback: string | null;
}

async function parseEvaluationFile(filePath: string): Promise<QAPair[]> {
  try {
    const xmlContent = readFileSync(filePath, 'utf-8');
    const result = await parseXML(xmlContent);
    const evaluations: QAPair[] = [];

    if (result.evaluation && result.evaluation.qa_pair) {
      const qaPairs = Array.isArray(result.evaluation.qa_pair)
        ? result.evaluation.qa_pair
        : [result.evaluation.qa_pair];

      for (const qaPair of qaPairs) {
        if (qaPair.question && qaPair.answer) {
          evaluations.push({
            question: (qaPair.question[0] || '').trim(),
            answer: (qaPair.answer[0] || '').trim(),
          });
        }
      }
    }

    return evaluations;
  } catch (error) {
    console.error(`Error parsing evaluation file ${filePath}:`, error);
    return [];
  }
}

function extractXmlContent(text: string, tag: string): string | null {
  const pattern = new RegExp(`<${tag}>(.*?)</${tag}>`, 's');
  const matches = text.match(pattern);
  return matches ? matches[1].trim() : null;
}

function convertMCPToolsToAnthropic(tools: Tool[]): AnthropicTool[] {
  return tools.map(tool => ({
    name: tool.name,
    description: tool.description || '',
    input_schema: tool.inputSchema || {
      type: 'object',
      properties: {},
      required: []
    }
  }));
}

async function agentLoop(
  client: Anthropic,
  model: string,
  question: string,
  tools: AnthropicTool[],
  connection: MCPConnection
): Promise<[string, ToolMetrics]> {
  const messages: MessageParam[] = [
    { role: 'user', content: question }
  ];

  let response = await client.messages.create({
    model: model,
    max_tokens: 4096,
    system: EVALUATION_PROMPT,
    messages: messages,
    tools: tools
  });

  messages.push({
    role: 'assistant',
    content: response.content
  });

  const toolMetrics: ToolMetrics = {};

  while (response.stop_reason === 'tool_use') {
    const toolUseBlock = response.content.find(
      (block): block is ToolUseBlock => block.type === 'tool_use'
    );

    if (!toolUseBlock) break;

    const toolName = toolUseBlock.name;
    const toolInput = toolUseBlock.input as Record<string, any>;

    const toolStartTime = Date.now();
    let toolResponse: string;

    try {
      const result = await connection.callTool(toolName, toolInput);
      toolResponse = typeof result === 'object'
        ? JSON.stringify(result)
        : String(result);
    } catch (error) {
      toolResponse = `Error executing tool ${toolName}: ${error}\n`;
      if (error instanceof Error && error.stack) {
        toolResponse += error.stack;
      }
    }

    const toolDuration = (Date.now() - toolStartTime) / 1000;

    if (!toolMetrics[toolName]) {
      toolMetrics[toolName] = { count: 0, durations: [] };
    }
    toolMetrics[toolName].count++;
    toolMetrics[toolName].durations.push(toolDuration);

    messages.push({
      role: 'user',
      content: [{
        type: 'tool_result',
        tool_use_id: toolUseBlock.id,
        content: toolResponse
      }]
    });

    response = await client.messages.create({
      model: model,
      max_tokens: 4096,
      system: EVALUATION_PROMPT,
      messages: messages,
      tools: tools
    });

    messages.push({
      role: 'assistant',
      content: response.content
    });
  }

  const responseText = response.content
    .filter((block): block is { type: 'text'; text: string } =>
      block.type === 'text'
    )
    .map(block => block.text)
    .join('\n');

  return [responseText, toolMetrics];
}

async function evaluateSingleTask(
  client: Anthropic,
  model: string,
  qaPair: QAPair,
  tools: AnthropicTool[],
  connection: MCPConnection,
  taskIndex: number
): Promise<EvaluationResult> {
  const startTime = Date.now();

  console.log(`Task ${taskIndex + 1}: Running task with question: ${qaPair.question}`);

  const [response, toolMetrics] = await agentLoop(
    client,
    model,
    qaPair.question,
    tools,
    connection
  );

  const responseValue = extractXmlContent(response, 'response');
  const summary = extractXmlContent(response, 'summary');
  const feedback = extractXmlContent(response, 'feedback');

  const durationSeconds = (Date.now() - startTime) / 1000;

  return {
    question: qaPair.question,
    expected: qaPair.answer,
    actual: responseValue,
    score: responseValue === qaPair.answer ? 1 : 0,
    totalDuration: durationSeconds,
    toolCalls: toolMetrics,
    numToolCalls: Object.values(toolMetrics).reduce(
      (sum, metrics) => sum + metrics.durations.length,
      0
    ),
    summary: summary,
    feedback: feedback
  };
}

function generateReport(results: EvaluationResult[], qaPairs: QAPair[]): string {
  const correct = results.reduce((sum, r) => sum + r.score, 0);
  const accuracy = results.length > 0 ? (correct / results.length) * 100 : 0;
  const avgDuration = results.length > 0
    ? results.reduce((sum, r) => sum + r.totalDuration, 0) / results.length
    : 0;
  const avgToolCalls = results.length > 0
    ? results.reduce((sum, r) => sum + r.numToolCalls, 0) / results.length
    : 0;
  const totalToolCalls = results.reduce((sum, r) => sum + r.numToolCalls, 0);

  let report = `
# Evaluation Report

## Summary

- **Accuracy**: ${correct}/${results.length} (${accuracy.toFixed(1)}%)
- **Average Task Duration**: ${avgDuration.toFixed(2)}s
- **Average Tool Calls per Task**: ${avgToolCalls.toFixed(2)}
- **Total Tool Calls**: ${totalToolCalls}

---
`;

  results.forEach((result, i) => {
    report += `
### Task ${i + 1}

**Question**: ${qaPairs[i].question}
**Ground Truth Answer**: \`${qaPairs[i].answer}\`
**Actual Answer**: \`${result.actual || 'N/A'}\`
**Correct**: ${result.score ? '✅' : '❌'}
**Duration**: ${result.totalDuration.toFixed(2)}s
**Tool Calls**: ${JSON.stringify(result.toolCalls, null, 2)}

**Summary**
${result.summary || 'N/A'}

**Feedback**
${result.feedback || 'N/A'}

---
`;
  });

  return report;
}

async function runEvaluation(
  evalPath: string,
  connection: MCPConnection,
  model: string = 'claude-3-5-sonnet-20241022'
): Promise<string> {
  console.log('🚀 Starting Evaluation');

  const client = new Anthropic({
    apiKey: process.env.ANTHROPIC_API_KEY
  });

  const mcpTools = await connection.listTools();
  const anthropicTools = convertMCPToolsToAnthropic(mcpTools);
  console.log(`📋 Loaded ${mcpTools.length} tools from MCP server`);

  const qaPairs = await parseEvaluationFile(evalPath);
  console.log(`📋 Loaded ${qaPairs.length} evaluation tasks`);

  const results: EvaluationResult[] = [];
  for (let i = 0; i < qaPairs.length; i++) {
    console.log(`Processing task ${i + 1}/${qaPairs.length}`);
    const result = await evaluateSingleTask(
      client,
      model,
      qaPairs[i],
      anthropicTools,
      connection,
      i
    );
    results.push(result);
  }

  return generateReport(results, qaPairs);
}

function parseHeaders(headers: string[]): Record<string, string> {
  const result: Record<string, string> = {};

  for (const header of headers) {
    const colonIndex = header.indexOf(':');
    if (colonIndex > 0) {
      const key = header.substring(0, colonIndex).trim();
      const value = header.substring(colonIndex + 1).trim();
      result[key] = value;
    } else {
      console.warn(`Warning: Ignoring malformed header: ${header}`);
    }
  }

  return result;
}

function parseEnvVars(envVars: string[]): Record<string, string> {
  const result: Record<string, string> = {};

  for (const envVar of envVars) {
    const equalsIndex = envVar.indexOf('=');
    if (equalsIndex > 0) {
      const key = envVar.substring(0, equalsIndex).trim();
      const value = envVar.substring(equalsIndex + 1).trim();
      result[key] = value;
    } else {
      console.warn(`Warning: Ignoring malformed environment variable: ${envVar}`);
    }
  }

  return result;
}

async function main() {
  const program = new Command();

  program
    .name('evaluation')
    .description('Evaluate MCP servers using test questions')
    .version('1.0.0')
    .argument('<eval-file>', 'Path to evaluation XML file')
    .option('-t, --transport <type>', 'Transport type', 'stdio')
    .option('-m, --model <model>', 'Claude model to use', 'claude-3-5-sonnet-20241022')
    .option('-c, --command <command>', 'Command to run MCP server (stdio only)')
    .option('-a, --args <args...>', 'Arguments for the command (stdio only)')
    .option('-e, --env <env...>', 'Environment variables in KEY=VALUE format (stdio only)')
    .option('-u, --url <url>', 'MCP server URL (sse/http only)')
    .option('-H, --header <headers...>', 'HTTP headers in "Key: Value" format (sse/http only)')
    .option('-o, --output <file>', 'Output file for evaluation report (default: stdout)')
    .addHelpText('after', `
Examples:
  # Evaluate a local stdio MCP server
  node evaluation.js -t stdio -c python -a my_server.py eval.xml

  # Evaluate an SSE MCP server
  node evaluation.js -t sse -u https://example.com/mcp -H "Authorization: Bearer token" eval.xml

  # Evaluate with custom model
  node evaluation.js -t stdio -c node -a server.js -m claude-3-opus-20240229 eval.xml
    `);

  program.parse();

  const options = program.opts();
  const evalFile = program.args[0];

  // Validate API key
  if (!process.env.ANTHROPIC_API_KEY) {
    console.error('Error: ANTHROPIC_API_KEY environment variable is required');
    process.exit(1);
  }

  // Parse headers and environment variables
  const headers = options.header ? parseHeaders(options.header) : undefined;
  const env = options.env ? parseEnvVars(options.env) : undefined;

  // Create connection
  const connection = createConnection({
    transport: options.transport,
    command: options.command,
    args: options.args,
    env: env,
    url: options.url,
    headers: headers
  });

  console.log(`🔗 Connecting to MCP server via ${options.transport}...`);

  try {
    await connection.connect();
    console.log('✅ Connected successfully');

    const report = await runEvaluation(evalFile, connection, options.model);

    if (options.output) {
      writeFileSync(options.output, report);
      console.log(`\n✅ Report saved to ${options.output}`);
    } else {
      console.log('\n' + report);
    }
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  } finally {
    await connection.disconnect();
  }
}

// Run the main function
// Bun and Node.js compatible entry point check
declare const Bun: any;
const isMainModule = import.meta.url === `file://${process.argv[1]}` ||
                     (typeof Bun !== 'undefined' && import.meta.main);

if (isMainModule) {
  main().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}