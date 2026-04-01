import asyncio
import serial
import serial.tools.list_ports
from typing import Optional
from fastapi import APIRouter, WebSocket, WebSocketDisconnect, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel

from refact_webgui.webgui.selfhost_webutils import log

__all__ = ["TabUartRouter"]


class UartConfigRequest(BaseModel):
    port: Optional[str] = None
    baud_rate: int = 115200
    parity: str = "N"
    stop_bits: int = 1
    bytesize: int = 8


class TabUartRouter(APIRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._setup_routes()
        self._active_connections: set[WebSocket] = set()
        self._serial_connection: Optional[serial.Serial] = None
        self._uart_task: Optional[asyncio.Task] = None

    def _setup_routes(self):
        self.add_api_route("/ws/uart-stream", self._uart_websocket, methods=["GET"])
        self.add_api_route("/tab-uart-config", self._get_uart_config, methods=["GET"])
        self.add_api_route("/tab-uart-config", self._set_uart_config, methods=["POST"])
        self.add_api_route("/tab-uart-ports", self._list_ports, methods=["GET"])

    async def _uart_websocket(self, websocket: WebSocket):
        """WebSocket endpoint for streaming UART data"""
        await websocket.accept()
        self._active_connections.add(websocket)
        
        try:
            # Try to connect to serial port
            config = await self._get_uart_config_internal()
            if config.get("port"):
                try:
                    self._serial_connection = serial.Serial(
                        port=config["port"],
                        baudrate=config.get("baud_rate", 115200),
                        parity=config.get("parity", "N"),
                        stopbits=config.get("stop_bits", 1),
                        bytesize=config.get("bytesize", 8),
                        timeout=1,
                    )
                    
                    # Start reading from serial port
                    self._uart_task = asyncio.create_task(
                        self._read_uart_and_broadcast(websocket)
                    )
                    
                    await websocket.send_json({
                        "type": "info",
                        "content": f"Connected to {config['port']} at {config.get('baud_rate', 115200)} baud"
                    })
                except Exception as e:
                    await websocket.send_json({
                        "type": "error",
                        "content": f"Failed to connect to serial port: {str(e)}"
                    })
            else:
                await websocket.send_json({
                    "type": "warning",
                    "content": "No UART port configured. Please configure a port in settings."
                })

            # Keep connection alive and handle incoming messages
            while True:
                try:
                    data = await websocket.receive_text()
                    # Handle commands from client (e.g., pause, resume, clear)
                    if data == "ping":
                        await websocket.send_json({"type": "info", "content": "pong"})
                except WebSocketDisconnect:
                    break
                    
        except Exception as e:
            log(f"UART WebSocket error: {e}")
        finally:
            self._active_connections.discard(websocket)
            if self._uart_task:
                self._uart_task.cancel()
            if self._serial_connection and self._serial_connection.is_open:
                self._serial_connection.close()

    async def _read_uart_and_broadcast(self, websocket: WebSocket):
        """Read from serial port and send to WebSocket"""
        buffer = ""
        try:
            while True:
                if self._serial_connection and self._serial_connection.is_open:
                    try:
                        # Read available data
                        if self._serial_connection.in_waiting > 0:
                            data = self._serial_connection.read(
                                self._serial_connection.in_waiting
                            ).decode("utf-8", errors="replace")
                            buffer += data
                            
                            # Send complete lines
                            while "\n" in buffer:
                                line, buffer = buffer.split("\n", 1)
                                if line.strip():
                                    await websocket.send_json({
                                        "type": "info",
                                        "content": line.strip()
                                    })
                        else:
                            await asyncio.sleep(0.1)
                    except Exception as e:
                        await websocket.send_json({
                            "type": "error",
                            "content": f"Serial read error: {str(e)}"
                        })
                        await asyncio.sleep(1)
                else:
                    await asyncio.sleep(1)
        except asyncio.CancelledError:
            pass
        except Exception as e:
            log(f"UART read task error: {e}")

    async def _get_uart_config_internal(self):
        """Get UART configuration from cache or file"""
        # TODO: Load from config file or use defaults
        # For now, try to auto-detect common ports
        ports = serial.tools.list_ports.comports()
        common_ports = ["/dev/ttyUSB0", "/dev/ttyACM0", "/dev/ttyS0"]
        
        for port in common_ports:
            if any(p.device == port for p in ports):
                return {
                    "port": port,
                    "baud_rate": 115200,
                    "parity": "N",
                    "stop_bits": 1,
                    "bytesize": 8,
                }
        
        # Return first available port or None
        if ports:
            return {
                "port": ports[0].device,
                "baud_rate": 115200,
                "parity": "N",
                "stop_bits": 1,
                "bytesize": 8,
            }
        
        return {
            "port": None,
            "baud_rate": 115200,
            "parity": "N",
            "stop_bits": 1,
            "bytesize": 8,
        }

    async def _get_uart_config(self):
        """Get current UART configuration"""
        try:
            config = await self._get_uart_config_internal()
            return JSONResponse({
                "success": True,
                "config": config,
            })
        except Exception as e:
            log(f"Error getting UART config: {e}")
            return JSONResponse(
                {"success": False, "error": str(e)},
                status_code=500,
            )

    async def _set_uart_config(self, request: UartConfigRequest):
        """Set UART configuration"""
        try:
            # TODO: Save to config file
            # For now, just validate the configuration
            if request.port:
                # Test if port is accessible
                try:
                    test_serial = serial.Serial(
                        port=request.port,
                        baudrate=request.baud_rate,
                        timeout=0.1,
                    )
                    test_serial.close()
                except Exception as e:
                    return JSONResponse(
                        {"success": False, "error": f"Cannot access port: {str(e)}"},
                        status_code=400,
                    )
            
            return JSONResponse({
                "success": True,
                "message": "Configuration updated",
            })
        except Exception as e:
            log(f"Error setting UART config: {e}")
            return JSONResponse(
                {"success": False, "error": str(e)},
                status_code=500,
            )

    async def _list_ports(self):
        """List available serial ports"""
        try:
            ports = serial.tools.list_ports.comports()
            port_list = [
                {
                    "device": port.device,
                    "description": port.description,
                    "manufacturer": port.manufacturer,
                    "hwid": port.hwid,
                }
                for port in ports
            ]
            return JSONResponse({
                "success": True,
                "ports": port_list,
            })
        except Exception as e:
            log(f"Error listing ports: {e}")
            return JSONResponse(
                {"success": False, "error": str(e)},
                status_code=500,
            )







