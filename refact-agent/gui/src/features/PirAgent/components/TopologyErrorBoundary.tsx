import React from "react";
import { Box, Button, Text } from "@radix-ui/themes";

type TopologyErrorBoundaryProps = {
  children: React.ReactNode;
  label?: string;
};

type State = { error: Error | null };

export class TopologyErrorBoundary extends React.Component<
  TopologyErrorBoundaryProps,
  State
> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <Box p="4" style={{ border: "1px solid var(--red-6)", borderRadius: 8 }}>
          <Text size="2" weight="bold" color="red" mb="2">
            Topology UI error
            {this.props.label ? ` (${this.props.label})` : ""}
          </Text>
          <Text size="1" color="gray" mb="3">
            {this.state.error.message}
          </Text>
          <Button
            size="1"
            variant="soft"
            onClick={() => this.setState({ error: null })}
          >
            Try again
          </Button>
        </Box>
      );
    }
    return this.props.children;
  }
}
