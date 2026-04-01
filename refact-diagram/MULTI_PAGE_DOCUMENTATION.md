# Refact Interactive Block Diagram - Multi-Page Documentation

This React application provides an interactive, document-style visualization of the Refact Agentic Workflow for C2000 toolchain. Users can explore detailed information about each component by clicking on interactive blocks.

## Features

### 🎯 Interactive Navigation
- **Clickable Blocks**: Main workflow components are clickable and lead to detailed pages
- **Visual Indicators**: External link icons show which blocks are interactive
- **Smooth Animations**: Framer Motion animations enhance user experience
- **Responsive Design**: Works on desktop and mobile devices

### 📚 Detailed Documentation Pages

#### 1. Natural Language Interface
- **Route**: `/natural-language-interface`
- **Content**: Command examples, key features, processing flow
- **Highlights**: Context-aware interpretation, multi-step workflows

#### 2. AI Intent Parsing
- **Route**: `/ai-intent-parsing`
- **Content**: Core capabilities, processing steps, example analysis
- **Highlights**: Intent classification, tool selection, dependency graphs

#### 3. Dynamic Configuration System
- **Route**: `/dynamic-configuration-system`
- **Content**: HTTP API config, fallback mechanisms, tool execution
- **Highlights**: Real-time updates, error handling, configuration flow

#### 4. Native Rust Tools
- **Route**: `/native-rust-tools`
- **Content**: 8 categorized tools with detailed descriptions
- **Highlights**: Performance, reliability, integration capabilities
- **Individual Tool Pages**:
  - **c2000_project_create** (`/tools/c2000-project-create`): Project creation with templates and configurations
  - **c2000_build** (`/tools/c2000-build`): Multi-configuration builds with optimization
  - **c2000_flash** (`/tools/c2000-flash`): Firmware programming with verification
  - **c2000_uart_capture** (`/tools/c2000-uart-capture`): UART monitoring and analysis
  - **c2000_target_detect** (`/tools/c2000-target-detect`): Hardware detection and validation
  - **c2000_example_list** (`/tools/c2000-example-list`): C2000Ware example search and cataloging
  - **c2000_config_validate** (`/tools/c2000-config-validate`): Configuration validation and error checking
  - **c2000_code_evaluator** (`/tools/c2000-code-evaluator`): AI-powered code analysis and suggestions

#### 5. Complete Workflow Example
- **Route**: `/complete-workflow-example`
- **Content**: Step-by-step workflow with detailed explanations
- **Highlights**: Performance metrics, key benefits, execution details

#### 6. Prompt Journey Visualization
- **Route**: `/prompt-journey`
- **Content**: Interactive step-by-step journey of a natural language prompt
- **Highlights**: Real-time animation, interactive controls, detailed processing steps
- **Features**:
  - **11-step journey** from input to final report
  - **Play/Pause controls** with speed adjustment (0.5x to 5x)
  - **Real-time progress tracking** with visual indicators
  - **Detailed processing information** for each step
  - **Interactive step navigation** (click to jump to any step)
  - **Completion summary** with performance metrics

### 🧭 Navigation Features
- **Breadcrumb Navigation**: Shows current page location
- **Back Button**: Quick return to main diagram
- **Home Button**: Direct navigation to overview
- **Sticky Navigation**: Always accessible navigation bar

## Technical Implementation

### Dependencies
- **React Router DOM**: Multi-page navigation
- **Framer Motion**: Smooth animations and transitions
- **Lucide React**: Consistent iconography
- **Tailwind CSS**: Responsive styling

### Architecture
```
src/
├── components/
│   └── Navigation.jsx          # Top navigation bar
├── pages/
│   ├── NaturalLanguageInterface.jsx
│   ├── AIIntentParsing.jsx
│   ├── DynamicConfigurationSystem.jsx
│   ├── NativeRustTools.jsx
│   ├── CompleteWorkflowExample.jsx
│   ├── PromptJourney.jsx
│   └── tools/
│       ├── C2000ProjectCreate.jsx
│       ├── C2000Build.jsx
│       ├── C2000Flash.jsx
│       ├── C2000UartCapture.jsx
│       ├── C2000TargetDetect.jsx
│       ├── C2000ExampleList.jsx
│       ├── C2000ConfigValidate.jsx
│       └── C2000CodeEvaluator.jsx
├── C2000FullAnimatedDiagram.jsx # Main diagram component
└── App.js                      # Router configuration
```

### Key Features
- **Clickable Blocks**: Blocks with `clickable: true` and `route` properties
- **Visual Feedback**: Hover effects and external link icons
- **Consistent Styling**: Unified design system across all pages
- **Responsive Layout**: Mobile-friendly design patterns

## Usage

1. **Start the application**:
   ```bash
   npm start
   ```

2. **Navigate the diagram**:
   - View the animated overview on the home page
   - Click on any block with an external link icon
   - Use navigation controls to move between pages

3. **Explore detailed content**:
   - Each page provides comprehensive information
   - Interactive elements enhance understanding
   - Consistent visual design maintains context

4. **Watch the Prompt Journey**:
   - Click "Watch Prompt Journey" button on the main page
   - Experience the complete workflow in real-time
   - Use controls to play, pause, adjust speed, or jump to specific steps

## Customization

### Adding New Pages
1. Create a new component in `src/pages/`
2. Add route to `App.js`
3. Update `BLOCKS` array in `C2000FullAnimatedDiagram.jsx`
4. Add `clickable: true` and `route` properties

### Styling
- Uses Tailwind CSS for consistent styling
- Color scheme: Cyan/blue primary, neutral backgrounds
- Responsive breakpoints: `md:`, `lg:` prefixes
- Animation classes: Framer Motion integration

## Future Enhancements

- **Search Functionality**: Find specific tools or concepts
- **Interactive Diagrams**: Clickable sub-components
- **Code Examples**: Live code snippets and demos
- **User Preferences**: Customizable themes and layouts
- **Export Features**: PDF generation, sharing capabilities
