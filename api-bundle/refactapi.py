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
            base_url="https://api.intelligentedgesystems.com/v1/v1/",
            timeout=120.0,
            max_retries=3
        )

    if _client is None:
        api_key = os.environ.get("OPENAI_API_KEY")
        if not api_key:
            raise ValueError("OPENAI_API_KEY environment variable not set")
        _client = openai.AsyncOpenAI(
            api_key=api_key,
            base_url="https://api.intelligentedgesystems.com/v1/v1/",
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
                base_url="https://api.intelligentedgesystems.com/v1/v1/"
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
            # NOTE: The CraftifAI gateway does NOT support SSE streaming.
            # We always call upstream with stream=False to get a complete JSON
            # response, then simulate SSE chunks locally if the caller wants
            # streaming. This prevents the OpenAI SDK from trying to parse
            # a non-SSE response as an SSE stream (which yields 0 chunks).
            # ------------------------------------------------------------------
            else:
                openai_params: Dict[str, Any] = {
                    "model": model,
                    "messages": openai_messages,
                    "temperature": clamp(0, 2, post.temperature),
                    "top_p": clamp(0.0, 1.0, post.top_p),
                    "n": post.n,
                    "stream": False,  # Always non-streaming to CraftifAI gateway
                }

                if post.tools:
                    openai_params["tools"] = post.tools
                if post.tool_choice:
                    openai_params["tool_choice"] = post.tool_choice
                if post.stop:
                    openai_params["stop"] = post.stop if isinstance(post.stop, list) else [post.stop]

                response = await client.chat.completions.create(**openai_params)
                response_dict = response.model_dump()

                logger.info(f"{request_id} upstream response received, model={response_dict.get('model','?')}")

                if post.stream:
                    # Simulate SSE streaming from the complete response
                    # First chunk: role assignment
                    for choice in response_dict.get("choices", []):
                        msg = choice.get("message", {})
                        content = msg.get("content") or ""
                        tool_calls = msg.get("tool_calls")
                        finish_reason = choice.get("finish_reason", "stop")

                        # Send role chunk
                        role_chunk = {
                            "id": response_dict.get("id", request_id),
                            "object": "chat.completion.chunk",
                            "created": response_dict.get("created", int(time.time())),
                            "model": model,
                            "choices": [{"index": choice.get("index", 0),
                                         "delta": {"role": "assistant", "content": ""},
                                         "finish_reason": None}],
                        }
                        yield f"data: {json.dumps(role_chunk)}\n\n"

                        # Stream content in small chunks to simulate real-time
                        if content:
                            chunk_size = 20  # characters per SSE event
                            for i in range(0, len(content), chunk_size):
                                text_piece = content[i:i + chunk_size]
                                chunk = {
                                    "id": response_dict.get("id", request_id),
                                    "object": "chat.completion.chunk",
                                    "created": response_dict.get("created", int(time.time())),
                                    "model": model,
                                    "choices": [{"index": choice.get("index", 0),
                                                 "delta": {"content": text_piece},
                                                 "finish_reason": None}],
                                }
                                yield f"data: {json.dumps(chunk)}\n\n"

                        # Stream tool calls if present
                        if tool_calls:
                            for tc_idx, tc in enumerate(tool_calls):
                                # Send tool call header
                                tc_chunk = {
                                    "id": response_dict.get("id", request_id),
                                    "object": "chat.completion.chunk",
                                    "created": response_dict.get("created", int(time.time())),
                                    "model": model,
                                    "choices": [{"index": choice.get("index", 0),
                                                 "delta": {
                                                     "tool_calls": [{
                                                         "index": tc_idx,
                                                         "id": tc.get("id", ""),
                                                         "type": "function",
                                                         "function": {
                                                             "name": tc.get("function", {}).get("name", ""),
                                                             "arguments": "",
                                                         },
                                                     }],
                                                 },
                                                 "finish_reason": None}],
                                }
                                yield f"data: {json.dumps(tc_chunk)}\n\n"
                                # Send tool call arguments
                                args = tc.get("function", {}).get("arguments", "")
                                if args:
                                    args_chunk = {
                                        "id": response_dict.get("id", request_id),
                                        "object": "chat.completion.chunk",
                                        "created": response_dict.get("created", int(time.time())),
                                        "model": model,
                                        "choices": [{"index": choice.get("index", 0),
                                                     "delta": {
                                                         "tool_calls": [{
                                                             "index": tc_idx,
                                                             "function": {"arguments": args},
                                                         }],
                                                     },
                                                     "finish_reason": None}],
                                    }
                                    yield f"data: {json.dumps(args_chunk)}\n\n"

                        # Send finish chunk
                        done_chunk = {
                            "id": response_dict.get("id", request_id),
                            "object": "chat.completion.chunk",
                            "created": response_dict.get("created", int(time.time())),
                            "model": model,
                            "choices": [{"index": choice.get("index", 0),
                                         "delta": {},
                                         "finish_reason": finish_reason}],
                        }
                        yield f"data: {json.dumps(done_chunk)}\n\n"

                    yield "data: [DONE]\n\n"
                else:
                    yield json.dumps(response_dict)

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
