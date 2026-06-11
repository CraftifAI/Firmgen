# Build PIR_agent reference bundle from latest working-tree sources.
$ErrorActionPreference = "Stop"
$Root = "C:\Users\ritik\OneDrive\Desktop\CRAFTIF\updateeesss\IIS_agent-RITIK"
$Src = Join-Path $Root "refact-agent"
$Dest = Join-Path $Root "PIR_agent"

$RelativePaths = @(
    "context.md",
    "refact-agent/docs/PIR_MAKER.md",
    # Engine: PIR core
    "refact-agent/engine/src/pir_maker/mod.rs",
    "refact-agent/engine/src/pir_maker/schema.rs",
    "refact-agent/engine/src/pir_maker/service.rs",
    "refact-agent/engine/src/pir_maker/session.rs",
    "refact-agent/engine/src/pir_maker/sync.rs",
    "refact-agent/engine/src/pir_maker/persistence.rs",
    "refact-agent/engine/src/pir_maker/builder.rs",
    "refact-agent/engine/src/pir_maker/diagrams.rs",
    "refact-agent/engine/src/pir_maker/diagram_view_builders.rs",
    "refact-agent/engine/src/pir_maker/apply_patch.rs",
    "refact-agent/engine/src/pir_maker/board_validate.rs",
    "refact-agent/engine/src/pir_maker/validation_lock.rs",
    "refact-agent/engine/src/pir_maker/agent/mod.rs",
    "refact-agent/engine/src/pir_maker/agent/context.rs",
    "refact-agent/engine/src/pir_maker/agent/merge.rs",
    "refact-agent/engine/src/pir_maker/agent/parser.rs",
    "refact-agent/engine/src/pir_maker/agent/prompt.rs",
    "refact-agent/engine/src/pir_maker/analyzer/mod.rs",
    "refact-agent/engine/src/pir_maker/analyzer/manifest.rs",
    "refact-agent/engine/src/pir_maker/analyzer/static_extract.rs",
    "refact-agent/engine/src/pir_maker/analyzer/ast_extract.rs",
    "refact-agent/engine/src/pir_maker/analyzer/chat_gap.rs",
    # Firmware topology (shared graph layer)
    "refact-agent/engine/src/firmware_topology/mod.rs",
    "refact-agent/engine/src/firmware_topology/types.rs",
    "refact-agent/engine/src/firmware_topology/registry.rs",
    "refact-agent/engine/src/firmware_topology/validator.rs",
    "refact-agent/engine/src/firmware_topology/layout.rs",
    "refact-agent/engine/src/firmware_topology/samples.rs",
    "refact-agent/engine/schemas/firmware_topology.schema.json",
    "refact-agent/engine/schemas/firmware_topology.example.json",
    # HTTP + integration hooks
    "refact-agent/engine/src/http/routers/v1/pir_maker.rs",
    "refact-agent/engine/src/http/routers/v1.rs",
    "refact-agent/engine/src/main.rs",
    "refact-agent/engine/src/files_in_workspace.rs",
    "refact-agent/engine/src/scratchpads/chat_utils_prompts.rs",
    "refact-agent/engine/src/yaml_configs/customization_compiled_in.yaml",
    # GUI: PirAgent feature
    "refact-agent/gui/src/features/PirAgent/index.ts",
    "refact-agent/gui/src/features/PirAgent/pirTypes.ts",
    "refact-agent/gui/src/features/PirAgent/types.ts",
    "refact-agent/gui/src/features/PirAgent/utils/noop.ts",
    "refact-agent/gui/src/features/PirAgent/hooks/usePirMaker.ts",
    "refact-agent/gui/src/features/PirAgent/hooks/usePirChatAnchor.ts",
    "refact-agent/gui/src/features/PirAgent/hooks/usePirCodegenReady.ts",
    "refact-agent/gui/src/features/PirAgent/components/FirmwareNode.tsx",
    "refact-agent/gui/src/features/PirAgent/components/FirmwareNode.module.css",
    "refact-agent/gui/src/features/PirAgent/components/firmwareNodeTypes.ts",
    "refact-agent/gui/src/features/PirAgent/components/GraphCanvas.tsx",
    "refact-agent/gui/src/features/PirAgent/components/GraphCanvas.module.css",
    "refact-agent/gui/src/features/PirAgent/components/GraphDiagramViewSelect.tsx",
    "refact-agent/gui/src/features/PirAgent/components/GraphDiagramViewSelect.module.css",
    "refact-agent/gui/src/features/PirAgent/components/MermaidDiagram.tsx",
    "refact-agent/gui/src/features/PirAgent/components/NodeInspector.tsx",
    "refact-agent/gui/src/features/PirAgent/components/NodeInspector.module.css",
    "refact-agent/gui/src/features/PirAgent/components/PirAnalyzedFilesList.tsx",
    "refact-agent/gui/src/features/PirAgent/components/PirAnalyzedFilesList.module.css",
    "refact-agent/gui/src/features/PirAgent/components/PirTopologyChatBlock.tsx",
    "refact-agent/gui/src/features/PirAgent/components/PirTopologyChatBlock.module.css",
    "refact-agent/gui/src/features/PirAgent/components/PirTopologyEditorOverlay.tsx",
    "refact-agent/gui/src/features/PirAgent/components/TopologyApprovalCard.tsx",
    "refact-agent/gui/src/features/PirAgent/components/TopologyErrorBoundary.tsx",
    "refact-agent/gui/src/features/PirAgent/layout/applyLayout.ts",
    "refact-agent/gui/src/features/PirAgent/layout/diagramLayoutPositions.ts",
    "refact-agent/gui/src/features/PirAgent/layout/diagramLayoutPositions.test.ts",
    "refact-agent/gui/src/features/PirAgent/layout/graphViewTransforms.ts",
    "refact-agent/gui/src/features/PirAgent/layout/graphViewTypes.ts",
    "refact-agent/gui/src/features/PirAgent/layout/hldComponent.ts",
    "refact-agent/gui/src/features/PirAgent/layout/hldComponent.test.ts",
    "refact-agent/gui/src/features/PirAgent/layout/layoutEngine.ts",
    "refact-agent/gui/src/features/PirAgent/layout/layoutEngine.test.ts",
    "refact-agent/gui/src/features/PirAgent/layout/lddUml.ts",
    "refact-agent/gui/src/features/PirAgent/layout/lddUml.test.ts",
    # GUI: integration touchpoints
    "refact-agent/gui/src/services/refact/pirMaker.ts",
    "refact-agent/gui/src/services/refact/index.ts",
    "refact-agent/gui/src/features/App.tsx",
    "refact-agent/gui/src/features/Pages/pagesSlice.ts",
    "refact-agent/gui/src/components/Chat/Chat.tsx",
    "refact-agent/gui/src/components/ChatContent/ChatContent.tsx",
    "refact-agent/gui/src/components/Toolbar/Dropdown.tsx",
    "refact-agent/gui/package.json",
    "refact-agent/gui/.eslintrc.cjs"
)

# Fresh bundle (keep only listed paths)
if (Test-Path $Dest) {
    Remove-Item -LiteralPath $Dest -Recurse -Force
}
New-Item -ItemType Directory -Path $Dest -Force | Out-Null

$copied = 0
$missing = @()
foreach ($rel in $RelativePaths) {
    $from = Join-Path $Root $rel
    $to = Join-Path $Dest $rel
    if (-not (Test-Path -LiteralPath $from)) {
        $missing += $rel
        continue
    }
    $dir = Split-Path $to -Parent
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    Copy-Item -LiteralPath $from -Destination $to -Force
    $copied++
}

Write-Host "Copied $copied files."
if ($missing.Count -gt 0) {
    Write-Host "MISSING:"
    $missing | ForEach-Object { Write-Host "  $_" }
    exit 1
}

# Write manifest for inventory generator
$manifest = Join-Path $Dest "_file_list.txt"
$RelativePaths | Set-Content -Path $manifest -Encoding UTF8
Write-Host "Wrote $manifest"
