# Demo Scripts: ESP32 Agent

## ⚡ 5-Minute "Speed Run" Demo

**Goal:** Go from zero to a flashing LED on hardware in under 5 minutes.

1.  **Setup (Pre-demo)**:
    *   Laptop with Docker running.
    *   ESP32-S3 DevKitC plugged in via USB.
    *   Run `./run-all-docker-esp32.sh`.
    *   Open `localhost:5173`.

2.  **The Prompt (0:00 - 0:30)**:
    *   "Create a new 'hello_world_blink' project for my ESP32-S3 DevKitC. Use GPIO 38 for the RGB LED."
    *   *Highlight*: Agent acknowledges board type and creates file structure.

3.  **The Build (0:30 - 2:00)**:
    *   User: "Build and flash it."
    *   Agent runs `esp32_build`.
    *   *Highlight*: Streaming build logs in the chat. "See, it's actually running the compiler, not just hallucinating text."

4.  **The Flash (2:00 - 2:30)**:
    *   Agent runs `esp32_device` to detect port.
    *   Agent flashes the board.
    *   *Highlight*: "It auto-detected `/dev/ttyACM0`."

5.  **The Proof (2:30 - 3:00)**:
    *   User: "Monitor the output."
    *   Agent runs `esp32_device monitor`.
    *   *Highlight*: Real-time logs showing "Blinking!" and the actual LED blinking on the desk.

## 🕒 15-Minute "Deep Dive" Demo

**Goal:** Show intelligence, error handling, and component management.

1.  **Project Context (0:00 - 3:00)**:
    *   Show `esp32_tools.yaml` configuration.
    *   Explain the **Board Definition**: Open `board_definitions/esp32-s3-devkitc-1-n32r8v.json`. "This is the source of truth. The agent knows GPIO 0 is a boot pin and won't use it for your relay."

2.  **Complex Task: Wi-Fi Station (3:00 - 7:00)**:
    *   Prompt: Paste content from `example_wifi_prompt_esp32_agent.txt` (Wi-Fi station with LED status).
    *   *Highlight*: Agent adds `esp_wifi` component, configures `sdkconfig` for NVS and Wi-Fi stack, and implements the state machine code.

3.  **The "Oops" Moment (7:00 - 11:00)**:
    *   *Action*: Deliberately introduce a syntax error (e.g., delete a semicolon or misspell `esp_wifi_init`).
    *   Ask Agent to Build.
    *   *Outcome*: Build fails.
    *   **The Magic**: Agent triggers `esp32_analyze`. It parses the GCC error, pinpointing the line. It then suggests the fix.
    *   User: "Apply fix." -> Agent applies -> Rebuild succeeds.

4.  **Configuration Change (11:00 - 13:00)**:
    *   Prompt: "Change the Wi-Fi SSID to 'Conference_Guest' and enable power saving mode."
    *   *Highlight*: Agent uses `esp32_config` to modify `sdkconfig` (Power Management) and updates the source code macro.

5.  **Q&A / Wrap-up (13:00 - 15:00)**:
    *   Show `caps.json` to prove local tool capabilities.

## Failure Modes & Recovery

| Issue | Symptom | Recovery Line |
| :--- | :--- | :--- |
| **Device Busy** | "Resource busy" / "Failed to open port" | "Ah, the serial monitor is still open. Let me stop it." (Click 'Stop' in GUI terminal or `killall monitor`). |
| **Build Timeout** | Build hangs on large component | "First builds take longer due to full compilation. Subsequent builds are incremental." |
| **Wrong Port** | Flashing fails | "Let me explicitly tell it the port." -> "Use /dev/ttyUSB1". |
| **Hallucination** | Agent suggests non-existent API | "Let's check the RAG." -> "Search vector DB for 'esp_wifi_init' signature." |
