import { useMemo } from 'react';
import type { TaskSnapshot } from '../components/WorkflowPanel/types';
import type { ChatMessages } from '../services/refact/types';
import { isToolMessage } from '../services/refact/types';

export type PipelineStage = 'PLANNING' | 'GENERATION' | 'COMPILING' | 'FLASH' | 'MONITORING';

export interface WorkflowStatus {
    currentStage: PipelineStage;
    hasError: boolean;
}

/** Must match `WORKSPACE_PLANNING_TOOL_NAMES` in engine `progressbar/mod.rs`. */
export const WORKSPACE_PLANNING_TOOL_NAMES = new Set<string>([
    'strategic_planning',
    'task_list',
    'tree',
    'cat',
    'search_semantic',
    'search_pattern',
    'locate',
    'search_symbol_definition',
    'search_symbol_usages',
    'web',
    'knowledge',
]);

/** Must match `WORKSPACE_GENERATION_TOOL_NAMES` in engine `progressbar/mod.rs`. */
export const WORKSPACE_GENERATION_TOOL_NAMES = new Set<string>([
    'create_textdoc',
    'update_textdoc',
    'update_textdoc_regex',
    'create_knowledge',
    'create_memory_bank',
    'rm',
    'mv',
]);

function toolNameToStage(toolName: string, args?: string): PipelineStage {
    const name = (toolName || '').toLowerCase();
    const argStr = (args || '').toLowerCase();
    const combined = `${name} ${argStr}`;

    // ESP32 / device pipeline (same ordering as Rust `WorkflowNode::from_tool_operation`)
    if (combined.includes('esp32_device') && argStr.includes('monitor')) {
        return 'MONITORING';
    }
    if (combined.includes('esp32_device')) {
        if (
            argStr.includes('detect') ||
            argStr.includes('flash') ||
            argStr.includes('erase')
        ) {
            return 'FLASH';
        }
    }
    if (name === 'esp32_build') {
        if (
            argStr.includes('build') ||
            argStr.includes('clean') ||
            argStr.includes('reconfigure')
        ) {
            return 'COMPILING';
        }
    }
    if (name === 'esp32_project' && argStr.includes('create')) {
        return 'GENERATION';
    }
    if (name === 'esp32_config') {
        return 'GENERATION';
    }
    if (
        name === 'esp32_component' &&
        (argStr.includes('add') || argStr.includes('remove'))
    ) {
        return 'GENERATION';
    }

    if (WORKSPACE_GENERATION_TOOL_NAMES.has(name)) {
        return 'GENERATION';
    }
    if (WORKSPACE_PLANNING_TOOL_NAMES.has(name)) {
        return 'PLANNING';
    }

    // Heuristic fallback for unknown tools / older message shapes
    if (combined.match(/(monitor)/)) {
        return 'MONITORING';
    }
    if (combined.match(/(flash|deploy|upload)/)) {
        return 'FLASH';
    }
    if (combined.match(/(compile|debug|test|cargo|idf\.py)/)) {
        return 'COMPILING';
    }
    if (
        combined.match(
            /(replace_textdoc|write|edit|generat|implement|patch|apply)/,
        )
    ) {
        return 'GENERATION';
    }
    return 'PLANNING';
}

function hasLastToolFailed(messages: ChatMessages): boolean {
    for (let i = messages.length - 1; i >= 0; i--) {
        const msg = messages[i];
        if (isToolMessage(msg) && msg.content?.tool_failed) {
            return true;
        }
        if (msg.role === 'assistant') break;
    }
    return false;
}

function deriveStageFromMessages(messages: ChatMessages): PipelineStage {
    let highestStage: PipelineStage = 'PLANNING';
    const order: PipelineStage[] = ['PLANNING', 'GENERATION', 'COMPILING', 'FLASH', 'MONITORING'];

    for (const msg of messages) {
        if (msg.role !== 'assistant' || !msg.tool_calls?.length) continue;
        for (const tc of msg.tool_calls) {
            const name = tc.function?.name || '';
            const stage = toolNameToStage(name, tc.function?.arguments);
            if (order.indexOf(stage) >= order.indexOf(highestStage)) {
                highestStage = stage;
            }
        }
    }
    return highestStage;
}

export function useWorkflowStatus(
    tasks: TaskSnapshot[],
    messages?: ChatMessages | null
): WorkflowStatus {
    return useMemo(() => {
        let currentStage: PipelineStage = 'PLANNING';

        // Prefer workflow tasks when available
        if (tasks.length > 0) {
            const activeTask = tasks.find(t =>
                t.status === 'in_progress' ||
                t.status === 'pending' ||
                t.status === 'failed'
            );

            if (activeTask) {
                const { description, tool_to_call } = activeTask;
                const descLower = (description || '').toLowerCase();
                const toolLower = (tool_to_call || '').toLowerCase();
                const combinedStr = `${descLower} ${toolLower}`;

                if (combinedStr.match(/(monitor)/))
                    currentStage = 'MONITORING';
                else if (combinedStr.match(/(flash|deploy|upload)/))
                    currentStage = 'FLASH';
                else if (combinedStr.match(/(compile|build|debug|test|cargo)/))
                    currentStage = 'COMPILING';
                else if (WORKSPACE_GENERATION_TOOL_NAMES.has(toolLower))
                    currentStage = 'GENERATION';
                else if (WORKSPACE_PLANNING_TOOL_NAMES.has(toolLower))
                    currentStage = 'PLANNING';
                else if (
                    combinedStr.match(
                        /(write|replace|edit|generat|implement|code)/,
                    )
                )
                    currentStage = 'GENERATION';
            }
        } else if (messages && messages.length > 0) {
            currentStage = deriveStageFromMessages(messages);
        }

        const hasError = messages && messages.length > 0 ? hasLastToolFailed(messages) : false;

        return { currentStage, hasError };
    }, [tasks, messages]);
}
