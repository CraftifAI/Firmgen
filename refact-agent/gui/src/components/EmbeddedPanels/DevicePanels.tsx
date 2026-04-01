import React, { useState, useRef } from "react";
import { Flex, Box, Text, IconButton } from "@radix-ui/themes";
import {
    ChevronDownIcon,
    ChevronRightIcon,
} from "@radix-ui/react-icons";
import { VscTerminal , VscCircuitBoard } from "react-icons/vsc";
import classNames from "classnames";

import { UartOutputWindow } from "./UartOutputWindow/UartOutputWindow"; // Adjusted path to assume relative structure
import { CommandLogWindow } from "./CommandLogWindow/CommandLogWindow"; // Adjusted path to assume relative structure
import craftifPanelBtn from "../CraftifPanelButton/craftifPanelButton.module.css";

export const DevicePanels: React.FC = () => {
    const [areDevicePanelsOpen, setAreDevicePanelsOpen] = useState(false);
    const [activeDevicePanel, setActiveDevicePanel] = useState<"uart" | "command">("uart");
    const devicePanelsRef = useRef<HTMLDivElement | null>(null);



    // Handle Opening/Closing Bottom Panel
    const toggleDevicePanel = (panel: "uart" | "command") => {
        // If clicking the same active panel while open, collapse it
        if (areDevicePanelsOpen && activeDevicePanel === panel) {
            setAreDevicePanelsOpen(false);
        } else {
            // If closed, open it. If open but different panel, switch panel.
            if (!areDevicePanelsOpen) {
                setAreDevicePanelsOpen(true);
            }
            setActiveDevicePanel(panel);
        }
    };

    const collapseDevicePanel = () => {
        setAreDevicePanelsOpen((prev) => !prev);
    };

    return (
        <Box
            ref={devicePanelsRef}
            mt="2"
            style={{
                borderRadius: 0,
                borderTop: "1px solid var(--border-subtle)",
                borderLeft: "none",
                borderRight: "none",
                borderBottom: "none",
                height: areDevicePanelsOpen ? 250 : 40, // Adjust height for open/closed state
                background: "var(--palette-deep-navy)",
                overflow: "hidden",
                minHeight: 40,
                flexShrink: 0,
                transition: "height 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
                display: "flex",
                flexDirection: "column",
                position: "relative",
            }}
        >


            {/* Inner container for content to manage overflow */}
            <Flex direction="column" style={{ flex: 1, overflow: 'hidden' }}>
                <Flex
                    align="center"
                    justify="between"
                    px="2"
                    py="1"
                    style={{ cursor: "pointer", flexShrink: 0 }}
                    onClick={collapseDevicePanel}
                >
                    <Flex gap="1">
                        <Box
                            asChild
                            className={classNames(
                                craftifPanelBtn.root,
                                activeDevicePanel === "uart" && craftifPanelBtn.active,
                            )}
                            onClick={(e) => {
                                e.stopPropagation();
                                toggleDevicePanel("uart");
                            }}
                        >
                            <Flex align="center" className={craftifPanelBtn.tabInner}>
                                <VscCircuitBoard width="12" height="12" />
                                <Text>Output</Text>
                            </Flex>
                        </Box>

                        <Box
                            asChild
                            className={classNames(
                                craftifPanelBtn.root,
                                activeDevicePanel === "command" && craftifPanelBtn.active,
                            )}
                            onClick={(e) => {
                                e.stopPropagation();
                                toggleDevicePanel("command");
                            }}
                        >
                            <Flex align="center" className={craftifPanelBtn.tabInner}>
                                <VscTerminal width="12" height="12" />
                                <Text>Terminal</Text>
                            </Flex>
                        </Box>
                    </Flex>
                    <IconButton
                        size="1"
                        variant="ghost"
                        aria-label={
                            areDevicePanelsOpen
                                ? "Collapse device panels"
                                : "Expand device panels"
                        }
                    >
                        {areDevicePanelsOpen ? (
                            <ChevronDownIcon />
                        ) : (
                            <ChevronRightIcon />
                        )}
                    </IconButton>
                </Flex>

                {areDevicePanelsOpen && (
                    <Flex
                        direction="column"
                        px="2"
                        pb="2"
                        gap="1"
                        style={{ flex: 1, minHeight: 0, overflow: 'hidden' }}
                    >
                        <Box style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
                            {activeDevicePanel === "uart" ? (
                                <UartOutputWindow />
                            ) : (
                                <CommandLogWindow />
                            )}
                        </Box>
                    </Flex>
                )}
            </Flex>
        </Box>
    );
};
