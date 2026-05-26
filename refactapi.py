import os
import time
import uuid
import json
import logging
from pathlib import Path
from typing import Dict
from asyncio import Semaphore

import openai
from openai import RateLimitError, APIError
from fastapi import FastAPI, HTTPException, UploadFile, File, status, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse, JSONResponse

from pydantic import BaseModel
from typing import List, Optional, Any

from file_parsers import parse_file, SUPPORTED_EXTENSIONS, SUPPORTED_MIME_TYPES

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# -------- Models --------
# Models that ONLY support the v1/responses endpoint (not v1/chat/completions)
RESPONSES_API_MODELS: set = {
    "gpt-5-codex",
    "codex-mini-latest",
    "gpt-5.2-codex",
    "gpt-5.3-codex",
    "gpt-5.3-codex-spark",
    "gpt-5.1-codex-mini",
}

class Message(BaseModel):
    role: str
    content: Any  # str or list[dict] for multimodal
    tool_calls: Optional[Any] = None
    tool_call_id: Optional[str] = None

class ChatContext(BaseModel):
    messages: List[Message]
    model: Optional[str] = None
    temperature: float = 1.0
    top_p: float = 1.0
    n: int = 1
    stream: bool = False
    tools: Optional[Any] = None
    tool_choice: Optional[Any] = None
    stop: Optional[Any] = None
    reasoning_effort: Optional[str] = None
    actual_max_tokens: int = 4096

# -------- Helpers --------
def get_caps_version() -> str:
    return 1

def clamp(lo, hi, val):
    try:
        return max(lo, min(val, hi))
    except Exception:
        return lo

def estimate_tokens(texts, model="text-embedding-3-small"):
    """Estimate token count for a list of texts"""
    try:
        import tiktoken
        encoding = tiktoken.encoding_for_model(model)
        total = 0
        for text in texts:
            total += len(encoding.encode(str(text)))
        return total
    except ImportError:
        # Fallback: approximate 1 token ≈ 4 characters (conservative estimate)
        total = 0
        for text in texts:
            total += len(str(text)) // 4
        return total
    except Exception:
        # Final fallback
        total = 0
        for text in texts:
            total += len(str(text)) // 4
        return total

def split_oversized_chunk(text, max_tokens, model="text-embedding-3-small", target_chunk_size=1500, overlap=200):
    """
    Split an oversized chunk into multiple smaller chunks that fit within token limits.
    
    Args:
        text: The text to split
        max_tokens: Maximum tokens per chunk
        model: Embedding model name
        target_chunk_size: Target size for each chunk (tokens)
        overlap: Number of tokens to overlap between chunks for context preservation
    
    Returns:
        List of text chunks that fit within max_tokens
    """
    if not isinstance(text, str):
        return [text]  # Can't split non-string items
    
    # Check if splitting is needed
    if estimate_tokens([text], model) <= max_tokens:
        return [text]
    
    # Try to split at natural boundaries (lines, paragraphs, sentences)
    chunks = []
    
    # First, try splitting by double newlines (paragraphs)
    paragraphs = text.split('\n\n')
    current_chunk = []
    current_tokens = 0
    
    for para in paragraphs:
        para_tokens = estimate_tokens([para], model)
        
        # If paragraph itself exceeds limit, split it by single newlines
        if para_tokens > max_tokens:
            # Flush current chunk if any
            if current_chunk:
                chunks.append('\n\n'.join(current_chunk))
                current_chunk = []
                current_tokens = 0
            
            # Split by lines
            lines = para.split('\n')
            for line in lines:
                line_tokens = estimate_tokens([line], model)
                
                # If single line is too large, split by characters (last resort)
                if line_tokens > max_tokens:
                    # Add accumulated chunk first
                    if current_chunk:
                        chunks.append('\n\n'.join(current_chunk))
                        current_chunk = []
                        current_tokens = 0
                    
                    # Character-level splitting for extremely long lines
                    char_chunk = ""
                    for char in line:
                        test_chunk = char_chunk + char
                        if estimate_tokens([test_chunk], model) > target_chunk_size:
                            if char_chunk:
                                chunks.append(char_chunk)
                            char_chunk = char
                        else:
                            char_chunk = test_chunk
                    if char_chunk:
                        current_chunk = [char_chunk]
                        current_tokens = estimate_tokens([char_chunk], model)
                else:
                    # Check if adding this line would exceed limit
                    if current_tokens + line_tokens > max_tokens:
                        if current_chunk:
                            chunks.append('\n\n'.join(current_chunk))
                            # Add overlap from previous chunk
                            if overlap > 0 and chunks:
                                overlap_text = '\n\n'.join(current_chunk[-overlap//100:]) if len(current_chunk) > overlap//100 else '\n\n'.join(current_chunk)
                                current_chunk = [overlap_text, line] if overlap_text else [line]
                                current_tokens = estimate_tokens(['\n\n'.join(current_chunk)], model)
                            else:
                                current_chunk = [line]
                                current_tokens = line_tokens
                        else:
                            current_chunk = [line]
                            current_tokens = line_tokens
                    else:
                        current_chunk.append(line)
                        current_tokens += line_tokens
        else:
            # Normal paragraph handling
            if current_tokens + para_tokens > max_tokens:
                if current_chunk:
                    chunks.append('\n\n'.join(current_chunk))
                    # Add overlap if enabled
                    if overlap > 0 and chunks and len(current_chunk) > 0:
                        overlap_text = '\n\n'.join(current_chunk[-1:]) if len(current_chunk) >= 1 else current_chunk[-1]
                        current_chunk = [overlap_text, para] if overlap_text else [para]
                        current_tokens = estimate_tokens(['\n\n'.join(current_chunk)], model)
                    else:
                        current_chunk = [para]
                        current_tokens = para_tokens
                else:
                    current_chunk = [para]
                    current_tokens = para_tokens
            else:
                current_chunk.append(para)
                current_tokens += para_tokens
    
    # Add remaining chunk
    if current_chunk:
        chunks.append('\n\n'.join(current_chunk))
    
    # Verify all chunks are within limit and split further if needed
    final_chunks = []
    for chunk in chunks:
        chunk_tokens = estimate_tokens([chunk], model)
        if chunk_tokens > max_tokens:
            # Recursively split if still too large (with smaller target)
            sub_chunks = split_oversized_chunk(chunk, max_tokens, model, target_chunk_size // 2, 0)
            final_chunks.extend(sub_chunks)
        else:
            final_chunks.append(chunk)
    
    return final_chunks if final_chunks else [text]  # Fallback to original if splitting failed

# -------- OpenAI Client Management --------
_client = None
_embedding_semaphore = Semaphore(20)  # Limit concurrent embedding requests to 20

def get_openai_client(auth_key: str = None):
    """Get or create shared OpenAI client"""
    global _client
    
    if auth_key:
        return openai.AsyncOpenAI(
            api_key=auth_key,
            base_url="https://api.craftifai.com/v1/",
            timeout=120.0,
            max_retries=3
        )

    if _client is None:
        api_key = os.environ.get("OPENAI_API_KEY")
        if not api_key:
            raise ValueError("OPENAI_API_KEY environment variable not set")
        _client = openai.AsyncOpenAI(
            api_key=api_key,
            base_url="https://api.craftifai.com/v1/",
            timeout=120.0,  # 120 second timeout for large batches
            max_retries=3  # Retry up to 3 times on transient errors
        )
        logger.info("Created shared OpenAI client")
    return _client

# -------- FastAPI --------
app = FastAPI()

# Allow browser requests from the frontend (e.g. Vite dev server or embedded host)
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Global active JWT to intercept mismatched keys
ACTIVE_JWT_TOKEN = None

class SetJwtRequest(BaseModel):
    token: str

@app.post("/v1/proxy-set-jwt")
async def set_jwt(req: SetJwtRequest):
    global ACTIVE_JWT_TOKEN
    ACTIVE_JWT_TOKEN = req.token
    return {"status": "ok"}


# ── Board-definition context injection ────────────────────────────────────────
# Loaded once when the user confirms board selection in the wizard. The
# formatted text is prepended to the system message on every chat request so
# the LLM has accurate GPIO/hardware knowledge from message 1.

ACTIVE_BOARD_DEFINITION: dict = {}


def _load_board_json(board_id: str) -> dict:
    """Return the board definition dict from cache or local folder."""
    cache_path = _refact_cache_dir() / "board_definitions" / f"{board_id}.json"
    if cache_path.exists():
        try:
            with open(cache_path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
    local_path = Path("board_definitions") / f"{board_id}.json"
    if local_path.exists():
        with open(local_path, "r", encoding="utf-8") as f:
            return json.load(f)
    raise FileNotFoundError(f"Board definition not found for: {board_id}")


def _format_board_context(board: dict) -> str:
    """Format the board definition into a compact LLM-readable context block."""
    gpio = board.get("gpio", {})
    hw = board.get("hardware", {})
    flash = hw.get("flash", {})
    psram = hw.get("psram", {})
    uart = hw.get("uart_console", {})
    usb = hw.get("usb_jtag", {})
    adc = board.get("adc", {})
    led = gpio.get("led", {})
    button = gpio.get("button", {})
    presets = list(board.get("config_presets", {}).keys())

    restricted_reasons = gpio.get("restricted_reasons", {})
    restricted_str = "; ".join(
        f"GPIO {k}: {v}" for k, v in restricted_reasons.items()
    )

    lines = [
        "=== ACTIVE BOARD CONTEXT (injected at session start) ===",
        f"Board   : {board.get('name', '?')} — variant {board.get('variant', '?')}",
        f"Chip    : {board.get('chip', {}).get('type', '?')}",
        f"Flash   : {flash.get('size', '?')}  mode={flash.get('mode', '?')}  freq={flash.get('freq', '?')}",
        f"PSRAM   : {'enabled  size=' + psram.get('size', '?') + '  mode=' + psram.get('mode', '?') if psram.get('enabled') else 'none'}",
        f"UART console : TX={uart.get('tx', '?')}  RX={uart.get('rx', '?')}",
        f"USB JTAG     : D-={usb.get('d_minus', '?')}  D+={usb.get('d_plus', '?')}",
        f"LED     : GPIO {led.get('pin', '?')}  driver={led.get('driver', '?')}  ({led.get('notes', '')})",
        f"Button  : GPIO {button.get('pin', '?')}  ({button.get('notes', '')})",
        f"Safe GPIO pins    : {gpio.get('safe_pins', [])}",
        f"Restricted GPIO   : {gpio.get('restricted_pins', [])}",
        f"Restricted reasons: {restricted_str}",
        f"ADC1 pins : {adc.get('adc1_pins', [])}",
        f"ADC2 pins : {adc.get('adc2_pins', [])}  (WARNING: ADC2 conflicts with WiFi when active)",
        f"Config presets available: {presets}",
        "IMPORTANT: Always respect restricted GPIO pins in generated code. Never assign",
        "user peripherals to restricted pins. Use safe_pins for general I/O.",
        "=== END BOARD CONTEXT ===",
    ]
    return "\n".join(lines)


class SetActiveBoardRequest(BaseModel):
    board_id: str


@app.post("/v1/set-active-board")
async def set_active_board(req: SetActiveBoardRequest):
    """Load the selected board definition and store it for LLM context injection."""
    global ACTIVE_BOARD_DEFINITION
    try:
        board = _load_board_json(req.board_id)
        ACTIVE_BOARD_DEFINITION = board
        logger.info(f"Active board set to: {req.board_id} ({board.get('name', '?')})")
        return {"status": "ok", "board_id": req.board_id, "name": board.get("name")}
    except FileNotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to load board: {e}")


@app.get("/v1/active-board")
async def get_active_board():
    """Debug endpoint — returns the currently loaded board and the exact context
    block that will be injected into every LLM system prompt."""
    if not ACTIVE_BOARD_DEFINITION:
        return {
            "status": "not_set",
            "message": "No board loaded. POST /v1/set-active-board first.",
            "board": None,
            "injected_context": None,
        }
    return {
        "status": "ok",
        "board_id": ACTIVE_BOARD_DEFINITION.get("board_id"),
        "name": ACTIVE_BOARD_DEFINITION.get("name"),
        "variant": ACTIVE_BOARD_DEFINITION.get("variant"),
        "injected_context": _format_board_context(ACTIVE_BOARD_DEFINITION),
    }


@app.get("/v1/caps")
async def get_v1_caps():
    """V1 caps endpoint for Refact agent"""
    file_path = Path("caps.json")
    if not file_path.exists():
        raise HTTPException(status_code=404, detail="caps.json file not found")
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Error reading caps.json: {str(e)}")

@app.post("/v1/embeddings")
async def embeddings(request: dict, fastapi_req: Request):
    """Handle embedding requests with proper error handling, rate limiting, and batch splitting"""
    async with _embedding_semaphore:  # Limit concurrent requests
        try:
            # Validate request
            if "input" not in request:
                logger.error("Embedding request missing 'input' field")
                raise HTTPException(status_code=400, detail="Missing 'input' field")
            
            input_data = request["input"]
            model = request.get("model", "text-embedding-3-small")
            
            # Model-specific token limits (safety margin below actual limit)
            MAX_TOKENS = {
                "text-embedding-3-small": 8000,  # Actual: 8192, using 7000 for safety margin
                "text-embedding-3-large": 7000,  # Actual: 8191
                "text-embedding-ada-002": 7000,  # Actual: 8191
            }
            max_tokens = MAX_TOKENS.get(model, 8000)
            
            # Handle batch splitting for lists
            if isinstance(input_data, list) and len(input_data) > 1:
                logger.info(f"Embedding request: {len(input_data)} items, model: {model}")
                
                # Estimate total tokens
                total_tokens = estimate_tokens(input_data, model)
                
                if total_tokens > max_tokens:
                    logger.warning(f"Batch too large ({total_tokens} tokens > {max_tokens}), splitting into smaller batches")
                    
                    # Split into smaller batches
                    auth_header = fastapi_req.headers.get("Authorization")
                    auth_key = auth_header.split(" ")[1] if auth_header and "Bearer " in auth_header else None
                    client = get_openai_client(auth_key)
                    all_embeddings = []
                    all_data = []
                    batch_start = 0
                    batch_num = 1
                    
                    while batch_start < len(input_data):
                        # Start with remaining items
                        remaining_items = input_data[batch_start:]
                        current_batch = []
                        current_tokens = 0
                        
                        # Build batch within token limit
                        for item_idx, item in enumerate(remaining_items):
                            item_index = batch_start + len(current_batch)
                            item_tokens = estimate_tokens([item], model)
                            
                            # If single item exceeds limit, truncate it
                            if item_tokens > max_tokens:
                                logger.warning(f"Item at index {item_index} exceeds token limit ({item_tokens} > {max_tokens}), truncating")
                                
                                # Truncate the item to fit within limit
                                if isinstance(item, str):
                                    # Binary search for optimal truncation point
                                    original_item = item
                                    low, high = 0, len(item)
                                    best_item = item
                                    
                                    while low < high:
                                        mid = (low + high) // 2
                                        truncated = item[:mid] if mid > 0 else ""
                                        truncated_tokens = estimate_tokens([truncated], model) if truncated else 0
                                        
                                        if truncated_tokens <= max_tokens:
                                            best_item = truncated
                                            low = mid + 1
                                        else:
                                            high = mid
                                    
                                    if best_item and estimate_tokens([best_item], model) <= max_tokens:
                                        item = best_item
                                        item_tokens = estimate_tokens([item], model)
                                        logger.info(f"Truncated item {item_index} from {len(original_item)} to {len(item)} chars (~{item_tokens} tokens)")
                                    else:
                                        # If still too large, skip it
                                        logger.error(f"Item {item_index} still exceeds limit after truncation, skipping")
                                        continue
                                else:
                                    logger.error(f"Item {item_index} is not a string and exceeds limit, skipping")
                                    continue
                            
                            if current_tokens + item_tokens > max_tokens:
                                break
                            
                            current_batch.append(item)
                            current_tokens += item_tokens
                        
                        if not current_batch:
                            logger.error(f"Cannot process batch starting at index {batch_start} - no items fit in token limit")
                            batch_start += 1
                            continue
                        
                        # Process this batch
                        batch_tokens = estimate_tokens(current_batch, model)
                        logger.info(f"Processing batch {batch_num}: {len(current_batch)} items, ~{batch_tokens} tokens (items {batch_start}-{batch_start + len(current_batch) - 1})")
                        
                        try:
                            response = await client.embeddings.create(
                                input=current_batch,
                                model=model
                            )
                            
                            # Extract embeddings and maintain order
                            for i, item in enumerate(response.data):
                                all_embeddings.append(item.embedding)
                                all_data.append({
                                    "object": "embedding",
                                    "embedding": item.embedding,
                                    "index": batch_start + i
                                })
                            
                            batch_start += len(current_batch)
                            batch_num += 1
                            
                        except Exception as e:
                            logger.error(f"Error processing batch {batch_num}: {e}")
                            raise
                    
                    logger.info(f"Successfully processed {len(input_data)} items in {batch_num - 1} batches")
                    return {
                        "object": "list",
                        "data": all_data,
                        "model": model,
                        "usage": {
                            "prompt_tokens": total_tokens,
                            "total_tokens": total_tokens
                        }
                    }
            
            # Original handling for single items or small batches
            if isinstance(input_data, list):
                logger.info(f"Embedding request: {len(input_data)} items, model: {model}")
            else:
                logger.info(f"Embedding request: single item, model: {model}")
            
            # Get shared client
            auth_header = fastapi_req.headers.get("Authorization")
            auth_key = auth_header.split(" ")[1] if auth_header and "Bearer " in auth_header else None
            client = get_openai_client(auth_key)
            
            # Make the API call with error handling
            response = await client.embeddings.create(
                input=input_data,
                model=model
            )
            
            # Log success
            if isinstance(input_data, list):
                logger.info(f"Embedding success: {len(input_data)} items processed")
            else:
                logger.info("Embedding success: single item processed")
            
            return response.model_dump()
            
        except RateLimitError as e:
            logger.error(f"Rate limit error: {e}")
            raise HTTPException(
                status_code=429,
                detail=f"Rate limit exceeded: {str(e)}"
            )
        except APIError as e:
            # Check if it's a token limit error and retry with splitting
            error_str = str(e)
            if "maximum context length" in error_str or "8192 tokens" in error_str or "exceeded" in error_str.lower():
                logger.warning(f"Token limit error detected, this should have been caught earlier: {e}")
                # This shouldn't happen with our pre-splitting, but handle it gracefully
                raise HTTPException(
                    status_code=400,
                    detail=f"Token limit exceeded. Please reduce batch size or contact support. Error: {str(e)}"
                )
            logger.error(f"OpenAI API error: {type(e).__name__}: {e}")
            raise HTTPException(
                status_code=502,
                detail=f"OpenAI API error: {str(e)}"
            )
        except KeyError as e:
            logger.error(f"Missing required field: {e}")
            raise HTTPException(status_code=400, detail=f"Missing required field: {str(e)}")
        except Exception as e:
            logger.error(f"Embedding error: {type(e).__name__}: {str(e)}", exc_info=True)
            raise HTTPException(status_code=500, detail=f"Internal error: {str(e)}")

@app.get("/caps.json")
async def handle_caps():
    file_path = Path("caps.json")
    if not file_path.exists():
        raise HTTPException(status_code=404, detail="caps.json file not found")
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Error reading caps.json: {str(e)}")

@app.get("/refact-caps")
async def get_caps():
    """Full caps for Refact agent"""
    file_path = Path("caps.json")
    if not file_path.exists():
        raise HTTPException(status_code=404, detail="caps.json file not found")
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Error reading caps.json: {str(e)}")

@app.get("/v1/c2000-config")
async def get_v1_c2000_config():
    """V1 C2000 config endpoint for C2000 tools"""
    config_path = Path("/home/shubham/.cache/refact/c2000_tools.yaml")
    if not config_path.exists():
        raise HTTPException(status_code=404, detail="C2000 config file not found")
    try:
        import yaml
        with open(config_path, "r", encoding="utf-8") as f:
            config_data = yaml.safe_load(f)
            return config_data
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Error reading C2000 config: {str(e)}")

def _refact_cache_dir() -> Path:
    """Base cache dir for refact (esp32_tools.yaml, etc.). Override with REFACT_CACHE_DIR."""
    if "REFACT_CACHE_DIR" in os.environ:
        return Path(os.environ["REFACT_CACHE_DIR"])
    if os.name == "nt":
        base = os.environ.get("LOCALAPPDATA") or str(Path.home() / "AppData" / "Local")
        return Path(base) / "refact"
    return Path.home() / ".cache" / "refact"


@app.get("/v1/esp32-config")
async def get_v1_esp32_config():
    """V1 ESP32 config endpoint for ESP32 tools. Uses REFACT_CACHE_DIR/esp32_tools.yaml or bundled default."""
    import yaml
    config_path = _refact_cache_dir() / "esp32_tools.yaml"
    fallback_path = Path("/app/default_esp32_tools.yaml")  # bundled in Docker image
    path_to_use = config_path if config_path.exists() else (fallback_path if fallback_path.exists() else None)
    if path_to_use is None:
        raise HTTPException(status_code=404, detail=f"ESP32 config file not found at {config_path}")
    try:
        with open(path_to_use, "r", encoding="utf-8") as f:
            return yaml.safe_load(f)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Error reading ESP32 config: {str(e)}")

def _user_board_dir() -> Path:
    """Writable directory for user-created board definitions."""
    d = _refact_cache_dir() / "board_definitions"
    d.mkdir(parents=True, exist_ok=True)
    return d


def _builtin_board_ids() -> set:
    """IDs of boards shipped with the app (read-only)."""
    local_dir = Path("board_definitions")
    if not local_dir.is_dir():
        return set()
    return {p.stem for p in local_dir.glob("*.json")}


@app.get("/v1/boards")
async def list_v1_boards():
    """Return a list of all available board definitions with summary metadata.

    Scans the local board_definitions folder first (built-in, read-only) then
    the user cache directory (user-created, editable). Any board JSON dropped
    in either location appears here automatically.
    """
    seen: dict[str, dict] = {}
    builtin_ids: set[str] = set()

    def _add(path: Path, is_builtin: bool) -> None:
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
            board_id = data.get("board_id") or path.stem
            if board_id not in seen:
                hw = data.get("hardware", {})
                flash = hw.get("flash", {})
                psram = hw.get("psram", {})
                seen[board_id] = {
                    "board_id": board_id,
                    "name": data.get("name", board_id),
                    "variant": data.get("variant", ""),
                    "description": data.get("description", ""),
                    "chip": data.get("chip", {}).get("type", ""),
                    "flash_size": flash.get("size", ""),
                    "psram_size": psram.get("size", "") if psram.get("enabled") else "",
                    "is_builtin": is_builtin,
                }
                if is_builtin:
                    builtin_ids.add(board_id)
        except Exception as e:
            logger.warning(f"Skipping board file {path}: {e}")

    local_dir = Path("board_definitions")
    if local_dir.is_dir():
        for p in sorted(local_dir.glob("*.json")):
            _add(p, is_builtin=True)

    cache_dir = _refact_cache_dir() / "board_definitions"
    if cache_dir.is_dir():
        for p in sorted(cache_dir.glob("*.json")):
            _add(p, is_builtin=(p.stem in builtin_ids))

    return {"boards": list(seen.values())}


@app.post("/v1/boards")
async def create_board(request: Request):
    """Save a new board definition JSON to the user board directory.

    Accepts the full board definition as a JSON body. The board_id field is
    used as the filename. Returns 409 if a board with that ID already exists
    in the built-in folder (user cannot overwrite built-in boards via this
    endpoint — use PUT to update user-created boards).
    """
    try:
        data = await request.json()
    except Exception:
        raise HTTPException(status_code=400, detail="Invalid JSON body")

    board_id = data.get("board_id", "").strip()
    if not board_id:
        raise HTTPException(status_code=400, detail="board_id field is required")

    builtin_path = Path("board_definitions") / f"{board_id}.json"
    if builtin_path.exists():
        raise HTTPException(
            status_code=409,
            detail=f"'{board_id}' is a built-in board and cannot be overwritten. Use a different board_id."
        )

    dest = _user_board_dir() / f"{board_id}.json"
    if dest.exists():
        raise HTTPException(
            status_code=409,
            detail=f"Board '{board_id}' already exists. Use PUT /v1/boards/{board_id} to update it."
        )

    with open(dest, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
    logger.info(f"Created user board: {board_id} → {dest}")
    return {"status": "created", "board_id": board_id, "path": str(dest)}


@app.put("/v1/boards/{board_id}")
async def update_board(board_id: str, request: Request):
    """Update an existing user-created board definition.

    Built-in boards cannot be updated via this endpoint.
    """
    global ACTIVE_BOARD_DEFINITION
    builtin_path = Path("board_definitions") / f"{board_id}.json"
    if builtin_path.exists():
        raise HTTPException(
            status_code=403,
            detail=f"'{board_id}' is a built-in board and cannot be modified."
        )

    dest = _user_board_dir() / f"{board_id}.json"
    if not dest.exists():
        raise HTTPException(status_code=404, detail=f"User board '{board_id}' not found.")

    try:
        data = await request.json()
    except Exception:
        raise HTTPException(status_code=400, detail="Invalid JSON body")

    data["board_id"] = board_id
    with open(dest, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
    logger.info(f"Updated user board: {board_id}")

    if ACTIVE_BOARD_DEFINITION.get("board_id") == board_id:
        ACTIVE_BOARD_DEFINITION = data
        logger.info(f"Refreshed active board context after update: {board_id}")

    return {"status": "updated", "board_id": board_id}


@app.delete("/v1/boards/{board_id}")
async def delete_board(board_id: str):
    """Delete a user-created board definition.

    Built-in boards cannot be deleted.
    """
    builtin_path = Path("board_definitions") / f"{board_id}.json"
    if builtin_path.exists():
        raise HTTPException(
            status_code=403,
            detail=f"'{board_id}' is a built-in board and cannot be deleted."
        )

    dest = _user_board_dir() / f"{board_id}.json"
    if not dest.exists():
        raise HTTPException(status_code=404, detail=f"User board '{board_id}' not found.")

    dest.unlink()
    logger.info(f"Deleted user board: {board_id}")
    return {"status": "deleted", "board_id": board_id}


# Board generation prompt — kept compact to save context tokens
_BOARD_EXTRACT_SYSTEM = """You are a hardware documentation parser for ESP32 boards.
Given the text extracted from a board datasheet or schematic, produce a JSON object
matching this schema (all fields optional unless marked *required*):

{
  "board_id": "<slug, lowercase, hyphens only>",   // *required*
  "name": "<human name>",                           // *required*
  "variant": "<variant string>",
  "description": "<one sentence>",
  "chip": { "type": "<esp32|esp32s3|esp32s2|esp32c3|esp32c6>" },
  "hardware": {
    "flash": { "size": "<4MB|8MB|16MB|32MB>", "mode": "<qio|dio>", "freq": "<80m|40m>" },
    "psram": { "enabled": true/false, "size": "<size>", "mode": "<quad|octal>" },
    "uart_console": { "tx": <pin>, "rx": <pin> },
    "usb_jtag": { "supported": true/false, "d_minus": <pin>, "d_plus": <pin> }
  },
  "gpio": {
    "led": { "pin": <n>, "type": "<rgb|standard>", "driver": "<ws2812|gpio>" },
    "button": { "pin": <n>, "pull": "<pullup|pulldown>" },
    "safe_pins": [<list of safe GPIO numbers>],
    "restricted_pins": [<list>],
    "restricted_reasons": { "<pin>": "<reason>", ... }
  },
  "adc": { "adc1_pins": [<list>], "adc2_pins": [<list>] },
  "config_presets": {
    "default": { "description": "...", "sdkconfig": { "<KEY>": "<val>", ... } }
  }
}

Return ONLY valid JSON. No markdown, no explanation. If you cannot determine a value,
omit that field rather than guessing."""


@app.post("/v1/boards/generate-from-pdf")
async def generate_board_from_pdf(
    file: UploadFile = File(...),
    hints: str = "",
):
    """Upload a board datasheet PDF and extract a draft board definition JSON.

    The draft is returned to the client for review — it is NOT saved automatically.
    The client should POST the reviewed JSON to /v1/boards to persist it.
    """
    if not file.filename or not file.filename.lower().endswith(".pdf"):
        raise HTTPException(status_code=400, detail="Only PDF files are supported.")

    try:
        from file_parsers.pdf_parser import PdfParser
        pdf_bytes = await file.read()
        parser = PdfParser()
        result = parser.parse(pdf_bytes, file.filename)
        pdf_text = result["text"]
    except Exception as e:
        raise HTTPException(status_code=422, detail=f"PDF parsing failed: {e}")

    # Truncate to avoid exceeding model context (keep first ~12000 chars)
    if len(pdf_text) > 12000:
        pdf_text = pdf_text[:12000] + "\n...[truncated]"

    user_message = f"Extract board definition from this datasheet text.\n"
    if hints:
        user_message += f"Additional hints: {hints}\n"
    user_message += f"\n---\n{pdf_text}\n---"

    global ACTIVE_JWT_TOKEN
    auth_key = ACTIVE_JWT_TOKEN or os.environ.get("OPENAI_API_KEY")
    if not auth_key:
        raise HTTPException(status_code=401, detail="No API key available for LLM extraction.")

    try:
        client = openai.AsyncOpenAI(api_key=auth_key, base_url=CRAFTIF_API_BASE + "/")
        response = await client.chat.completions.create(
            model="gpt-5.2",
            messages=[
                {"role": "system", "content": _BOARD_EXTRACT_SYSTEM},
                {"role": "user", "content": user_message},
            ],
            temperature=0.1,
        )
        raw = response.choices[0].message.content or ""
        raw = raw.strip().lstrip("```json").lstrip("```").rstrip("```").strip()
        draft = json.loads(raw)
    except json.JSONDecodeError as e:
        raise HTTPException(status_code=422, detail=f"LLM returned invalid JSON: {e}")
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"LLM extraction failed: {e}")

    return {"status": "draft", "board": draft}


@app.get("/v1/boards/{board_id}")
async def get_v1_board_definition(board_id: str):
    """V1 board definition endpoint - returns JSON board definition"""
    # Priority 1: Check cache folder
    cache_path = _refact_cache_dir() / "board_definitions" / f"{board_id}.json"
    if cache_path.exists():
        try:
            with open(cache_path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception as e:
            logger.warning(f"Failed to read board definition from cache: {e}")
            # Fall through to local folder
    
    # Priority 2: Check local board_definitions folder
    local_path = Path("board_definitions") / f"{board_id}.json"
    if local_path.exists():
        try:
            with open(local_path, "r", encoding="utf-8") as f:
                board_data = json.load(f)
                # Also save to cache for faster future access
                try:
                    cache_path.parent.mkdir(parents=True, exist_ok=True)
                    with open(cache_path, "w", encoding="utf-8") as f_cache:
                        json.dump(board_data, f_cache, indent=2)
                except Exception as e:
                    logger.warning(f"Failed to cache board definition: {e}")
                return board_data
        except json.JSONDecodeError as e:
            raise HTTPException(
                status_code=500,
                detail=f"Error parsing board definition JSON: {str(e)}"
            )
        except Exception as e:
            raise HTTPException(
                status_code=500,
                detail=f"Error reading board definition: {str(e)}"
            )
    
    raise HTTPException(
        status_code=404,
        detail=f"Board definition not found: {board_id}. Checked cache ({cache_path}) and local folder ({local_path})"
    )

@app.get("/v1/c2000-sysconfig-recipe")
async def get_v1_c2000_sysconfig_recipe():
    """V1 C2000 SysConfig recipe endpoint - returns launchxl_f28p65x recipe"""
    recipe_path = Path("/home/shubham/sdk_agent/refact/sysconfig/launchxl_f28p65x_syscfg_recipes.json")
    
    if not recipe_path.exists():
        raise HTTPException(
            status_code=404, 
            detail=f"Recipe file not found: {recipe_path}"
        )
    
    try:
        with open(recipe_path, "r", encoding="utf-8") as f:
            recipe_data = json.load(f)
            return recipe_data
    except json.JSONDecodeError as e:
        raise HTTPException(
            status_code=500, 
            detail=f"Error parsing recipe JSON at line {e.lineno}, column {e.colno}: {str(e)}"
        )
    except Exception as e:
        raise HTTPException(
            status_code=500, 
            detail=f"Error reading recipe file: {str(e)}"
        )

# ── File Upload Endpoints ─────────────────────────────────────────────────────

@app.post("/v1/upload")
async def upload_file(file: UploadFile = File(...)):
    """Parse an uploaded document and return extracted plain text + metadata.

    Accepted formats: PDF, DOCX, PPTX, XLSX, TXT (and other plain-text variants).

    Returns JSON:
    {
        "filename": "report.pdf",
        "mime_type": "application/pdf",
        "size_bytes": 123456,
        "text": "Extracted full text...",
        "metadata": { ... format-specific keys ... }
    }
    """
    filename = file.filename or "upload"
    content_type = file.content_type or ""

    try:
        data = await file.read()
    except Exception as exc:
        logger.error(f"Failed to read uploaded file '{filename}': {exc}")
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Could not read uploaded file: {exc}",
        )

    size_bytes = len(data)
    logger.info(f"Received upload: '{filename}' {size_bytes} bytes content_type={content_type!r}")

    try:
        result = parse_file(filename, data, content_type=content_type)
    except ValueError as exc:
        raise HTTPException(
            status_code=status.HTTP_415_UNSUPPORTED_MEDIA_TYPE,
            detail=str(exc),
        )
    except RuntimeError as exc:
        logger.error(f"Parser dependency missing: {exc}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=str(exc),
        )
    except Exception as exc:
        logger.error(f"Unexpected error parsing '{filename}': {exc}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Internal error while parsing file: {exc}",
        )

    mime_type = result.pop("mime_type", content_type)
    text = result.pop("text", "")
    metadata = result  # Remaining keys are format-specific metadata

    return JSONResponse(content={
        "filename": filename,
        "mime_type": mime_type,
        "size_bytes": size_bytes,
        "text": text,
        "metadata": metadata,
    })


@app.get("/v1/upload/supported-formats")
async def upload_supported_formats():
    """Return the list of file extensions and MIME types supported by /v1/upload."""
    return {
        "extensions": sorted(SUPPORTED_EXTENSIONS),
        "mime_types": sorted(SUPPORTED_MIME_TYPES),
    }


# ── Chat Completion Helpers ────────────────────────────────────────────────────

def _build_openai_messages(post: ChatContext) -> list:
    """Convert ChatContext messages to the OpenAI messages list format.

    Passes messages through directly without truncation or splitting,
    as the upstream CraftifAI gateway now supports large payloads.

    If a board definition has been set via /v1/set-active-board, its formatted
    context block is appended to the first system message (or inserted as a new
    system message before any user message) so the LLM is grounded in the
    correct board knowledge from message 1.
    """
    openai_messages = []

    for msg in post.messages:
        content = msg.content
        m: Dict[str, Any] = {"role": msg.role, "content": content}
        if msg.tool_calls:
            m["tool_calls"] = msg.tool_calls
        if msg.tool_call_id:
            m["tool_call_id"] = msg.tool_call_id
        openai_messages.append(m)

    # Inject board context into system prompt when a board is active
    if ACTIVE_BOARD_DEFINITION:
        board_ctx = _format_board_context(ACTIVE_BOARD_DEFINITION)
        injected = False
        for m in openai_messages:
            if m["role"] == "system":
                m["content"] = m["content"] + "\n\n" + board_ctx
                injected = True
                break
        if not injected:
            # No system message yet — insert one before the first user message
            openai_messages.insert(0, {"role": "system", "content": board_ctx})
        logger.info(
            f"[board-ctx] injected={injected} board={ACTIVE_BOARD_DEFINITION.get('board_id')} "
            f"messages_total={len(openai_messages)}"
        )
    else:
        logger.warning("[board-ctx] ACTIVE_BOARD_DEFINITION is empty — no board context injected")

    return openai_messages


def _responses_to_chat_completion(resp_obj: Any, model: str, request_id: str) -> dict:
    """
    Normalise a v1/responses Response object into the shape of a
    v1/chat/completions ChatCompletion so that callers need no changes.
    """
    # Extract text output from the response items
    output_text = ""
    tool_calls = None

    for item in getattr(resp_obj, "output", []):
        item_type = getattr(item, "type", None)
        if item_type == "message":
            for part in getattr(item, "content", []):
                if getattr(part, "type", None) == "output_text":
                    output_text += getattr(part, "text", "")
        elif item_type == "function_call":
            if tool_calls is None:
                tool_calls = []
            tool_calls.append({
                "id": getattr(item, "call_id", str(uuid.uuid4())),
                "type": "function",
                "function": {
                    "name": getattr(item, "name", ""),
                    "arguments": getattr(item, "arguments", "{}"),
                },
            })

    usage = getattr(resp_obj, "usage", None)
    usage_dict = {
        "prompt_tokens": getattr(usage, "input_tokens", 0),
        "completion_tokens": getattr(usage, "output_tokens", 0),
        "total_tokens": getattr(usage, "total_tokens", 0),
    } if usage else {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}

    message: Dict[str, Any] = {"role": "assistant", "content": output_text or None}
    if tool_calls:
        message["tool_calls"] = tool_calls

    finish_reason = "tool_calls" if tool_calls else "stop"

    return {
        "id": getattr(resp_obj, "id", request_id),
        "object": "chat.completion",
        "created": int(time.time()),
        "model": model,
        "choices": [
            {
                "index": 0,
                "message": message,
                "finish_reason": finish_reason,
            }
        ],
        "usage": usage_dict,
    }


CRAFTIF_API_BASE = "https://api.craftifai.com/v1"


@app.post("/v1/chat/completions")
async def chat_completions(post: ChatContext, request: Request):
    created_ts = time.time()
    request_id = f"chat-comp-{str(uuid.uuid4()).replace('-', '')[0:12]}"

    model = post.model or "gpt-5"
    use_responses_api = model in RESPONSES_API_MODELS

    logger.info(f"{request_id} model={model} endpoint={'responses' if use_responses_api else 'chat/completions'}")

    openai_messages = _build_openai_messages(post)

    async def stream_openai_response():
        try:
            auth_header = request.headers.get("Authorization")
            logger.info(f"DEBUG auth_header: {auth_header}")
            auth_key = auth_header.split(" ")[1] if auth_header and "Bearer " in auth_header else None
            
            # If the web UI manually provided the active session token, prioritize it over the Rust agent's dummy/fallback configuration
            global ACTIVE_JWT_TOKEN
            if ACTIVE_JWT_TOKEN:
                auth_key = ACTIVE_JWT_TOKEN
            elif not auth_key:
                # If no JWT is passed, fallback to environment variable
                auth_key = os.environ.get("OPENAI_API_KEY")
                if not auth_key:
                    raise ValueError("OPENAI_API_KEY environment variable not set and no Bearer token provided in request.")

            client = openai.AsyncOpenAI(
                api_key=auth_key,
                base_url=CRAFTIF_API_BASE + "/"
            )

            # ------------------------------------------------------------------
            # Branch A: models that require the v1/responses endpoint
            # ------------------------------------------------------------------
            if use_responses_api:
                # Convert chat-style messages to v1/responses "input" format.
                # The Responses API accepts a flat list of input items.
                input_items = []
                for msg in openai_messages:
                    role = msg["role"]
                    content = msg["content"]

                    # system messages become a "system" input item
                    if role == "system":
                        input_items.append({"type": "message", "role": "system", "content": content})
                    elif role == "user":
                        # content may be a string or a list (multimodal)
                        if isinstance(content, str):
                            input_items.append({"type": "message", "role": "user",
                                                "content": [{"type": "input_text", "text": content}]})
                        else:
                            # Already structured (e.g. multimodal list) – pass through
                            input_items.append({"type": "message", "role": "user", "content": content})
                    elif role == "assistant":
                        text = content if isinstance(content, str) else ""
                        if text:
                            input_items.append({"type": "message", "role": "assistant",
                                                "content": [{"type": "output_text", "text": text}]})
                        tool_calls_list = msg.get("tool_calls") or []
                        for tc in tool_calls_list:
                            fn = tc.get("function", {}) if isinstance(tc, dict) else {}
                            input_items.append({
                                "type": "function_call",
                                "call_id": (tc.get("id") or str(uuid.uuid4())) if isinstance(tc, dict) else str(uuid.uuid4()),
                                "name": fn.get("name", ""),
                                "arguments": fn.get("arguments", "{}"),
                            })
                    elif role == "tool":
                        # Function call results
                        input_items.append({
                            "type": "function_call_output",
                            "call_id": msg.get("tool_call_id", ""),
                            "output": content if isinstance(content, str) else json.dumps(content),
                        })

                responses_params: Dict[str, Any] = {
                    "model": model,
                    "input": input_items,
                }

                # Reasoning effort: default to "medium" so codex models stay
                # action-oriented rather than producing verbose planning text.
                reasoning_effort = post.reasoning_effort or "medium"
                responses_params["reasoning"] = {"effort": reasoning_effort}

                # Max output tokens
                if post.actual_max_tokens:
                    responses_params["max_output_tokens"] = post.actual_max_tokens

                # Map supported parameters
                if post.tools:
                    # Convert OpenAI-function-calling tools to Responses-API format
                    responses_tools = []
                    for tool in post.tools:
                        if isinstance(tool, dict) and tool.get("type") == "function":
                            fn = tool["function"]
                            responses_tools.append({
                                "type": "function",
                                "name": fn.get("name", ""),
                                "description": fn.get("description", ""),
                                "parameters": fn.get("parameters", {}),
                            })
                        else:
                            responses_tools.append(tool)
                    responses_params["tools"] = responses_tools
                    if post.tool_choice:
                        tc = post.tool_choice
                        # Chat completions uses {"type":"function","function":{"name":"foo"}}
                        # Responses API uses {"type":"function","name":"foo"} — convert if needed
                        if isinstance(tc, dict) and tc.get("type") == "function" and "function" in tc:
                            responses_params["tool_choice"] = {
                                "type": "function",
                                "name": tc["function"]["name"],
                            }
                        else:
                            responses_params["tool_choice"] = tc

                # streaming support via the Responses API
                # Note: .stream() already implies streaming – do NOT pass stream=True
                if post.stream:
                    tool_call_index = 0
                    output_item_tc_index: Dict[int, int] = {}

                    async with client.responses.stream(**responses_params) as stream:
                        async for event in stream:
                            event_type = getattr(event, "type", None)

                            if event_type == "response.output_text.delta":
                                delta_text = getattr(event, "delta", "")
                                chunk = {
                                    "id": request_id,
                                    "object": "chat.completion.chunk",
                                    "created": int(time.time()),
                                    "model": model,
                                    "choices": [{"index": 0,
                                                 "delta": {"role": "assistant", "content": delta_text},
                                                 "finish_reason": None}],
                                }
                                yield f"data: {json.dumps(chunk)}\n\n"

                            elif event_type == "response.output_item.added":
                                item = getattr(event, "item", None)
                                if item and getattr(item, "type", None) == "function_call":
                                    out_idx = getattr(event, "output_index", 0)
                                    tc_idx = tool_call_index
                                    output_item_tc_index[out_idx] = tc_idx
                                    tool_call_index += 1
                                    chunk = {
                                        "id": request_id,
                                        "object": "chat.completion.chunk",
                                        "created": int(time.time()),
                                        "model": model,
                                        "choices": [{"index": 0,
                                                     "delta": {
                                                         "role": "assistant",
                                                         "tool_calls": [{
                                                             "index": tc_idx,
                                                             "id": getattr(item, "call_id", str(uuid.uuid4())),
                                                             "type": "function",
                                                             "function": {
                                                                 "name": getattr(item, "name", ""),
                                                                 "arguments": "",
                                                             },
                                                         }],
                                                     },
                                                     "finish_reason": None}],
                                    }
                                    yield f"data: {json.dumps(chunk)}\n\n"

                            elif event_type == "response.function_call_arguments.delta":
                                out_idx = getattr(event, "output_index", 0)
                                tc_idx = output_item_tc_index.get(out_idx, 0)
                                delta_args = getattr(event, "delta", "")
                                chunk = {
                                    "id": request_id,
                                    "object": "chat.completion.chunk",
                                    "created": int(time.time()),
                                    "model": model,
                                    "choices": [{"index": 0,
                                                 "delta": {
                                                     "tool_calls": [{
                                                         "index": tc_idx,
                                                         "function": {"arguments": delta_args},
                                                     }],
                                                 },
                                                 "finish_reason": None}],
                                }
                                yield f"data: {json.dumps(chunk)}\n\n"

                            elif event_type == "response.completed":
                                resp = getattr(event, "response", None)
                                has_tc = any(
                                    getattr(it, "type", None) == "function_call"
                                    for it in getattr(resp, "output", [])
                                ) if resp else False
                                chunk = {
                                    "id": request_id,
                                    "object": "chat.completion.chunk",
                                    "created": int(time.time()),
                                    "model": model,
                                    "choices": [{"index": 0, "delta": {},
                                                 "finish_reason": "tool_calls" if has_tc else "stop"}],
                                }
                                yield f"data: {json.dumps(chunk)}\n\n"

                    yield "data: [DONE]\n\n"
                else:
                    # Non-streaming Responses API call
                    response = await client.responses.create(**responses_params)
                    normalized = _responses_to_chat_completion(response, model, request_id)
                    yield json.dumps(normalized)

            # ------------------------------------------------------------------
            # Branch B: standard chat/completions models (gpt-5.1, gpt-5.2, etc.)
            # ------------------------------------------------------------------
            else:
                openai_params: Dict[str, Any] = {
                    "model": model,
                    "messages": openai_messages,
                    "temperature": clamp(0, 2, post.temperature),
                    "top_p": clamp(0.0, 1.0, post.top_p),
                    "n": post.n,
                    "stream": post.stream,
                }

                if post.tools:
                    openai_params["tools"] = post.tools
                if post.tool_choice:
                    openai_params["tool_choice"] = post.tool_choice
                if post.stop:
                    openai_params["stop"] = post.stop if isinstance(post.stop, list) else [post.stop]

                if post.stream:
                    # CraftifAI gateway streaming is unreliable with OpenAI SDK 2.x;
                    # fetch the full completion and emit a single SSE chunk.
                    stream_params = dict(openai_params)
                    stream_params["stream"] = False
                    response = await client.chat.completions.create(**stream_params)
                    data = response.model_dump()
                    choice = (data.get("choices") or [{}])[0]
                    message = choice.get("message") or {}
                    delta: Dict[str, Any] = {"role": message.get("role", "assistant")}
                    if message.get("content"):
                        delta["content"] = message["content"]
                    if message.get("tool_calls"):
                        delta["tool_calls"] = message["tool_calls"]
                    chunk = {
                        "id": data.get("id", request_id),
                        "object": "chat.completion.chunk",
                        "created": data.get("created", int(time.time())),
                        "model": data.get("model", model),
                        "choices": [{
                            "index": 0,
                            "delta": delta,
                            "finish_reason": choice.get("finish_reason"),
                        }],
                    }
                    yield f"data: {json.dumps(chunk)}\n\n"
                    yield "data: [DONE]\n\n"
                else:
                    response = await client.chat.completions.create(**openai_params)
                    yield json.dumps(response.model_dump())

            logger.info(f"{request_id} finished in {(time.time() - created_ts) * 1000:.1f}ms")

        except Exception as e:
            logger.error(f"{request_id} error: {type(e).__name__}: {e}", exc_info=True)
            error_msg = {"error": {"message": str(e), "type": "api_error"}}
            if post.stream:
                yield f"data: {json.dumps(error_msg)}\n\n"
            else:
                yield json.dumps(error_msg)

    media_type = "text/event-stream" if post.stream else "application/json"
    return StreamingResponse(stream_openai_response(), media_type=media_type)
