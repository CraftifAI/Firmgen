import { motion } from "framer-motion";
import { Wrench, Play, Upload, Monitor, Search, List, CheckCircle, Brain, ExternalLink } from "lucide-react";
import { useNavigate } from "react-router-dom";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

const tools = [
  {
    category: "Core Workflow Tools",
    icon: Wrench,
    color: "cyan",
    tools: [
      {
        name: "c2000_project_create",
        description: "Creates new CCS projects from templates",
        features: ["Template selection", "Project configuration", "Workspace setup", "Dependency management"],
        route: "/tools/c2000-project-create"
      },
      {
        name: "c2000_build",
        description: "Compiles projects with optimal settings",
        features: ["Multi-configuration builds", "Error reporting", "Optimization flags", "Cross-compilation"],
        route: "/tools/c2000-build"
      },
      {
        name: "c2000_flash",
        description: "Programs firmware to target hardware",
        features: ["Hardware detection", "Firmware verification", "Reset control", "Error recovery"],
        route: "/tools/c2000-flash"
      },
      {
        name: "c2000_uart_capture",
        description: "Captures and analyzes UART communication",
        features: ["Real-time monitoring", "Data logging", "Pattern analysis", "Export capabilities"],
        route: "/tools/c2000-uart-capture"
      }
    ]
  },
  {
    category: "Diagnostic & Support Tools",
    icon: Search,
    color: "green",
    tools: [
      {
        name: "c2000_target_detect",
        description: "Detects and identifies connected hardware",
        features: ["Hardware enumeration", "Connection validation", "Debug probe detection", "Status reporting"],
        route: "/tools/c2000-target-detect"
      },
      {
        name: "c2000_example_list",
        description: "Searches and lists available examples",
        features: ["C2000Ware integration", "Filtering capabilities", "Metadata extraction", "Path resolution"],
        route: "/tools/c2000-example-list"
      },
      {
        name: "c2000_config_validate",
        description: "Validates project configurations",
        features: ["Syntax checking", "Dependency validation", "Compatibility checks", "Error reporting"],
        route: "/tools/c2000-config-validate"
      }
    ]
  },
  {
    category: "AI-Powered Analysis Tool",
    icon: Brain,
    color: "purple",
    tools: [
      {
        name: "c2000_code_evaluator",
        description: "AI-powered code and output analysis",
        features: ["Semantic analysis", "Code comparison", "Pattern recognition", "Improvement suggestions"],
        route: "/tools/c2000-code-evaluator"
      }
    ]
  }
];

export default function NativeRustTools() {
  const navigate = useNavigate();

  const handleToolClick = (tool) => {
    if (tool.route) {
      navigate(tool.route);
    }
  };
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-6xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Wrench className="inline w-8 h-8 mr-3" />
            8 Native Rust Tools
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            High-performance Rust-based utilities that provide direct hardware interaction, 
            project management, and AI-powered analysis capabilities for the C2000 ecosystem.
          </p>

          {tools.map((category, categoryIndex) => (
            <div key={category.category} className="mb-8">
              <h2 className={`text-xl font-semibold text-${category.color}-300 mb-4 flex items-center`}>
                <category.icon className="w-6 h-6 mr-2" />
                {category.category}
              </h2>
              
              <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-4">
                {category.tools.map((tool, toolIndex) => (
                  <div
                    key={tool.name}
                    className={`bg-neutral-900/80 border border-${category.color}-400 rounded-xl p-4 hover:border-${category.color}-300 transition-colors cursor-pointer`}
                    onClick={() => handleToolClick(tool)}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <h3 className={`text-${category.color}-300 font-medium`}>
                        {tool.name}
                      </h3>
                      <ExternalLink className={`w-4 h-4 text-${category.color}-400 opacity-60`} />
                    </div>
                    <p className="text-neutral-300 text-sm mb-3">
                      {tool.description}
                    </p>
                    <ul className="space-y-1">
                      {tool.features.map((feature, featureIndex) => (
                        <li key={featureIndex} className="text-xs text-neutral-400 flex items-start">
                          <span className={`text-${category.color}-400 mr-1`}>•</span>
                          {feature}
                        </li>
                      ))}
                    </ul>
                  </div>
                ))}
              </div>
            </div>
          ))}

          <div className="bg-gradient-to-r from-cyan-500/10 to-purple-500/10 border border-cyan-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Tool Architecture</h3>
            <div className="grid md:grid-cols-3 gap-6">
              <div className="text-center">
                <div className="w-16 h-16 bg-cyan-500/20 rounded-full flex items-center justify-center mx-auto mb-3">
                  <Play className="w-8 h-8 text-cyan-400" />
                </div>
                <h4 className="text-cyan-300 font-medium mb-2">Performance</h4>
                <p className="text-sm text-neutral-300">Rust's zero-cost abstractions ensure optimal performance</p>
              </div>
              <div className="text-center">
                <div className="w-16 h-16 bg-green-500/20 rounded-full flex items-center justify-center mx-auto mb-3">
                  <CheckCircle className="w-8 h-8 text-green-400" />
                </div>
                <h4 className="text-cyan-300 font-medium mb-2">Reliability</h4>
                <p className="text-sm text-neutral-300">Memory safety and error handling prevent crashes</p>
              </div>
              <div className="text-center">
                <div className="w-16 h-16 bg-purple-500/20 rounded-full flex items-center justify-center mx-auto mb-3">
                  <Brain className="w-8 h-8 text-purple-400" />
                </div>
                <h4 className="text-cyan-300 font-medium mb-2">Integration</h4>
                <p className="text-sm text-neutral-300">Seamless integration with AI and hardware systems</p>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6">
            <h3 className={subtitle}>Example Usage</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-orange-300 font-medium mb-2">Complete Workflow</h4>
                <div className="space-y-2 text-sm text-neutral-200">
                  <div className="flex items-center space-x-2">
                    <span className="w-6 h-6 bg-orange-500 rounded-full flex items-center justify-center text-xs font-bold text-black">1</span>
                    <code className="bg-neutral-700 px-2 py-1 rounded">c2000_example_list --type spi</code>
                  </div>
                  <div className="flex items-center space-x-2">
                    <span className="w-6 h-6 bg-orange-500 rounded-full flex items-center justify-center text-xs font-bold text-black">2</span>
                    <code className="bg-neutral-700 px-2 py-1 rounded">c2000_project_create --template spi_loopback</code>
                  </div>
                  <div className="flex items-center space-x-2">
                    <span className="w-6 h-6 bg-orange-500 rounded-full flex items-center justify-center text-xs font-bold text-black">3</span>
                    <code className="bg-neutral-700 px-2 py-1 rounded">c2000_build --config CPU1_LAUNCHXL_RAM</code>
                  </div>
                  <div className="flex items-center space-x-2">
                    <span className="w-6 h-6 bg-orange-500 rounded-full flex items-center justify-center text-xs font-bold text-black">4</span>
                    <code className="bg-neutral-700 px-2 py-1 rounded">c2000_flash --verify</code>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
