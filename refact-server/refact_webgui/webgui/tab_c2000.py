import asyncio
import os
import json
import subprocess
from pathlib import Path
from typing import Dict, List, Optional
from fastapi import APIRouter, Request, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel

from refact_webgui.webgui.selfhost_webutils import log

__all__ = ["TabC2000Router"]


class C2000ConfigRequest(BaseModel):
    ccs_path: Optional[str] = None
    c2000ware_path: Optional[str] = None
    workspace_path: Optional[str] = None
    target_device: Optional[str] = None
    debug_probe: Optional[str] = None


class C2000ProjectRequest(BaseModel):
    project_name: str
    example_path: Optional[str] = None


class C2000BuildRequest(BaseModel):
    project_name: str
    configuration: str = "FLASH"


class C2000FlashRequest(BaseModel):
    project_name: str
    binary_path: Optional[str] = None


class C2000DebugRequest(BaseModel):
    issue_description: str
    log_files: List[str]
    project_name: str


class TabC2000Router(APIRouter):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._config_cache = None
        self._setup_routes()

    def _setup_routes(self):
        self.add_api_route("/tab-c2000-config-get", self._get_config, methods=["GET"])
        self.add_api_route("/tab-c2000-config-set", self._set_config, methods=["POST"])
        self.add_api_route("/tab-c2000-config-validate", self._validate_config, methods=["GET"])
        self.add_api_route("/tab-c2000-examples-list", self._list_examples, methods=["GET"])
        self.add_api_route("/tab-c2000-projects-list", self._list_projects, methods=["GET"])
        self.add_api_route("/tab-c2000-project-create", self._create_project, methods=["POST"])
        self.add_api_route("/tab-c2000-build", self._build_project, methods=["POST"])
        self.add_api_route("/tab-c2000-flash", self._flash_project, methods=["POST"])
        self.add_api_route("/tab-c2000-target-detect", self._detect_target, methods=["GET"])
        self.add_api_route("/tab-c2000-debug", self._debug_issue, methods=["POST"])
        self.add_api_route("/tab-c2000-logs-analyze", self._analyze_logs, methods=["POST"])

    def _get_config_file_path(self) -> Path:
        """Get the path to C2000 config file"""
        cache_dir = Path.home() / ".cache" / "refact"
        cache_dir.mkdir(parents=True, exist_ok=True)
        return cache_dir / "c2000_tools.yaml"

    async def _get_config(self):
        """Get C2000 configuration"""
        try:
            config_file = self._get_config_file_path()
            if config_file.exists():
                try:
                    import yaml
                except ImportError:
                    # Fallback to JSON if yaml not available
                    import json
                    with open(config_file, 'r') as f:
                        config = json.load(f) or {}
                else:
                    with open(config_file, 'r') as f:
                        config = yaml.safe_load(f) or {}
                
                c2000_config = config.get('c2000_config', {})
                return JSONResponse({
                    "success": True,
                    "config": {
                        "ccs_path": c2000_config.get("ccs_path", ""),
                        "c2000ware_path": c2000_config.get("c2000ware_path", ""),
                        "workspace_path": c2000_config.get("workspace_path", ""),
                        "target_device": c2000_config.get("target_device", "F28P65x"),
                        "debug_probe": c2000_config.get("debug_probe", "XDS110"),
                    }
                })
            else:
                return JSONResponse({
                    "success": True,
                    "config": {
                        "ccs_path": "",
                        "c2000ware_path": "",
                        "workspace_path": "",
                        "target_device": "F28P65x",
                        "debug_probe": "XDS110",
                    }
                })
        except Exception as e:
            log(f"Error getting C2000 config: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _set_config(self, request: C2000ConfigRequest):
        """Set C2000 configuration"""
        try:
            config_file = self._get_config_file_path()
            try:
                import yaml
                use_yaml = True
            except ImportError:
                use_yaml = False
            
            # Load existing config or create new
            if config_file.exists():
                if use_yaml:
                    with open(config_file, 'r') as f:
                        config = yaml.safe_load(f) or {}
                else:
                    import json
                    with open(config_file, 'r') as f:
                        config = json.load(f) or {}
            else:
                config = {}
            
            # Update c2000_config section
            if 'c2000_config' not in config:
                config['c2000_config'] = {}
            
            c2000_config = config['c2000_config']
            if request.ccs_path:
                c2000_config['ccs_path'] = request.ccs_path
            if request.c2000ware_path:
                c2000_config['c2000ware_path'] = request.c2000ware_path
            if request.workspace_path:
                c2000_config['workspace_path'] = request.workspace_path
            if request.target_device:
                c2000_config['target_device'] = request.target_device
            if request.debug_probe:
                c2000_config['debug_probe'] = request.debug_probe
            
            # Save config
            if use_yaml:
                with open(config_file, 'w') as f:
                    yaml.dump(config, f, default_flow_style=False)
            else:
                import json
                with open(config_file, 'w') as f:
                    json.dump(config, f, indent=2)
            
            return JSONResponse({"success": True, "message": "Configuration saved"})
        except Exception as e:
            log(f"Error setting C2000 config: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _validate_config(self):
        """Validate C2000 configuration"""
        try:
            config_file = self._get_config_file_path()
            if not config_file.exists():
                return JSONResponse({
                    "success": False,
                    "valid": False,
                    "errors": ["Configuration file not found"]
                })
            
            try:
                import yaml
                with open(config_file, 'r') as f:
                    config = yaml.safe_load(f) or {}
            except ImportError:
                import json
                with open(config_file, 'r') as f:
                    config = json.load(f) or {}
            
            c2000_config = config.get('c2000_config', {})
            errors = []
            warnings = []
            
            # Validate paths
            ccs_path = c2000_config.get('ccs_path', '')
            if not ccs_path:
                errors.append("CCS path not configured")
            elif not os.path.exists(ccs_path):
                errors.append(f"CCS path does not exist: {ccs_path}")
            
            c2000ware_path = c2000_config.get('c2000ware_path', '')
            if not c2000ware_path:
                errors.append("C2000Ware path not configured")
            elif not os.path.exists(c2000ware_path):
                errors.append(f"C2000Ware path does not exist: {c2000ware_path}")
            
            workspace_path = c2000_config.get('workspace_path', '')
            if not workspace_path:
                warnings.append("Workspace path not configured")
            elif not os.path.exists(workspace_path):
                warnings.append(f"Workspace path does not exist: {workspace_path}")
            
            return JSONResponse({
                "success": True,
                "valid": len(errors) == 0,
                "errors": errors,
                "warnings": warnings
            })
        except Exception as e:
            log(f"Error validating C2000 config: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _list_examples(self):
        """List available C2000Ware examples"""
        try:
            config_file = self._get_config_file_path()
            if not config_file.exists():
                return JSONResponse({"success": False, "error": "Configuration not found"})
            
            try:
                import yaml
                with open(config_file, 'r') as f:
                    config = yaml.safe_load(f) or {}
            except ImportError:
                import json
                with open(config_file, 'r') as f:
                    config = json.load(f) or {}
            
            c2000ware_path = config.get('c2000_config', {}).get('c2000ware_path', '')
            if not c2000ware_path or not os.path.exists(c2000ware_path):
                return JSONResponse({"success": False, "error": "C2000Ware path not configured or invalid"})
            
            # Search for .projectspec files
            examples = []
            for root, dirs, files in os.walk(c2000ware_path):
                for file in files:
                    if file.endswith('.projectspec'):
                        rel_path = os.path.relpath(os.path.join(root, file), c2000ware_path)
                        examples.append({
                            "name": file.replace('.projectspec', ''),
                            "path": os.path.join(root, file),
                            "relative_path": rel_path
                        })
            
            return JSONResponse({"success": True, "examples": examples[:100]})  # Limit to 100
        except Exception as e:
            log(f"Error listing examples: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _list_projects(self):
        """List existing C2000 projects"""
        try:
            config_file = self._get_config_file_path()
            if not config_file.exists():
                return JSONResponse({"success": False, "error": "Configuration not found"})
            
            try:
                import yaml
                with open(config_file, 'r') as f:
                    config = yaml.safe_load(f) or {}
            except ImportError:
                import json
                with open(config_file, 'r') as f:
                    config = json.load(f) or {}
            
            workspace_path = config.get('c2000_config', {}).get('workspace_path', '')
            if not workspace_path or not os.path.exists(workspace_path):
                return JSONResponse({"success": True, "projects": []})
            
            projects = []
            for item in os.listdir(workspace_path):
                item_path = os.path.join(workspace_path, item)
                if os.path.isdir(item_path):
                    # Check if it's a CCS project
                    if any(f.endswith('.projectspec') for f in os.listdir(item_path) if os.path.isfile(os.path.join(item_path, f))):
                        projects.append({
                            "name": item,
                            "path": item_path
                        })
            
            return JSONResponse({"success": True, "projects": projects})
        except Exception as e:
            log(f"Error listing projects: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _create_project(self, request: C2000ProjectRequest):
        """Create a new C2000 project"""
        try:
            # This would call the actual c2000_project_create tool
            # For now, return a placeholder
            return JSONResponse({
                "success": True,
                "message": f"Project creation initiated for {request.project_name}",
                "project_name": request.project_name
            })
        except Exception as e:
            log(f"Error creating project: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _build_project(self, request: C2000BuildRequest):
        """Build a C2000 project"""
        try:
            # This would call the actual c2000_build tool
            return JSONResponse({
                "success": True,
                "message": f"Build initiated for {request.project_name} ({request.configuration})",
                "project_name": request.project_name,
                "configuration": request.configuration
            })
        except Exception as e:
            log(f"Error building project: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _flash_project(self, request: C2000FlashRequest):
        """Flash a C2000 project"""
        try:
            # This would call the actual c2000_flash tool
            return JSONResponse({
                "success": True,
                "message": f"Flash initiated for {request.project_name}",
                "project_name": request.project_name
            })
        except Exception as e:
            log(f"Error flashing project: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _detect_target(self):
        """Detect connected C2000 target"""
        try:
            # This would call the actual c2000_target_detect tool
            return JSONResponse({
                "success": True,
                "targets": [
                    {
                        "name": "LAUNCHXL-F28P65X",
                        "device": "F28P65x",
                        "probe": "XDS110",
                        "connected": True
                    }
                ]
            })
        except Exception as e:
            log(f"Error detecting target: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _debug_issue(self, request: C2000DebugRequest):
        """Run AI debugging assistant"""
        try:
            # Call the c2000_debug_assistant.py script
            script_path = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(__file__)))), 
                                      "c2000_debug_assistant.py")
            
            if not os.path.exists(script_path):
                return JSONResponse({
                    "success": False,
                    "error": "Debug assistant script not found"
                })
            
            # Run the debug assistant
            import tempfile
            with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
                output_file = f.name
            
            cmd = [
                "python3", script_path,
                "--issue_description", request.issue_description,
                "--log_files", ",".join(request.log_files),
                "--project_name", request.project_name,
                "--output_file", output_file
            ]
            
            process = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            
            stdout, stderr = await process.communicate()
            
            if os.path.exists(output_file):
                with open(output_file, 'r') as f:
                    report = json.load(f)
                os.unlink(output_file)
                return JSONResponse({"success": True, "report": report})
            else:
                return JSONResponse({
                    "success": False,
                    "error": stderr.decode() if stderr else "Debug assistant failed"
                })
        except Exception as e:
            log(f"Error running debug assistant: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

    async def _analyze_logs(self, request: Request):
        """Analyze log files"""
        try:
            data = await request.json()
            log_files = data.get('log_files', [])
            
            if not log_files:
                return JSONResponse({"success": False, "error": "No log files provided"})
            
            analysis = {
                "errors": [],
                "warnings": [],
                "communication_issues": [],
                "timing_issues": [],
                "memory_issues": []
            }
            
            for log_file in log_files:
                if os.path.exists(log_file):
                    with open(log_file, 'r', encoding='utf-8', errors='ignore') as f:
                        lines = f.readlines()
                    
                    for i, line in enumerate(lines):
                        line_lower = line.lower()
                        if any(word in line_lower for word in ['error', 'fail', 'exception', 'timeout']):
                            analysis['errors'].append({
                                "file": log_file,
                                "line": i + 1,
                                "content": line.strip()
                            })
                        elif any(word in line_lower for word in ['warning', 'caution']):
                            analysis['warnings'].append({
                                "file": log_file,
                                "line": i + 1,
                                "content": line.strip()
                            })
            
            return JSONResponse({"success": True, "analysis": analysis})
        except Exception as e:
            log(f"Error analyzing logs: {e}")
            return JSONResponse({"success": False, "error": str(e)}, status_code=500)

