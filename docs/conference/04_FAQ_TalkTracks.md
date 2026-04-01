# FAQ & Talk Tracks

## General Product Questions

### Q: What exactly is this?
**A:** It's an intelligent AI agent that lives on your machine and helps you build embedded firmware. Think of it as a senior engineer pair-programmer who knows the ESP-IDF SDK inside and out, can run builds, fix errors, and flash your board for you.

### Q: Is this just a wrapper around ChatGPT?
**A:** No. While we use LLMs for reasoning, the magic is in the **Toolchain Integration**. We have custom Rust-based tools (`esp32_build`, `esp32_device`) that actually execute commands, parse the output, and feed structured error data back to the model. A standard LLM can't flash your board or read your serial port; we can.

### Q: Which hardware do you support?
**A:**
*   **Production:** TI C2000 (F28P65x series).
*   **Beta (Feature Complete):** ESP32-S3, ESP32-C3, ESP32-C6, ESP32-P4.
*   **Roadmap:** STM32, Nordic nRF.

### Q: Can I run this offline?
**A:** Yes! The architecture is designed for it. The agent engine, tools, and vector database are all local. If you point the API to a local LLM inference server (e.g., Ollama or TGI), the entire stack can run air-gapped.

## Technical Questions

### Q: How do you prevent the AI from hallucinating non-existent pins?
**A:** We use strict **Board Definitions** (JSON files). The agent "reads" the hardware spec before suggesting code. If it tries to use a pin that isn't safe or doesn't exist on your specific board variant, our validation layer catches it.

### Q: What happens if the code doesn't compile?
**A:** That's our superpower. We have a specialized **Error Parser** that reads GCC and Linker output. It identifies the exact file and line, understands the error context (e.g., "undefined reference" or "DRAM overflow"), and feeds that specific context back to the agent to generate a fix automatically.

### Q: Can I use my own ESP-IDF installation?
**A:** Yes. You configure `esp_idf_path` in `esp32_tools.yaml`. The agent respects your local environment variables and toolchains.

### Q: Does it support OTA?
**A:** Yes. We have tools to configure partition tables for OTA and generate the necessary project structure.

## Workflow & Setup

### Q: How long does it take to set up?
**A:** About 5 minutes.
1.  Run `./run-all-docker-esp32.sh`.
2.  Open browser to `localhost:5173`.
3.  Connect your board.

### Q: Do I need to learn a new IDE?
**A:** No. You can use our GUI for the agentic workflow, but the code is generated on your file system. You can open the same folder in VS Code, CLion, or Vim and work as you normally would. The agent works on the real files.

## Security & Data

### Q: Where does my code go?
**A:** Your code stays local on your machine (or within the Docker container). It is *never* sent to our servers for storage. Snippets are only sent to the inference endpoint (e.g., OpenAI/Azure) for completion if you use a cloud model.

### Q: Do you train on my code?
**A:** No. We are a tool provider, not a model trainer.
