# VIB3+ Complete Usage Guide

## What VIB3+ Is

VIB3+ is a **shader-based visualization engine** for web pages. It creates animated backgrounds using WebGL shaders - NOT wireframe 3D geometry.

**Use cases:**
- Landing page backgrounds
- Portfolio site effects
- Music visualizers
- Interactive art installations

---

## The Four Systems

| Key | System | Description | Best For |
|-----|--------|-------------|----------|
| 1 | **Faceted** | 2D geometric patterns with 4D rotation | Clean, modern backgrounds |
| 2 | **Quantum** | 3D lattice with velocity-based effects | Dynamic, flowing visuals |
| 3 | **Holographic** | Audio-reactive shimmer effects | Music visualizers |
| 4 | **Polychora** | 4D polytope projections | Mathematical art |

**Note on Polychora:** It exists in the UI (key 4) but documentation indicates it may be less stable than the other three systems. Use faceted/quantum/holographic for production.

---

## The 24 Geometries

Each system has 24 geometry variants using this formula:

```
geometry = coreIndex * 8 + baseIndex
```

### Base Geometries (0-7)
| Index | Name | Shortcut |
|-------|------|----------|
| 0 | Tetrahedron | Alt+Q |
| 1 | Hypercube | Alt+W |
| 2 | Sphere | Alt+E |
| 3 | Torus | Alt+R |
| 4 | Klein Bottle | Alt+A |
| 5 | Fractal | Alt+S |
| 6 | Wave | Alt+D |
| 7 | Crystal | Alt+F |

### Core Variations (multiply by 8)
| Core | Index | Effect | Shortcut |
|------|-------|--------|----------|
| Base | 0 | Original geometry | Alt+1 |
| Hypersphere | 1 | 4D sphere warp (indices 8-15) | Alt+2 |
| Hypertetrahedron | 2 | 4D tetrahedron warp (indices 16-23) | Alt+3 |

**Examples:**
- Geometry 0 = Base Tetrahedron (0×8 + 0)
- Geometry 3 = Base Torus (0×8 + 3)
- Geometry 11 = Hypersphere Torus (1×8 + 3)
- Geometry 19 = Hypertetrahedron Torus (2×8 + 3)

---

## All Parameters

### 6D Rotation (radians: -6.28 to 6.28)
| Parameter | Planes |
|-----------|--------|
| rot4dXY | X-Y rotation |
| rot4dXZ | X-Z rotation |
| rot4dYZ | Y-Z rotation |
| rot4dXW | X-W hyperspace |
| rot4dYW | Y-W hyperspace |
| rot4dZW | Z-W hyperspace |

### Visual Parameters
| Parameter | Range | Default | Effect |
|-----------|-------|---------|--------|
| gridDensity | 5-100 | 15 | Pattern detail level |
| morphFactor | 0-2 | 1.0 | Shape blending amount |
| chaos | 0-1 | 0.2 | Randomness/organic feel |
| speed | 0.1-3 | 1.0 | Animation speed |

### Color Parameters
| Parameter | Range | Default | Effect |
|-----------|-------|---------|--------|
| hue | 0-360 | 200 | Base color (degrees) |
| saturation | 0-1 | 0.8 | Color intensity |
| intensity | 0-1 | 0.5 | Brightness |

---

## Complete Keyboard Shortcuts

### System & Geometry
| Key | Action |
|-----|--------|
| 1-4 | Switch systems (Faceted, Quantum, Holographic, Polychora) |
| Alt+1-3 | Select core type (Base, Hypersphere, Hypertetra) |
| Alt+Q/W/E/R | Base geometries 0-3 (Tetra, Hypercube, Sphere, Torus) |
| Alt+A/S/D/F | Base geometries 4-7 (Klein, Fractal, Wave, Crystal) |
| ← / → | Navigate geometries (wraps 0↔23) |
| ↑ / ↓ | Cycle systems forward/backward |

### Controls & Actions
| Key | Action |
|-----|--------|
| H | Show help modal with all shortcuts |
| Ctrl+R | Randomize all parameters |
| Ctrl+Shift+R | Randomize everything including system |
| Ctrl+Shift+Z | Reset all to defaults |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |

### State Management
| Key | Action |
|-----|--------|
| Ctrl+S | Save to gallery |
| Ctrl+G | Open gallery |
| Ctrl+L | Copy shareable URL |
| Ctrl+E | Export trading card |

### UI & Display
| Key | Action |
|-----|--------|
| Ctrl+B or Space | Toggle bottom panel collapse |
| Ctrl+1-5 | Switch tabs (Controls, Color, Geometry, Reactivity, Export) |
| P | Show/hide performance stats |
| M | Cycle performance modes (Auto→Low→Medium→High→Ultra) |

### Toggles
| Key | Action |
|-----|--------|
| A | Toggle audio reactivity |
| T | Toggle device tilt |
| I | Toggle interactivity menu |
| F | Toggle fullscreen |

---

## JavaScript API

All functions are on `window.*`:

### Core Functions
```javascript
// Switch system
window.switchSystem('holographic')  // 'faceted', 'quantum', 'holographic', 'polychora'

// Select geometry (0-23)
window.selectGeometry(11)  // Hypersphere Torus

// Update any parameter
window.updateParameter('hue', 280)
window.updateParameter('speed', 1.5)
window.updateParameter('rot4dXW', 3.14)

// Randomize
window.randomizeAll()

// Reset
window.resetAll()
```

### Gallery & Export
```javascript
window.saveToGallery()
window.openGallery()
window.createTradingCard()
```

### Toggles
```javascript
window.toggleAudio()
window.toggleDeviceTilt()
window.toggleInteractivity()
```

### Reactivity Control
```javascript
// Toggle mouse/click/scroll reactivity per system
window.toggleSystemReactivity('holographic', 'mouse', true)

// Toggle audio reactivity per frequency band
window.toggleAudioReactivity('bass', 'rotation', true)
```

---

## Embedding in a Webpage

### Method 1: Full-Page Background
```html
<!DOCTYPE html>
<html>
<head>
  <title>My Page</title>
  <style>
    .vib3-bg {
      position: fixed;
      inset: 0;
      z-index: -1;
    }
    .vib3-bg iframe {
      width: 100%;
      height: 100%;
      border: none;
    }
    .content {
      position: relative;
      z-index: 1;
    }
  </style>
</head>
<body>
  <div class="vib3-bg">
    <iframe src="https://domusgpt.github.io/vib3-plus-engine/"></iframe>
  </div>
  <div class="content">
    <h1>Your Content Here</h1>
  </div>
</body>
</html>
```

### Method 2: With URL Parameters
The engine supports URL state restoration via Ctrl+L share links. Example:
```
https://domusgpt.github.io/vib3-plus-engine/?[encoded-state]
```

To get a shareable URL:
1. Configure the visualization how you want
2. Press Ctrl+L to copy the URL
3. Use that URL in your iframe

---

## Quick Start Examples

### Purple Holographic Background
```javascript
window.switchSystem('holographic');
window.selectGeometry(11);  // Hypersphere Torus
window.updateParameter('hue', 280);
window.updateParameter('intensity', 0.6);
```

### Animated Torus with Mouse Interaction
```javascript
window.switchSystem('quantum');
window.selectGeometry(3);  // Base Torus
window.toggleSystemReactivity('quantum', 'mouse', true);
```

### Music Visualizer Setup
```javascript
window.switchSystem('holographic');
window.toggleAudio();  // Enable audio reactivity
window.toggleAudioReactivity('bass', 'intensity', true);
window.toggleAudioReactivity('mid', 'rotation', true);
```

---

## Live Demo & Source

- **Demo**: https://domusgpt.github.io/vib3-plus-engine/
- **Source**: https://github.com/Domusgpt/vib3-plus-engine
- **SDK**: https://github.com/Domusgpt/vib34d-vib3plus (npm: vib34d-xr-quaternion-sdk)

---

## Common Mistakes to Avoid

1. **Don't use system=polychora for production** - Use faceted, quantum, or holographic
2. **Remember geometry is 0-23, not 1-24** - Zero-indexed
3. **Rotation values are radians** - Range is -6.28 to 6.28 (roughly -2π to 2π)
4. **Press H for help** - Shows all shortcuts in the app
