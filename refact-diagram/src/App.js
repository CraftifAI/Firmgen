import { BrowserRouter as Router, Routes, Route } from "react-router-dom";
import C2000FullAnimatedDiagram from "./C2000FullAnimatedDiagram";
import Navigation from "./components/Navigation";
import NaturalLanguageInterface from "./pages/NaturalLanguageInterface";
import AIIntentParsing from "./pages/AIIntentParsing";
import DynamicConfigurationSystem from "./pages/DynamicConfigurationSystem";
import NativeRustTools from "./pages/NativeRustTools";
import CompleteWorkflowExample from "./pages/CompleteWorkflowExample";
import PromptJourney from "./pages/PromptJourney";

// Individual tool pages
import C2000ProjectCreate from "./pages/tools/C2000ProjectCreate";
import C2000Build from "./pages/tools/C2000Build";
import C2000Flash from "./pages/tools/C2000Flash";
import C2000UartCapture from "./pages/tools/C2000UartCapture";
import C2000TargetDetect from "./pages/tools/C2000TargetDetect";
import C2000ExampleList from "./pages/tools/C2000ExampleList";
import C2000ConfigValidate from "./pages/tools/C2000ConfigValidate";
import C2000CodeEvaluator from "./pages/tools/C2000CodeEvaluator";

export default function App() {
  return (
    <Router>
      <div className="min-h-screen bg-neutral-950">
        <Navigation />
        <Routes>
          <Route path="/" element={<C2000FullAnimatedDiagram />} />
          <Route path="/natural-language-interface" element={<NaturalLanguageInterface />} />
          <Route path="/ai-intent-parsing" element={<AIIntentParsing />} />
          <Route path="/dynamic-configuration-system" element={<DynamicConfigurationSystem />} />
          <Route path="/native-rust-tools" element={<NativeRustTools />} />
          <Route path="/complete-workflow-example" element={<CompleteWorkflowExample />} />
          <Route path="/prompt-journey" element={<PromptJourney />} />
          
          {/* Individual tool pages */}
          <Route path="/tools/c2000-project-create" element={<C2000ProjectCreate />} />
          <Route path="/tools/c2000-build" element={<C2000Build />} />
          <Route path="/tools/c2000-flash" element={<C2000Flash />} />
          <Route path="/tools/c2000-uart-capture" element={<C2000UartCapture />} />
          <Route path="/tools/c2000-target-detect" element={<C2000TargetDetect />} />
          <Route path="/tools/c2000-example-list" element={<C2000ExampleList />} />
          <Route path="/tools/c2000-config-validate" element={<C2000ConfigValidate />} />
          <Route path="/tools/c2000-code-evaluator" element={<C2000CodeEvaluator />} />
        </Routes>
      </div>
    </Router>
  );
}



