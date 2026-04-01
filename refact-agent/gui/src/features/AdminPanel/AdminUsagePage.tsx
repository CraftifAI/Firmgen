import React, { useEffect, useState } from "react";
import { Container, Heading, Table, Text, Card, Flex, Button } from "@radix-ui/themes";
import { CRAFTIF_API_BASE } from "../../config/craftifApiBase";
import { useCraftifAuth } from "../../hooks";

interface UsageRecord {
    id: string;
    userId: string;
    model: string;
    promptTokens: number;
    completionTokens: number;
    totalTokens: number;
    createdAt: string;
}

export const AdminUsagePage: React.FC = () => {
    const { jwt, user, logout } = useCraftifAuth();
    const [usage, setUsage] = useState<UsageRecord[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (!jwt || user?.role !== "ADMIN") {
            setLoading(false);
            return;
        }

        const fetchUsage = async () => {
            try {
                const res = await fetch(`${CRAFTIF_API_BASE}/usage/admin/users`, {
                    headers: {
                        Authorization: `Bearer ${jwt}`,
                    },
                });
                if (!res.ok) {
                    throw new Error("Failed to fetch usage data");
                }
                const data = await res.json();
                setUsage(data);
            } catch (err: unknown) {
                const message = err instanceof Error ? err.message : "An error occurred";
                setError(message);
            } finally {
                setLoading(false);
            }
        };

        void fetchUsage();
    }, [jwt, user?.role]);

    if (!jwt) {
        return (
            <Container>
                <Text>Please log in to view this page.</Text>
            </Container>
        );
    }

    if (user?.role !== "ADMIN") {
        return (
            <Container>
                <Text color="red">Access Denied: You must be an administrator to view usage statistics.</Text>
            </Container>
        );
    }

    return (
        <Container size="4" py="6">
            <Flex justify="between" align="center" mb="6">
                <Heading size="6">Admin Panel: Platform Usage</Heading>
                <Button variant="soft" onClick={logout}>Sign Out</Button>
            </Flex>

            <Card size="2" style={{ backgroundColor: "var(--color-surface)" }}>
                {loading ? (
                    <Text>Loading usage data...</Text>
                ) : error ? (
                    <Text color="red">{error}</Text>
                ) : usage.length === 0 ? (
                    <Text>No usage records found.</Text>
                ) : (
                    <Table.Root>
                        <Table.Header>
                            <Table.Row>
                                <Table.ColumnHeaderCell>User ID</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell>Model</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell>Prompt Tokens</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell>Completion Tokens</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell>Total</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell>Date</Table.ColumnHeaderCell>
                            </Table.Row>
                        </Table.Header>
                        <Table.Body>
                            {usage.map((record) => (
                                <Table.Row key={record.id}>
                                    <Table.RowHeaderCell>{record.userId}</Table.RowHeaderCell>
                                    <Table.Cell>{record.model}</Table.Cell>
                                    <Table.Cell>{record.promptTokens}</Table.Cell>
                                    <Table.Cell>{record.completionTokens}</Table.Cell>
                                    <Table.Cell>{record.totalTokens}</Table.Cell>
                                    <Table.Cell>{new Date(record.createdAt).toLocaleString()}</Table.Cell>
                                </Table.Row>
                            ))}
                        </Table.Body>
                    </Table.Root>
                )}
            </Card>
        </Container>
    );
};
