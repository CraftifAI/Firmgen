import { Box, IconButton } from "@radix-ui/themes";
import { FiShare } from "react-icons/fi";

import craftifPanelBtn from "../CraftifPanelButton/craftifPanelButton.module.css";

export const Sharebutton = () => {
  return (
    <Box
      style={{
        /* Toolbar uses align=end; margin on inner IconButton doesn’t shift the block — nudge whole control */
        display: "inline-flex",
        alignItems: "center",
        transform: "translateY(-4px)",
        marginBottom: "5px",
        
      }}
    >
      <IconButton
        type="button"
        size="2"
        variant="ghost"
        title="Share"
        aria-label="Share"
        className={craftifPanelBtn.settingsTrigger + " " + craftifPanelBtn.hoverable}
      >
        <FiShare size={16} aria-hidden />
      </IconButton>
    </Box>
  );
};

export default Sharebutton;
