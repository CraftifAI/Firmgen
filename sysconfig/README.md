Agent-oriented SysConfig Recipes — LAUNCHXL_F28P65X

Use case
- LLM agents need deterministic, idempotent ways to modify .syscfg for LaunchPad-F28P65x projects
- Recipes are machine-readable and include detection/fix rules

Files
- launchxl_f28p65x_syscfg_recipes.json: core dataset for imports, config, pins, conflicts, references

Contract for agents
1) Operate on workspace copy
- Always edit the SysConfig file inside the CCS workspace (e.g., new_workspace/<project>/<project>.syscfg)
- Do not edit the original example’s .syscfg after project creation

2) Set board token
- Ensure @cliArgs board is /boards/LAUNCHXL_F28P65X (underscore, caps)
- If build error shows a missing board JSON, fix the token

3) Idempotent insertion
- Before inserting:
  • Check for existing module import: detect_module_decl regex
  • Check for existing instance: detect_instance_decl regex
- If duplicates exist, remove subsequent duplicates (keep the first)

4) Conflicts
- SCI_SCIA and USB0 conflict on GPIO42/43
- If both are present, move SCI to GPIO28/29 or disable USB

5) Validation
- After modifying .syscfg, run SysConfig CLI (CCS build) and parse errors/warnings
- If syntax errors (e.g., malformed properties), rewrite using the recipe’s exact property names ($name, $assign)

6) References
- Each peripheral includes example file paths under driverlib/... for pattern verification

7) Example flow: add SCI to LED project
- Confirm board token
- Insert sci imports and config from SCI_SCIA recipe
- Rebuild; on success, C code can use mySCI0_BASE from board.h

This dataset is meant to be extended. Add new peripherals or device variants by following the same structure.
