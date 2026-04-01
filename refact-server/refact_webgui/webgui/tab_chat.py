import asyncio
import json
import os
from typing import Optional
from fastapi import APIRouter, Request, HTTPException
from fastapi.responses import JSONResponse, StreamingResponse
from pydantic import BaseModel
import httpx

from refact_webgui.webgui.selfhost_webutils import log

__all__ = ["TabChatRouter"]


class ChatMessage(BaseModel):
    role: str  # "user" or "assistant"
    content: str
    tool_calls: Optional[list] = None
    tool_call_id: Optional[str] = None


class ChatRequest(BaseModel):
    messages: list
    model: Optional[str] = None
    stream: bool = False
    chat_id: Optional[str] = None
    workspace_folder: Optional[str] = None


class TabChatRouter(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # Try to detect agent port, default to 8001
        self._agent_port = int(os.environ.get("REFACT_AGENT_PORT", "8001"))
        self._setup_routes()

    def _setup_routes(self):
        self.add_api_route("/tab-chat-send", self._send_chat, methods=["POST"])
        self.add_api_route("/tab-chat-caps", self._get_caps, methods=["GET"])
        self.add_api_route("/tab-chat-tools", self._get_tools, methods=["GET"])
        self.add_api_route("/tab-chat-ping", self._ping_agent, methods=["GET"])
        self.add_api_route("/tab-chat-detect-port", self._detect_port, methods=["GET"])

    def _get_agent_url(self, endpoint: str) -> str:
        """Get the full URL for the refact agent endpoint"""
        return f"http://127.0.0.1:{self._agent_port}{endpoint}"

    async def _detect_agent_port(self) -> Optional[int]:
        """Try to detect which port the refact agent is running on"""
        # First, try to get port from process list (faster)
        import subprocess
        try:
            result = subprocess.run(
                ['ps', 'aux'], 
                capture_output=True, 
                text=True, 
                timeout=1
            )
            for line in result.stdout.split('\n'):
                if 'refact-lsp' in line and '--http-port=' in line:
                    # Extract port from command line
                    import re
                    match = re.search(r'--http-port=(\d+)', line)
                    if match:
                        port = int(match.group(1))
                        # Verify it's actually responding
                        try:
                            async with httpx.AsyncClient(timeout=0.5) as client:
                                response = await client.get(f"http://127.0.0.1:{port}/v1/ping", timeout=0.5)
                                if response.status_code == 200:
                                    return port
                        except:
                            pass
        except:
            pass
        
        # Fallback: try common ports sequentially (faster than parallel for small ranges)
        # Try 8001 first, then sample ports in 8100-9100 range
        ports_to_try = [8001] + list(range(8100, 9110, 1))  # Check every port
        
        async with httpx.AsyncClient(timeout=0.3) as client:
            for port in ports_to_try:
                try:
                    response = await client.get(f"http://127.0.0.1:{port}/v1/ping", timeout=0.3)
                    if response.status_code == 200:
                        return port
                except:
                    continue
        return None

    async def _ping_agent(self):
        """Check if refact agent is running"""
        # First try the configured port
        try:
            async with httpx.AsyncClient(timeout=2.0) as client:
                response = await client.get(self._get_agent_url("/v1/ping"))
                if response.status_code == 200:
                    return JSONResponse({
                        "success": True,
                        "connected": True,
                        "port": self._agent_port,
                        "message": response.json().get("message", "pong")
                    })
        except:
            pass
        
        # If configured port fails, try to auto-detect
        detected_port = await self._detect_agent_port()
        if detected_port:
            self._agent_port = detected_port
            try:
                async with httpx.AsyncClient(timeout=2.0) as client:
                    response = await client.get(self._get_agent_url("/v1/ping"))
                    if response.status_code == 200:
                        return JSONResponse({
                            "success": True,
                            "connected": True,
                            "port": self._agent_port,
                            "message": response.json().get("message", "pong"),
                            "auto_detected": True
                        })
            except:
                pass
        
        # If still not found, return error
        return JSONResponse({
            "success": False,
            "connected": False,
            "error": f"Could not connect to refact agent. Tried port {self._agent_port} and common ports. Make sure the agent is running with: refact <workspace_path>"
        })

    async def _detect_port(self):
        """Manually trigger port detection"""
        detected_port = await self._detect_agent_port()
        if detected_port:
            self._agent_port = detected_port
            return JSONResponse({
                "success": True,
                "port": detected_port,
                "message": f"Agent detected on port {detected_port}"
            })
        else:
            return JSONResponse({
                "success": False,
                "error": "Could not detect agent port. Make sure the agent is running."
            })

    async def _get_caps(self):
        """Get capabilities from refact agent"""
        try:
            # Verify port is correct before making request
            try:
                async with httpx.AsyncClient(timeout=2.0) as test_client:
                    test_response = await test_client.get(self._get_agent_url("/v1/ping"), timeout=2.0)
                    if test_response.status_code != 200:
                        # Port might be wrong, try to detect
                        detected_port = await self._detect_agent_port()
                        if detected_port:
                            self._agent_port = detected_port
            except:
                # Connection failed, try to detect port
                detected_port = await self._detect_agent_port()
                if detected_port:
                    self._agent_port = detected_port
                else:
                    return JSONResponse({
                        "success": False,
                        "error": f"Could not connect to refact agent on port {self._agent_port}"
                    }, status_code=503)
            
            async with httpx.AsyncClient(timeout=10.0) as client:
                response = await client.get(self._get_agent_url("/v1/caps"))
                if response.status_code == 200:
                    return JSONResponse({
                        "success": True,
                        "caps": response.json()
                    })
                else:
                    return JSONResponse({
                        "success": False,
                        "error": f"Agent returned status {response.status_code}"
                    }, status_code=500)
        except httpx.ConnectError:
            return JSONResponse({
                "success": False,
                "error": f"Could not connect to refact agent on port {self._agent_port}"
            }, status_code=503)
        except Exception as e:
            log(f"Error getting caps: {e}")
            return JSONResponse({
                "success": False,
                "error": str(e)
            }, status_code=500)

    async def _get_tools(self):
        """Get available tools from refact agent"""
        try:
            # Verify port is correct before making request
            try:
                async with httpx.AsyncClient(timeout=2.0) as test_client:
                    test_response = await test_client.get(self._get_agent_url("/v1/ping"), timeout=2.0)
                    if test_response.status_code != 200:
                        # Port might be wrong, try to detect
                        detected_port = await self._detect_agent_port()
                        if detected_port:
                            self._agent_port = detected_port
            except:
                # Connection failed, try to detect port
                detected_port = await self._detect_agent_port()
                if detected_port:
                    self._agent_port = detected_port
                else:
                    return JSONResponse({
                        "success": False,
                        "error": f"Could not connect to refact agent on port {self._agent_port}"
                    }, status_code=503)
            
            async with httpx.AsyncClient(timeout=10.0) as client:
                response = await client.get(self._get_agent_url("/v1/tools"))
                if response.status_code == 200:
                    return JSONResponse({
                        "success": True,
                        "tools": response.json()
                    })
                else:
                    return JSONResponse({
                        "success": False,
                        "error": f"Agent returned status {response.status_code}"
                    }, status_code=500)
        except httpx.ConnectError:
            return JSONResponse({
                "success": False,
                "error": f"Could not connect to refact agent on port {self._agent_port}"
            }, status_code=503)
        except Exception as e:
            log(f"Error getting tools: {e}")
            return JSONResponse({
                "success": False,
                "error": str(e)
            }, status_code=500)

    async def _send_chat(self, request: ChatRequest):
        """Send chat message to refact agent"""
        try:
            # First, make sure we have the correct port
            # Try to ping first to verify connection
            try:
                async with httpx.AsyncClient(timeout=2.0) as test_client:
                    test_response = await test_client.get(self._get_agent_url("/v1/ping"), timeout=2.0)
                    if test_response.status_code != 200:
                        # Port might be wrong, try to detect
                        detected_port = await self._detect_agent_port()
                        if detected_port:
                            self._agent_port = detected_port
            except:
                # Connection failed, try to detect port
                detected_port = await self._detect_agent_port()
                if detected_port:
                    self._agent_port = detected_port
                else:
                    return JSONResponse({
                        "success": False,
                        "error": f"Could not connect to refact agent. Tried port {self._agent_port}. Make sure the agent is running with: refact <workspace_path>"
                    }, status_code=503)
            
            # Prepare request body - must match ChatPost structure
            body = {
                "messages": request.messages,
                "stream": request.stream if request.stream else None,
            }
            
            # Model is required - use default if not provided
            if request.model:
                body["model"] = request.model
            else:
                # Try to get default model from caps
                try:
                    async with httpx.AsyncClient(timeout=5.0) as caps_client:
                        caps_response = await caps_client.get(self._get_agent_url("/v1/caps"), timeout=5.0)
                        if caps_response.status_code == 200:
                            caps = caps_response.json()
                            # Try to get default model
                            if "defaults" in caps and "chat_light_model" in caps["defaults"]:
                                body["model"] = caps["defaults"]["chat_light_model"]
                            elif "chat_models" in caps and len(caps["chat_models"]) > 0:
                                # Use first available chat model
                                first_model = list(caps["chat_models"].keys())[0]
                                body["model"] = first_model
                            else:
                                return JSONResponse({
                                    "success": False,
                                    "error": "No model specified and no default model available"
                                }, status_code=400)
                        else:
                            return JSONResponse({
                                "success": False,
                                "error": "Could not get capabilities to determine default model"
                            }, status_code=500)
                except Exception as e:
                    log(f"Error getting default model: {e}")
                    return JSONResponse({
                        "success": False,
                        "error": f"Model is required. Please select a model or ensure agent has default model configured. Error: {str(e)}"
                    }, status_code=400)
            
            # Build meta object with required fields
            import uuid
            meta = {
                "chat_id": request.chat_id or f"webui-{uuid.uuid4().hex[:10]}",
                "request_attempt_id": "",
                "chat_remote": False,
                "chat_mode": "EXPLORE",  # Default chat mode
                "current_config_file": ""
            }
            
            if request.workspace_folder:
                meta["current_config_file"] = request.workspace_folder
            
            body["meta"] = meta

            async with httpx.AsyncClient(timeout=300.0) as client:
                if request.stream:
                    # Streaming response
                    async with client.stream(
                        "POST",
                        self._get_agent_url("/v1/chat"),
                        json=body,
                        headers={"Content-Type": "application/json"},
                        timeout=300.0
                    ) as response:
                        if response.status_code != 200:
                            error_text = await response.aread()
                            return JSONResponse({
                                "success": False,
                                "error": f"Agent returned status {response.status_code}: {error_text.decode()}"
                            }, status_code=response.status_code)
                        
                        async def generate():
                            try:
                                # Use aiter_bytes for better control over streaming
                                async for chunk in response.aiter_bytes():
                                    if chunk:
                                        yield chunk
                            except Exception as e:
                                log(f"Error streaming response: {e}")
                                # Send error as SSE format
                                error_msg = json.dumps({"error": str(e)})
                                yield f"data: {error_msg}\n\n".encode()
                        
                        return StreamingResponse(
                            generate(),
                            media_type="text/event-stream",
                            headers={
                                "Cache-Control": "no-cache",
                                "Connection": "keep-alive",
                                "X-Accel-Buffering": "no",  # Disable nginx buffering
                            }
                        )
                else:
                    # Non-streaming response
                    response = await client.post(
                        self._get_agent_url("/v1/chat"),
                        json=body,
                        headers={"Content-Type": "application/json"},
                        timeout=300.0
                    )
                    
                    if response.status_code == 200:
                        return JSONResponse({
                            "success": True,
                            "response": response.json()
                        })
                    else:
                        error_text = response.text
                        return JSONResponse({
                            "success": False,
                            "error": f"Agent returned status {response.status_code}: {error_text}"
                        }, status_code=response.status_code)
                        
        except httpx.ConnectError:
            return JSONResponse({
                "success": False,
                "error": f"Could not connect to refact agent on port {self._agent_port}. Make sure the agent is running with: refact <workspace_path>"
            }, status_code=503)
        except Exception as e:
            log(f"Error sending chat: {e}")
            return JSONResponse({
                "success": False,
                "error": str(e)
            }, status_code=500)

