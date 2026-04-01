# Refact Diagram - Interactive Documentation

## 🚀 Quick Start with Docker

### Prerequisites
- Docker installed on your system
- No other dependencies required!

### Running the Application

#### Option 1: Direct Docker Command
```bash
docker run -p 3000:80 refact-diagram
```

#### Option 2: Using Docker Compose
```bash
docker-compose up -d
```

#### Option 3: Build and Run
```bash
# Build the image
docker build -t refact-diagram .

# Run the container
docker run -p 3000:80 refact-diagram
```

### Access the Application
Open your browser and go to: **http://localhost:3000**

## 🎯 Features

### Interactive Diagram
- **Clickable Blocks**: Click on any block with an external link icon to see detailed information
- **Smooth Animations**: Framer Motion animations enhance the experience
- **Responsive Design**: Works on desktop and mobile devices

### Detailed Documentation Pages
- **Natural Language Interface**: Command examples and processing flow
- **AI Intent Parsing**: Core capabilities and processing steps
- **Dynamic Configuration System**: HTTP API config and fallback mechanisms
- **Native Rust Tools**: 8 categorized tools with individual detailed pages
- **Complete Workflow Example**: Step-by-step workflow with performance metrics

### Prompt Journey Visualization
- **Interactive Animation**: Watch a natural language prompt flow through all system components
- **Play/Pause Controls**: Control the animation with speed adjustment (0.5x to 5x)
- **Step Navigation**: Click any step to jump directly to it
- **Real-time Progress**: Visual progress tracking with completion summary

### Individual Tool Pages
Each of the 8 Rust tools has its own detailed page with:
- **Command Examples**: Real command-line usage examples
- **Configuration Details**: Specific options and settings
- **Error Handling**: Comprehensive error scenarios
- **Performance Metrics**: Timing and optimization information

## 📦 Sharing Options

### Share Docker Image
```bash
# Save image to file
docker save refact-diagram > refact-diagram.tar

# Share the .tar file
# Recipients load it with:
docker load < refact-diagram.tar
docker run -p 3000:80 refact-diagram
```

### Share Source Code
```bash
# Create source package
tar -czf refact-diagram-source.tar.gz \
  --exclude=node_modules \
  --exclude=.git \
  --exclude=build \
  .

# Recipients build and run:
# tar -xzf refact-diagram-source.tar.gz
# docker build -t refact-diagram .
# docker run -p 3000:80 refact-diagram
```

## 🔧 Troubleshooting

### Port Already in Use
If port 3000 is busy, use a different port:
```bash
docker run -p 8080:80 refact-diagram
# Then access: http://localhost:8080
```

### Container Won't Start
Check if Docker is running:
```bash
docker --version
docker ps
```

### Build Issues
Make sure you're in the project directory:
```bash
cd /path/to/refact-diagram
docker build -t refact-diagram .
```

## 🎨 What You'll See

1. **Main Diagram**: Animated overview of the Refact Agentic Workflow
2. **"Watch Prompt Journey" Button**: Prominent button to start the interactive journey
3. **Clickable Blocks**: Blocks with external link icons lead to detailed pages
4. **Navigation Bar**: Easy navigation between pages with breadcrumbs
5. **Responsive Design**: Works perfectly on any screen size

## 🏢 Organization Benefits

- **Private Sharing**: Keep within your organization
- **Zero Dependencies**: Recipients only need Docker
- **Consistent Environment**: Works the same everywhere
- **Easy Updates**: Share new versions by updating the Docker image
- **Professional Presentation**: High-quality interactive documentation

## 📞 Support

If you encounter any issues:
1. Ensure Docker is properly installed
2. Check that port 3000 (or your chosen port) is available
3. Verify the Docker image was built successfully
4. Check Docker logs: `docker logs refact-diagram`

---

**Enjoy exploring the Refact Agentic Workflow for C2000 toolchain!** 🚀


