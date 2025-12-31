# VIB3+ 4D Shader System Reference

Complete reference for building webpages using the VIB3+ visualization engine.

## System Overview

VIB3+ is a WebGL-based 4D visualization engine with three core systems:
- **Faceted** - 2D pattern generation with geometric effects
- **Quantum** - 3D lattice structures and interference patterns
- **Holographic** - Audio-reactive visualizations with dimensional blending

Each system renders across **5 glassmorphic canvas layers** with **24 geometries** and **6D rotation control**.

## Quick Start

### Minimal HTML Structure

```html
<!DOCTYPE html>
<html>
<head>
    <style>
        #visualization-container {
            position: fixed;
            top: 0; left: 0;
            width: 100vw; height: 100vh;
            overflow: hidden;
        }
        .visualization-canvas {
            position: absolute;
            top: 0; left: 0;
            width: 100%; height: 100%;
        }
    </style>
</head>
<body>
    <div id="visualization-container"></div>
    <script src="path/to/vib3-engine.js"></script>
    <script>
        // Initialize the system
        const system = window.currentSystem || 'faceted';
        // System is auto-initialized on load
    </script>
</body>
</html>
```

### Required Global State

The engine expects these window globals:

```javascript
// User-controlled parameters
window.userParameterState = {
    geometry: 0,
    gridDensity: 15,
    morphFactor: 1.0,
    chaos: 0.2,
    speed: 1.0,
    hue: 200,
    saturation: 0.8,
    intensity: 0.5,
    rot4dXY: 0, rot4dXZ: 0, rot4dYZ: 0,  // 3D rotations
    rot4dXW: 0, rot4dYW: 0, rot4dZW: 0   // 4D rotations
};

// Active system ('faceted', 'quantum', 'holographic', 'polychora')
window.currentSystem = 'faceted';

// Module ready flag
window.moduleReady = true;

// Audio reactivity (optional)
window.audioEnabled = false;
window.audioReactive = { bass: 0, mid: 0, high: 0, energy: 0 };

// Interactivity flag
window.interactivityEnabled = true;
```

---

## Parameters Reference

### Visualization Parameters

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `gridDensity` | 4-100 | 15 | Mesh detail/pattern frequency |
| `morphFactor` | 0-2 | 1.0 | Shape distortion intensity |
| `chaos` | 0-1 | 0.2 | Random vertex displacement |
| `speed` | 0.1-3 | 1.0 | Animation rate |
| `dimension` | 3.0-4.5 | 4.0 | 4D projection influence |

### Color Parameters

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `hue` | 0-360 | 200 | Base color (degrees) |
| `saturation` | 0-1 | 0.8 | Color purity |
| `intensity` | 0-1 | 0.5 | Brightness level |

### Geometry Selection

| Parameter | Range | Description |
|-----------|-------|-------------|
| `geometry` | 0-23 | Combined geometry index |
| `geometryBase` | 0-7 | Base shape (see below) |
| `geometryCore` | 0-2 | Core warp type |

**Formula**: `geometry = geometryCore * 8 + geometryBase`

---

## 24-Geometry System

### Base Geometries (0-7)

| Index | Name | Description |
|-------|------|-------------|
| 0 | Tetrahedron | 4-faced polyhedron |
| 1 | Hypercube | 4D cube projection (tesseract) |
| 2 | Sphere | Curved continuous surface |
| 3 | Torus | Donut-shaped ring surface |
| 4 | Klein Bottle | Non-orientable 4D surface |
| 5 | Fractal | Self-similar recursive pattern |
| 6 | Wave | Interference/sinusoidal pattern |
| 7 | Crystal | Lattice/crystalline structure |

### Core Warp Types (0-2)

| Index | Name | Effect |
|-------|------|--------|
| 0 | Base | No transformation |
| 1 | Hypersphere | 4D spherical warp |
| 2 | Hypertetrahedron | 4D tetrahedral warp |

**Total**: 24 geometry variants (8 base x 3 cores)

---

## 6D Rotation System

### 3D Rotations (XY, XZ, YZ)

Standard 3D space rotations:

| Plane | Uniform | Range | Effect |
|-------|---------|-------|--------|
| XY | `u_rot4dXY` | -3.14 to 3.14 | Compass/heading rotation |
| XZ | `u_rot4dXZ` | -3.14 to 3.14 | Front-back tilt |
| YZ | `u_rot4dYZ` | -3.14 to 3.14 | Left-right tilt |

### 4D Rotations (XW, YW, ZW)

4th dimension (W-axis) projections:

| Plane | Uniform | Range | Effect |
|-------|---------|-------|--------|
| XW | `u_rot4dXW` | -6.28 to 6.28 | X projected into 4D |
| YW | `u_rot4dYW` | -6.28 to 6.28 | Y projected into 4D |
| ZW | `u_rot4dZW` | -6.28 to 6.28 | Z projected into 4D |

### Rotation Order

Applied sequentially:
1. XY, XZ, YZ (3D space)
2. XW, YW, ZW (4D hyperspace)
3. 4D-to-3D perspective projection

---

## 5-Layer Canvas System

Each visualization uses 5 stacked canvases:

| Layer | z-index | Scale | Opacity | Purpose |
|-------|---------|-------|---------|---------|
| Background | 1 | 1.5 | 0.25 | Ambient depth |
| Shadow | 2 | 1.2 | 0.4 | Depth shadows |
| Content | 3 | 1.0 | 0.85 | Main visualization |
| Highlight | 4 | 0.8 | 0.7 | Detail emphasis |
| Accent | 5 | 0.6 | 0.4 | Fine accents |

### Canvas Setup Code

```javascript
function createCanvasLayers(containerId) {
    const container = document.getElementById(containerId);
    const layers = ['background', 'shadow', 'content', 'highlight', 'accent'];

    layers.forEach((layer, index) => {
        const canvas = document.createElement('canvas');
        canvas.id = `${layer}-canvas`;
        canvas.className = 'visualization-canvas';
        canvas.style.zIndex = index + 1;
        container.appendChild(canvas);
    });
}
```

### Resize Handling

```javascript
function resizeCanvases() {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    document.querySelectorAll('.visualization-canvas').forEach(canvas => {
        canvas.width = window.innerWidth * dpr;
        canvas.height = window.innerHeight * dpr;
    });
}
window.addEventListener('resize', resizeCanvases);
```

---

## Interactivity System

### Mouse Movement Modes

| Mode | Parameters Affected |
|------|---------------------|
| Rotation | rot4dXW, rot4dYW, rot4dZW, hue |
| Velocity | chaos, speed, gridDensity, intensity |
| Distance | gridDensity, intensity, saturation |

### Click Modes

| Mode | Effect |
|------|--------|
| Burst | Color flash, chaos/speed boost with decay |
| Blast | Hue shift with wave effects |
| Ripple | Morphing geometry, distance-based intensity |

### Scroll Modes

| Mode | Effect |
|------|--------|
| Cycle | Grid density + hue rotation |
| Wave | Morph factor increments |
| Sweep | Cyclic stepping through parameters |

### Setting Up Interactivity

```javascript
// Update parameter function (exposed globally)
window.updateParameter = function(param, value) {
    window.userParameterState[param] = value;
    // Systems read from userParameterState each frame
};

// Mouse handler example
document.addEventListener('mousemove', (e) => {
    const x = e.clientX / window.innerWidth;
    const y = e.clientY / window.innerHeight;

    // Rotation mode: map mouse to 4D rotations
    window.updateParameter('rot4dXW', (x - 0.5) * 2 * Math.PI);
    window.updateParameter('rot4dYW', (y - 0.5) * 2 * Math.PI);
});
```

---

## Audio Reactivity

### Global Audio State

```javascript
window.audioEnabled = true;
window.audioReactive = {
    bass: 0,    // 0-1, low frequencies
    mid: 0,     // 0-1, mid frequencies
    high: 0,    // 0-1, high frequencies
    energy: 0   // 0-1, overall intensity
};
```

### Audio Mappings

| Frequency | Parameter | Multiplier |
|-----------|-----------|------------|
| Bass | rot4dXW | 3.0x |
| Mid | rot4dYW | 2.5x |
| Treble | rot4dZW | 2.0x |
| Energy | Cross-section depth | 0.5x |
| Bass | Hue modulation | ±60° |

### Audio Sensitivity Levels

```javascript
// LOW: 30% reactivity
window.audioReactive.bass *= 0.3;

// MEDIUM: 100% reactivity (default)
window.audioReactive.bass *= 1.0;

// HIGH: 200% reactivity
window.audioReactive.bass *= 2.0;
```

---

## Shader Uniforms Reference

### Essential Uniforms

```glsl
// Time and resolution
uniform float u_time;
uniform vec2 u_resolution;

// Geometry selection
uniform int u_geometry;      // 0-23
uniform int u_polytope;      // 0-5 (polychora only)

// 6D rotations
uniform float u_rot4dXY, u_rot4dXZ, u_rot4dYZ;
uniform float u_rot4dXW, u_rot4dYW, u_rot4dZW;

// Visualization
uniform float u_gridDensity;
uniform float u_morphFactor;
uniform float u_chaos;
uniform float u_speed;

// Color
uniform float u_hue;
uniform float u_saturation;
uniform float u_intensity;

// Interaction
uniform vec2 u_mouse;
uniform float u_mouseIntensity;
uniform float u_clickIntensity;

// Layer-specific
uniform vec3 u_layerColor;
uniform float u_layerScale;
uniform float u_layerOpacity;
```

### Glass Effect Uniforms (Polychora)

```glsl
uniform float u_refractionIndex;       // default: 1.5
uniform float u_chromaticAberration;   // RGB separation
uniform float u_noiseAmplitude;        // Surface noise
uniform float u_faceTransparency;
uniform float u_edgeThickness;
uniform float u_projectionDistance;    // 1.0-5.0
```

---

## Common Patterns

### Initialize a System

```javascript
function initVisualization(systemType) {
    window.currentSystem = systemType;
    window.moduleReady = true;

    createCanvasLayers('visualization-container');
    resizeCanvases();

    // System auto-starts when moduleReady is true
}
```

### Switch Geometries

```javascript
function setGeometry(baseIndex, coreIndex = 0) {
    const geometry = coreIndex * 8 + baseIndex;
    window.updateParameter('geometry', geometry);
}

// Examples:
setGeometry(1, 0);  // Hypercube, base
setGeometry(1, 1);  // Hypercube, hypersphere core
setGeometry(4, 2);  // Klein bottle, hypertetrahedron core
```

### Animate Rotations

```javascript
function animateRotations() {
    const time = Date.now() * 0.001;

    // Gentle 4D rotation
    window.updateParameter('rot4dXW', Math.sin(time * 0.3) * 0.5);
    window.updateParameter('rot4dYW', Math.cos(time * 0.4) * 0.5);
    window.updateParameter('rot4dZW', Math.sin(time * 0.5) * 0.3);

    requestAnimationFrame(animateRotations);
}
```

---

## Source Files Reference

| File | Purpose |
|------|---------|
| `PolychoraSystem.js` | 4D polytope renderer (5-cell to 120-cell) |
| `Visualizer.js` | Holographic visualization renderer |
| `AdaptiveSDK.js` | SDK initialization and configuration |
| `CanvasManager.js` | Canvas creation and lifecycle |
| `Parameters.js` | Parameter definitions and validation |
| `ReactivityManager.js` | Input handling (mouse, click, scroll, audio) |

---

## Debugging

### Check WebGL Support

```javascript
function checkWebGL() {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
    if (!gl) {
        console.error('WebGL not supported');
        return false;
    }
    return true;
}
```

### Monitor Parameters

```javascript
// Log parameter changes
const originalUpdate = window.updateParameter;
window.updateParameter = function(param, value) {
    console.log(`Parameter: ${param} = ${value}`);
    originalUpdate(param, value);
};
```

### Performance

- Cap device pixel ratio at 2x for mobile
- Use `requestAnimationFrame` for render loop
- Batch parameter updates when possible
