import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Play, Pause, RotateCcw, ChevronRight, Clock, CheckCircle, AlertTriangle, Zap } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

const journeySteps = [
  {
    id: 1,
    title: "Natural Language Input",
    description: "User enters natural language command",
    input: '"Create SPI loopback project for F28P65x"',
    processing: "Capturing user intent...",
    output: "Intent captured: Create SPI project for F28P65x",
    duration: 2000,
    icon: "💬",
    color: "blue"
  },
  {
    id: 2,
    title: "AI Intent Parsing",
    description: "AI analyzes and structures the request",
    input: "Intent: Create SPI project for F28P65x",
    processing: "Analyzing intent... Identifying tools... Generating workflow...",
    output: "Required tools: [c2000_example_list, c2000_project_create, c2000_build, c2000_flash, c2000_uart_capture, c2000_code_evaluator]",
    duration: 3000,
    icon: "🧠",
    color: "purple"
  },
  {
    id: 3,
    title: "Dynamic Configuration",
    description: "System loads and validates configuration",
    input: "Tool list and target requirements",
    processing: "Loading config from API... Validating settings... Broadcasting updates...",
    output: "Configuration loaded: F28P65x target, SPI template, CPU1_LAUNCHXL_RAM config",
    duration: 1500,
    icon: "⚙️",
    color: "green"
  },
  {
    id: 4,
    title: "Tool Discovery",
    description: "c2000_example_list searches for SPI examples",
    input: "Search criteria: SPI loopback examples for F28P65x",
    processing: "Scanning C2000Ware... Filtering examples... Extracting metadata...",
    output: "Found: spi_loopback_f28p65x at /opt/ti/c2000ware/examples/c28x/spi/spi_loopback_f28p65x",
    duration: 2500,
    icon: "🔍",
    color: "cyan"
  },
  {
    id: 5,
    title: "Project Creation",
    description: "c2000_project_create generates new project",
    input: "Template: spi_loopback_f28p65x, Target: F28P65x",
    processing: "Creating project structure... Configuring build settings... Setting up workspace...",
    output: "Project created: /workspace/my_spi_project with CPU1_LAUNCHXL_RAM configuration",
    duration: 3000,
    icon: "📁",
    color: "orange"
  },
  {
    id: 6,
    title: "Build Process",
    description: "c2000_build compiles the project",
    input: "Project: /workspace/my_spi_project, Config: CPU1_LAUNCHXL_RAM",
    processing: "Compiling source files... Linking libraries... Generating binary...",
    output: "Build successful: project.out (45KB), project.map generated",
    duration: 4000,
    icon: "🔨",
    color: "yellow"
  },
  {
    id: 7,
    title: "Hardware Detection",
    description: "c2000_target_detect finds connected hardware",
    input: "Target type: F28P65x",
    processing: "Scanning USB ports... Detecting debug probes... Validating connection...",
    output: "Hardware found: F28P65x LaunchPad via XDS110 debug probe",
    duration: 2000,
    icon: "🔌",
    color: "indigo"
  },
  {
    id: 8,
    title: "Firmware Programming",
    description: "c2000_flash programs the binary",
    input: "Binary: project.out, Target: F28P65x LaunchPad",
    processing: "Erasing flash... Programming binary... Verifying data... Resetting device...",
    output: "Programming successful: Firmware loaded and verified",
    duration: 3500,
    icon: "⚡",
    color: "red"
  },
  {
    id: 9,
    title: "UART Monitoring",
    description: "c2000_uart_capture monitors communication",
    input: "Port: /dev/ttyUSB0, Duration: 30 seconds",
    processing: "Configuring UART... Capturing data... Analyzing patterns...",
    output: "Capture complete: SPI loopback test PASSED, 156 successful transactions",
    duration: 30000,
    icon: "📡",
    color: "teal"
  },
  {
    id: 10,
    title: "AI Analysis",
    description: "c2000_code_evaluator analyzes results",
    input: "UART log: spi_test.log",
    processing: "Analyzing communication patterns... Detecting success indicators... Generating report...",
    output: "Analysis complete: SPI loopback test successful, no errors detected, performance optimal",
    duration: 3000,
    icon: "🤖",
    color: "pink"
  },
  {
    id: 11,
    title: "Final Report",
    description: "System generates comprehensive report",
    input: "All execution data and analysis results",
    processing: "Compiling results... Generating report... Uploading to dashboard...",
    output: "✅ Project created successfully\n✅ Built with CPU1_LAUNCHXL_RAM configuration\n✅ Flashed to F28P65x LaunchPad\n✅ UART output captured and analyzed\n✅ SPI loopback test completed",
    duration: 2000,
    icon: "📊",
    color: "emerald"
  }
];

export default function PromptJourney() {
  const [currentStep, setCurrentStep] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [showDetails, setShowDetails] = useState(false);

  useEffect(() => {
    let interval;
    if (isPlaying && currentStep < journeySteps.length) {
      interval = setInterval(() => {
        setCurrentStep(prev => prev + 1);
      }, journeySteps[currentStep]?.duration / speed || 2000);
    }
    return () => clearInterval(interval);
  }, [isPlaying, currentStep, speed]);

  const handlePlay = () => {
    if (currentStep >= journeySteps.length) {
      setCurrentStep(0);
    }
    setIsPlaying(!isPlaying);
  };

  const handleReset = () => {
    setCurrentStep(0);
    setIsPlaying(false);
  };

  const handleStepClick = (stepIndex) => {
    setCurrentStep(stepIndex);
    setIsPlaying(false);
  };

  const getStepStatus = (stepIndex) => {
    if (stepIndex < currentStep) return "completed";
    if (stepIndex === currentStep) return "active";
    return "pending";
  };

  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-7xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Zap className="inline w-8 h-8 mr-3" />
            Prompt Journey: "Create SPI loopback project for F28P65x"
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            Follow the complete journey of a natural language prompt as it flows through 
            all system components, from initial input to final report generation.
          </p>

          {/* Controls */}
          <div className="flex items-center justify-between mb-8 p-4 bg-neutral-900/50 rounded-xl">
            <div className="flex items-center space-x-4">
              <button
                onClick={handlePlay}
                className="flex items-center space-x-2 px-4 py-2 bg-cyan-500 hover:bg-cyan-600 rounded-lg transition-colors"
              >
                {isPlaying ? <Pause className="w-5 h-5" /> : <Play className="w-5 h-5" />}
                <span>{isPlaying ? 'Pause' : 'Play'}</span>
              </button>
              
              <button
                onClick={handleReset}
                className="flex items-center space-x-2 px-4 py-2 bg-neutral-600 hover:bg-neutral-700 rounded-lg transition-colors"
              >
                <RotateCcw className="w-5 h-5" />
                <span>Reset</span>
              </button>
            </div>

            <div className="flex items-center space-x-4">
              <div className="flex items-center space-x-2">
                <Clock className="w-5 h-5 text-neutral-400" />
                <span className="text-sm text-neutral-300">Speed:</span>
                <select
                  value={speed}
                  onChange={(e) => setSpeed(Number(e.target.value))}
                  className="bg-neutral-800 border border-neutral-600 rounded px-2 py-1 text-sm"
                >
                  <option value={0.5}>0.5x</option>
                  <option value={1}>1x</option>
                  <option value={2}>2x</option>
                  <option value={5}>5x</option>
                </select>
              </div>
              
              <button
                onClick={() => setShowDetails(!showDetails)}
                className="px-4 py-2 bg-purple-500 hover:bg-purple-600 rounded-lg transition-colors text-sm"
              >
                {showDetails ? 'Hide Details' : 'Show Details'}
              </button>
            </div>
          </div>

          {/* Progress Bar */}
          <div className="mb-8">
            <div className="flex justify-between text-sm text-neutral-400 mb-2">
              <span>Progress: {currentStep} / {journeySteps.length}</span>
              <span>{Math.round((currentStep / journeySteps.length) * 100)}%</span>
            </div>
            <div className="w-full bg-neutral-700 rounded-full h-2">
              <motion.div
                className="bg-gradient-to-r from-cyan-500 to-purple-500 h-2 rounded-full"
                initial={{ width: 0 }}
                animate={{ width: `${(currentStep / journeySteps.length) * 100}%` }}
                transition={{ duration: 0.3 }}
              />
            </div>
          </div>

          {/* Journey Steps */}
          <div className="space-y-4">
            {journeySteps.map((step, index) => {
              const status = getStepStatus(index);
              const isActive = status === "active";
              const isCompleted = status === "completed";
              
              return (
                <motion.div
                  key={step.id}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.1 }}
                  className={`p-6 rounded-xl border-2 transition-all duration-500 cursor-pointer ${
                    isActive 
                      ? `border-${step.color}-400 bg-${step.color}-500/10 shadow-${step.color}-400/20` 
                      : isCompleted 
                        ? `border-green-400 bg-green-500/10` 
                        : 'border-neutral-700 bg-neutral-800/50'
                  }`}
                  onClick={() => handleStepClick(index)}
                >
                  <div className="flex items-start space-x-4">
                    <div className={`w-12 h-12 rounded-full flex items-center justify-center text-2xl ${
                      isActive 
                        ? `bg-${step.color}-500/20` 
                        : isCompleted 
                          ? 'bg-green-500/20' 
                          : 'bg-neutral-700'
                    }`}>
                      {isCompleted ? '✅' : step.icon}
                    </div>
                    
                    <div className="flex-1">
                      <div className="flex items-center justify-between mb-2">
                        <h3 className={`text-xl font-semibold ${
                          isActive 
                            ? `text-${step.color}-300` 
                            : isCompleted 
                              ? 'text-green-300' 
                              : 'text-neutral-300'
                        }`}>
                          Step {step.id}: {step.title}
                        </h3>
                        
                        {isActive && (
                          <motion.div
                            animate={{ rotate: 360 }}
                            transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
                            className="w-6 h-6"
                          >
                            <Zap className="w-6 h-6 text-cyan-400" />
                          </motion.div>
                        )}
                      </div>
                      
                      <p className="text-neutral-300 mb-3">{step.description}</p>
                      
                      <div className="grid md:grid-cols-3 gap-4 text-sm">
                        <div className="bg-neutral-900/50 rounded-lg p-3">
                          <h4 className="text-cyan-300 font-medium mb-1">Input:</h4>
                          <code className="text-neutral-300 text-xs">{step.input}</code>
                        </div>
                        
                        {showDetails && (
                          <div className="bg-neutral-900/50 rounded-lg p-3">
                            <h4 className="text-orange-300 font-medium mb-1">Processing:</h4>
                            <span className="text-neutral-300 text-xs">{step.processing}</span>
                          </div>
                        )}
                        
                        <div className="bg-neutral-900/50 rounded-lg p-3">
                          <h4 className="text-green-300 font-medium mb-1">Output:</h4>
                          <code className="text-neutral-300 text-xs whitespace-pre-line">{step.output}</code>
                        </div>
                      </div>
                      
                      {isActive && (
                        <motion.div
                          initial={{ opacity: 0 }}
                          animate={{ opacity: 1 }}
                          className="mt-3 flex items-center space-x-2 text-sm text-cyan-300"
                        >
                          <div className="w-2 h-2 bg-cyan-400 rounded-full animate-pulse"></div>
                          <span>Processing...</span>
                        </motion.div>
                      )}
                    </div>
                  </div>
                </motion.div>
              );
            })}
          </div>

          {/* Summary */}
          {currentStep >= journeySteps.length && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="mt-8 p-6 bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl"
            >
              <h3 className="text-xl font-semibold text-green-300 mb-4 flex items-center">
                <CheckCircle className="w-6 h-6 mr-2" />
                Journey Complete!
              </h3>
              <p className="text-neutral-300 mb-4">
                The natural language prompt has successfully flowed through all system components, 
                demonstrating the complete Refact Agentic Workflow for C2000 toolchain.
              </p>
              <div className="grid md:grid-cols-3 gap-4 text-sm">
                <div className="text-center">
                  <div className="text-2xl font-bold text-green-400">11</div>
                  <div className="text-neutral-300">Steps Executed</div>
                </div>
                <div className="text-center">
                  <div className="text-2xl font-bold text-blue-400">~60s</div>
                  <div className="text-neutral-300">Total Duration</div>
                </div>
                <div className="text-center">
                  <div className="text-2xl font-bold text-purple-400">100%</div>
                  <div className="text-neutral-300">Success Rate</div>
                </div>
              </div>
            </motion.div>
          )}
        </div>
      </motion.div>
    </div>
  );
}
