import React from "react";
import { Box, Flex, Text, Badge, Separator } from "@radix-ui/themes";
import styles from "./ContextPayloadSidebar.module.css";

// Assuming we want strict but simple inline types so we don't need to import large interfaces
export interface TokenUsagePanelProps {
    mainAgentUsage: {
        prompt: number;
        completion: number;
        total: number;
        hasData: boolean;
    };
    tokenAnalysis: {
        estimated: number;
    };
    subchatUsage?: {
        prompt_tokens: number;
        completion_tokens: number;
    } | null;
    subchatUsageByTool?: Record<
        string,
        {
            invocations: number;
            prompt_tokens: number;
            completion_tokens: number;
        }
    > | null;
}

export const TokenUsagePanel: React.FC<TokenUsagePanelProps> = ({
    mainAgentUsage,
    tokenAnalysis,
    subchatUsage,
    subchatUsageByTool,
}) => {
    return (
        <Box className={styles.tokenStrip}>
            {/* Main Agent */}
            <Flex direction="column" gap="1" className={styles.tokenStripAgent}>
                <Flex align="center" gap="1">
                    <Badge color="blue" size="1">
                        Main Agent
                    </Badge>
                </Flex>
                {mainAgentUsage.hasData ? (
                    <Flex direction="column" gap="1">
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Total:
                            </Text>
                            <Text size="1" weight="bold">
                                {mainAgentUsage.total.toLocaleString()}
                            </Text>
                        </Flex>
                    </Flex>
                ) : (
                    <Flex direction="column" gap="1">
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Prompt:
                            </Text>
                            <Text size="1" color="gray">
                                -
                            </Text>
                        </Flex>
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Completion:
                            </Text>
                            <Text size="1" color="gray">
                                -
                            </Text>
                        </Flex>
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Total (est.):
                            </Text>
                            <Text size="1" weight="bold" color="gray">
                                ~
                                {Math.max(
                                    0,
                                    tokenAnalysis.estimated -
                                    (subchatUsage
                                        ? subchatUsage.prompt_tokens +
                                        subchatUsage.completion_tokens
                                        : 0)
                                ).toLocaleString()}
                            </Text>
                        </Flex>
                    </Flex>
                )}
            </Flex>

            <Box className={styles.tokenStripDivider} />

            {/* Subchat Agent */}
            <Flex direction="column" gap="1" className={styles.tokenStripAgent}>
                <Flex align="center" gap="1">
                    <Badge color="orange" size="1">
                        Subchat Agent
                    </Badge>
                </Flex>
                {subchatUsage ? (
                    <Flex direction="column" gap="1">
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Prompt:
                            </Text>
                            <Text size="1">
                                {subchatUsage.prompt_tokens.toLocaleString()}
                            </Text>
                        </Flex>
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Completion:
                            </Text>
                            <Text size="1">
                                {subchatUsage.completion_tokens.toLocaleString()}
                            </Text>
                        </Flex>
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Total:
                            </Text>
                            <Text size="1" weight="bold">
                                {(
                                    subchatUsage.prompt_tokens + subchatUsage.completion_tokens
                                ).toLocaleString()}
                            </Text>
                        </Flex>

                        {subchatUsageByTool &&
                            Object.entries(subchatUsageByTool).length > 0 && (
                                <>
                                    <Separator my="2" size="4" />
                                    <Text size="1" weight="bold" color="gray" mb="1">
                                        Per-Tool Telemetry
                                    </Text>
                                    {Object.entries(subchatUsageByTool).map(
                                        ([toolName, usage]) => (
                                            <Box key={toolName} pb="2">
                                                <Flex justify="between" align="center" mb="1">
                                                    <Text
                                                        size="1"
                                                        weight="bold"
                                                        style={{ wordBreak: "break-all" }}
                                                    >
                                                        {toolName}
                                                    </Text>
                                                    <Badge color="gray" size="1" variant="outline">
                                                        {usage.invocations} run(s)
                                                    </Badge>
                                                </Flex>
                                                <Flex justify="between" pl="2">
                                                    <Text size="1" color="gray">
                                                        In (Prompt):
                                                    </Text>
                                                    <Text size="1">
                                                        {usage.prompt_tokens.toLocaleString()}
                                                    </Text>
                                                </Flex>
                                                <Flex justify="between" pl="2">
                                                    <Text size="1" color="gray">
                                                        Out (Completion):
                                                    </Text>
                                                    <Text size="1">
                                                        {usage.completion_tokens.toLocaleString()}
                                                    </Text>
                                                </Flex>
                                                <Flex justify="between" pl="2">
                                                    <Text size="1" color="gray">
                                                        Tool Total:
                                                    </Text>
                                                    <Text size="1" weight="bold">
                                                        {(
                                                            usage.prompt_tokens + usage.completion_tokens
                                                        ).toLocaleString()}
                                                    </Text>
                                                </Flex>
                                            </Box>
                                        )
                                    )}
                                </>
                            )}
                    </Flex>
                ) : (
                    <Flex direction="column" gap="1">
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Prompt:
                            </Text>
                            <Text size="1" color="gray">
                                -
                            </Text>
                        </Flex>
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Completion:
                            </Text>
                            <Text size="1" color="gray">
                                -
                            </Text>
                        </Flex>
                        <Flex justify="between">
                            <Text size="1" color="gray">
                                Total:
                            </Text>
                            <Text size="1" weight="bold" color="gray">
                                -
                            </Text>
                        </Flex>
                    </Flex>
                )}
            </Flex>
        </Box>
    );
};
