import { motion } from "framer-motion";
import { Play, CheckCircle, Clock, Target, Zap, FileText } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

const workflowSteps = [
  {
    step: 1,
    title: "DISCOVERY",
    tool: "c2000_example_list",
    description: "Searches C2000Ware for SPI examples",
    details: [
      "Scans C2000Ware installation directory",
      "Filters examples by SPI communication type",
      "Returns project specification paths",
      "Validates example compatibility with F28P65x"
    ],
    icon: Target,
    color: "blue"
  },
  {
    step: 2,
    title: "CREATION",
    tool: "c2000_project_create",
    description: "Creates CCS project from template",
    details: [
      "Copies SPI loopback template",
      "Configures project for F28P65x target",
      "Sets up workspace structure",
      "Initializes build configurations"
    ],
    icon: FileText,
    color: "green"
  },
  {
    step: 3,
    title: "BUILD & DEPLOYMENT",
    tool: "c2000_build",
    description: "Compiles project with optimal settings",
    details: [
      "Uses CPU1_LAUNCHXL_RAM configuration",
      "Applies optimization flags",
      "Generates binary output",
      "Validates build success"
    ],
    icon: Zap,
    color: "yellow"
  },
  {
    step: 4,
    title: "HARDWARE VERIFICATION",
    tool: "c2000_target_detect",
    description: "Detects and verifies target hardware",
    details: [
      "Scans for connected F28P65x devices",
      "Verifies debug probe connection",
      "Lists available debug interfaces",
      "Confirms hardware readiness"
    ],
    icon: CheckCircle,
    color: "purple"
  },
  {
    step: 5,
    title: "PROGRAMMING",
    tool: "c2000_flash",
    description: "Programs firmware to target device",
    details: [
      "Loads binary to target memory",
      "Verifies programming success",
      "Resets device to run mode",
      "Confirms firmware execution"
    ],
    icon: Play,
    color: "orange"
  },
  {
    step: 6,
    title: "MONITORING",
    tool: "c2000_uart_capture",
    description: "Captures UART output for analysis",
    details: [
      "Configures UART communication",
      "Captures 30-second data stream",
      "Saves output to log file",
      "Monitors communication patterns"
    ],
    icon: Clock,
    color: "cyan"
  },
  {
    step: 7,
    title: "AI ANALYSIS",
    tool: "c2000_code_evaluator",
    description: "AI-powered analysis of captured data",
    details: [
      "Analyzes UART communication patterns",
      "Uses Refact's internal LLM",
      "Identifies success/failure indicators",
      "Generates improvement suggestions"
    ],
    icon: CheckCircle,
    color: "pink"
  }
];

export default function CompleteWorkflowExample() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-6xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Play className="inline w-8 h-8 mr-3" />
            Complete Workflow Example
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            End-to-end demonstration of the Refact Agent workflow: 
            <code className="bg-neutral-800 px-2 py-1 rounded text-cyan-300 mx-2">
              "Create SPI loopback project for F28P65x"
            </code>
            This example showcases how all components work together seamlessly.
          </p>

          <div className="space-y-6 mb-8">
            {workflowSteps.map((step, index) => (
              <motion.div
                key={step.step}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.1 }}
                className={`bg-neutral-900/80 border border-${step.color}-400 rounded-xl p-6 hover:border-${step.color}-300 transition-colors`}
              >
                <div className="flex items-start space-x-4">
                  <div className={`w-12 h-12 bg-${step.color}-500/20 rounded-full flex items-center justify-center flex-shrink-0`}>
                    <step.icon className={`w-6 h-6 text-${step.color}-400`} />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center space-x-3 mb-2">
                      <span className={`w-8 h-8 bg-${step.color}-500 rounded-full flex items-center justify-center text-sm font-bold text-black`}>
                        {step.step}
                      </span>
                      <h3 className={`text-xl font-semibold text-${step.color}-300`}>
                        STEP {step.step}: {step.title}
                      </h3>
                    </div>
                    <div className="mb-3">
                      <code className={`bg-neutral-800 px-3 py-1 rounded text-${step.color}-300 font-medium`}>
                        {step.tool}
                      </code>
                    </div>
                    <p className="text-neutral-300 mb-3">{step.description}</p>
                    <ul className="space-y-1">
                      {step.details.map((detail, detailIndex) => (
                        <li key={detailIndex} className="text-sm text-neutral-400 flex items-start">
                          <span className={`text-${step.color}-400 mr-2`}>•</span>
                          {detail}
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>

          <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Final Report</h3>
            <div className="space-y-3">
              {[
                "✅ Project created successfully",
                "✅ Built with CPU1_LAUNCHXL_RAM configuration",
                "✅ Flashed to F28P65x LaunchPad",
                "✅ UART output captured and analyzed",
                "✅ SPI loopback test completed"
              ].map((item, index) => (
                <div key={index} className="flex items-center space-x-3">
                  <CheckCircle className="w-5 h-5 text-green-400" />
                  <span className="text-green-300">{item}</span>
                </div>
              ))}
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Performance Metrics</h3>
              <div className="space-y-2 text-neutral-200">
                <div className="flex justify-between">
                  <span>Total Execution Time:</span>
                  <span className="text-cyan-300">~45 seconds</span>
                </div>
                <div className="flex justify-between">
                  <span>Tools Executed:</span>
                  <span className="text-cyan-300">7 tools</span>
                </div>
                <div className="flex justify-between">
                  <span>Success Rate:</span>
                  <span className="text-green-300">100%</span>
                </div>
                <div className="flex justify-between">
                  <span>Manual Steps:</span>
                  <span className="text-green-300">0</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Key Benefits</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Fully automated workflow execution
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Real-time progress monitoring
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Comprehensive error handling
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  AI-powered result analysis
                </li>
              </ul>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
