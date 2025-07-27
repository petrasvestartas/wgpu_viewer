# 🔧 wgpu_viewer Refactoring Plan

## Overview

This document outlines the plan for splitting the monolithic `src/lib.rs` file (~2000 lines) into smaller, maintainable modules. The current architecture has multiple responsibilities mixed together, making it difficult to maintain and extend.

## Current Issues

- **Single Responsibility Violation**: One file handles rendering, geometry loading, input, hot reload, and state management
- **Large State Struct**: Contains rendering pipelines, geometry models, camera, and file loading logic
- **Mixed Concerns**: Hot reload functionality intertwined with core rendering
- **Testing Difficulty**: Monolithic structure makes unit testing challenging
- **Maintenance Burden**: Changes require understanding the entire 2000-line file

## Proposed Module Structure

### 1. 🏗️ **Core State Management** (`src/state.rs`)
**Responsibility**: Foundational state and window management
- `State` struct definition and basic lifecycle methods
- `new()` - State initialization and GPU setup
- `window()` - Window reference access
- `resize()` - Window resize handling
- Essential GPU resource management

**Risk Level**: 🟢 **Low** - Clear boundaries, foundational methods

### 2. 🎨 **Rendering Engine** (`src/render.rs`)
**Responsibility**: All rendering operations and pipeline management
- Main `render()` method and render pass orchestration
- Pipeline creation and management for different geometry types
- Draw call coordination for meshes, points, lines, pipes, polygons
- Depth buffer and surface management
- Render mode switching logic

**Risk Level**: 🟡 **Medium** - Complex rendering logic, careful dependency management needed

### 3. 📐 **Geometry Management** (`src/geometry_manager.rs`)
**Responsibility**: Geometry loading, creation, and model management
- `load_geometries_from_file()` - JSON geometry loading
- `create_sample_polygon()` - Procedural polygon generation
- `create_pipes_from_lines()` - Line-to-pipe conversion
- Model management for different geometry types
- OpenModel integration and mesh conversion

**Risk Level**: 🟡 **Low-Medium** - Well-defined geometry operations with clear data boundaries

### 4. ⌨️ **Input Handling** (`src/input.rs`)
**Responsibility**: User input processing and camera control
- `input()` method and event processing
- Camera controller integration
- Keyboard and mouse event handling
- Render mode switching via input
- Window event processing

**Risk Level**: 🟢 **Low** - Clear input/output boundaries, well-defined interface

### 5. 🔄 **Hot Reload System** (`src/hot_reload.rs`)
**Responsibility**: File watching and dynamic geometry reloading
- WASM hot reload functionality (`reload_geometry`, `fetch_and_reload_geometry`)
- Native file watching with `notify` crate
- `check_and_reload_geometry()` - File change detection
- Platform-specific reload implementations
- JSON parsing and state updates

**Risk Level**: 🟢 **Low** - Already somewhat isolated, clear platform boundaries

### 6. 🚀 **Application Runner** (`src/app.rs`)
**Responsibility**: High-level application orchestration
- `run()` function and main event loop
- Window creation and configuration
- Event loop management and dispatch
- Application lifecycle coordination
- Integration of all subsystems

**Risk Level**: 🟢 **Low** - High-level orchestration, minimal coupling with internals

## Refactoring Implementation Plan

### Phase-by-Phase Approach (Safest Strategy)

#### **Phase 1: Hot Reload Extraction** 🔄
- **Target**: `src/hot_reload.rs`
- **Rationale**: Least coupled system, easiest to isolate
- **Scope**: All WASM and native hot reload functionality
- **Dependencies**: Minimal - mostly self-contained

#### **Phase 2: Input Handling Extraction** ⌨️
- **Target**: `src/input.rs`
- **Rationale**: Well-defined interface with State
- **Scope**: Event processing and camera control
- **Dependencies**: Camera module (already exists)

#### **Phase 3: Geometry Management Extraction** 📐
- **Target**: `src/geometry_manager.rs`
- **Rationale**: Self-contained geometry operations
- **Scope**: All geometry loading and creation methods
- **Dependencies**: Existing model modules, OpenModel integration

#### **Phase 4: Application Runner Extraction** 🚀
- **Target**: `src/app.rs`
- **Rationale**: High-level orchestration with clear boundaries
- **Scope**: Main event loop and window management
- **Dependencies**: State and other extracted modules

#### **Phase 5: Rendering Engine Extraction** 🎨
- **Target**: `src/render.rs`
- **Rationale**: Most complex but State will be smaller by then
- **Scope**: All rendering logic and pipeline management
- **Dependencies**: All geometry and pipeline modules

#### **Phase 6: State Finalization** 🏗️
- **Target**: `src/state.rs`
- **Rationale**: Clean up remaining State struct
- **Scope**: Core state management and initialization
- **Dependencies**: All other modules

## Safety Measures & Best Practices

### 🛡️ **Security & Stability**
- ✅ **Incremental Extraction**: One module at a time with compilation checks
- ✅ **Preserve Public API**: Maintain existing public interface
- ✅ **Visibility Control**: Use `pub(crate)` for internal module communication
- ✅ **Import Preservation**: Keep existing module imports intact initially
- ✅ **Regression Testing**: Test after each extraction phase

### 📋 **Implementation Checklist**
For each phase:
- [ ] Identify code boundaries and dependencies
- [ ] Create new module file
- [ ] Extract relevant code with proper visibility
- [ ] Update imports and module declarations
- [ ] Compile and fix any issues
- [ ] Test functionality
- [ ] Update documentation
- [ ] Commit changes

### 🔍 **Validation Steps**
- **Compilation**: Ensure code compiles without warnings
- **Functionality**: Test all geometry types render correctly
- **Hot Reload**: Verify file watching and reload works
- **Input**: Test camera controls and render mode switching
- **Cross-Platform**: Test on both native and WASM targets

## Expected Benefits

### 🎯 **Maintainability**
- **Single Responsibility**: Each module has one clear purpose
- **Smaller Files**: Easier to understand and navigate
- **Clear Boundaries**: Reduced cognitive load when making changes

### 🔒 **Security & Encapsulation**
- **Better Visibility Control**: Internal implementation details hidden
- **Reduced Coupling**: Modules communicate through well-defined interfaces
- **Easier Security Audits**: Smaller, focused code units

### 🧪 **Testability**
- **Unit Testing**: Individual modules can be tested in isolation
- **Mock Dependencies**: Easier to create test doubles
- **Focused Tests**: Test specific functionality without setup overhead

### 📚 **Developer Experience**
- **Faster Navigation**: Jump to relevant code quickly
- **Reduced Merge Conflicts**: Multiple developers can work on different modules
- **Clearer Git History**: Changes are scoped to relevant modules

### 🔄 **Extensibility**
- **Plugin Architecture**: New geometry types easier to add
- **Modular Features**: Features can be enabled/disabled per module
- **Reusable Components**: Modules can be reused in other projects

## Migration Timeline

**Estimated Duration**: 2-3 days
- **Phase 1-2**: 4-6 hours (Hot reload + Input)
- **Phase 3-4**: 6-8 hours (Geometry + App runner)
- **Phase 5-6**: 8-10 hours (Rendering + State cleanup)
- **Testing & Documentation**: 2-4 hours

## Post-Refactoring Structure

```
src/
├── lib.rs              # Main exports and module declarations
├── state.rs            # Core state management
├── render.rs           # Rendering engine
├── geometry_manager.rs # Geometry loading and management
├── input.rs            # Input handling
├── hot_reload.rs       # Hot reload system
├── app.rs              # Application runner
├── camera.rs           # Camera system (existing)
├── instance.rs         # Instance management (existing)
├── model/              # Geometry models (existing)
│   ├── mod.rs
│   ├── line.rs
│   ├── pipe.rs
│   ├── point.rs
│   └── polygon.rs
├── pipeline.rs         # Pipeline utilities (existing)
├── renderer.rs         # Renderer utilities (existing)
└── resources.rs        # Resource management (existing)
```

## Notes

- This refactoring maintains backward compatibility
- All existing functionality will be preserved
- The OpenModel integration remains intact
- WASM and native builds both supported
- Hot reload functionality preserved for both platforms

---

**Status**: 📋 **Planning Phase**  
**Next Step**: Begin Phase 1 (Hot Reload Extraction)  
**Last Updated**: 2025-07-26
