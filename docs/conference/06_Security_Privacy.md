# Security & Privacy Architecture

## Data Flow

The Embedded Agent is architected with a **Local-First** philosophy.

```mermaid
graph LR
    User[User Input] -->|Serial/HTTP| Agent[Rust Agent (Local)]
    Agent -->|Code/Context| API[Python API (Local)]
    API -->|Search| RAG[Static VecDB (Local)]
    API -->|Inference (Sanitized)| LLM[Model Endpoint]
    
    subgraph "Your Machine / Private Cloud"
        Agent
        API
        RAG
    end
    
    subgraph "External (Optional)"
        LLM
    end
```

## Data Storage & Retention

| Data Type | Storage Location | Retention | Notes |
| :--- | :--- | :--- | :--- |
| **Source Code** | Local Filesystem | Indefinite (User controlled) | Never uploaded to our servers. |
| **Vector Index** | `static/*.vecdb` | Static / Read-only | Pre-computed knowledge base. User code is **not** indexed remotely. |
| **API Keys** | Environment Variables | Ephemeral | `OPENAI_API_KEY` injected at runtime. Not saved to disk. |
| **Logs** | `stderr` / Docker Logs | Ephemeral | No centralized telemetry. |
| **Chat History** | In-memory / Browser Storage | Session-based | Cleared on restart/refresh. |

## Enterprise Security Posture

### 1. Air-Gapped Operation
*   **Capability**: The system can function entirely without internet access if configured with a local LLM backend (e.g., vLLM, TGI, Ollama).
*   **Requirements**:
    *   Point `caps.json` to local endpoint.
    *   Use local `esp32_tools.yaml` config.
    *   Offline ESP-IDF installation.

### 2. No "Phone Home" Telemetry
*   **Evidence**: Codebase review of `refactapi.py` and rust agent confirms no analytics SDKs (Google Analytics, Sentry, Mixpanel) are initialized.
*   **Audit**: All outgoing HTTP calls are strictly for:
    1.  LLM Inference (user-configured endpoint).
    2.  Localhost communication between Agent/API/GUI.

### 3. Docker Isolation
*   **Runtime**: The API and GUI run in isolated Docker containers (`docker-compose.test.yml`).
*   **Host Access**: The Agent runs on the host for hardware access (`/dev/tty*`), but is restricted to the directories you explicitly mount or access.

## Known Limitations / User Responsibility ((Unknowns))

*   **LLM Provider logging**: If using OpenAI/Anthropic, their terms of service regarding data retention apply. **Recommendation**: Use Azure OpenAI or AWS Bedrock for zero-retention guarantees.
*   **Update Mechanism**: Updates are manual (`git pull` / `docker compose build`). There is no auto-updater, reducing the risk of supply chain attacks introducing malicious code automatically.
