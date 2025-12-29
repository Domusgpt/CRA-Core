# Building with VIB3+ SDK

Use the npm SDK when building custom applications with React, Vue, Flutter, or for AR/XR integration.

## When to Use This Method

- React/Vue/Flutter applications
- Custom input methods (touch, motion, gamepad)
- AR/XR applications
- Educational/scientific visualization tools
- Applications requiring programmatic control

## Installation

```bash
npm install vib34d-xr-quaternion-sdk
```

## Core Services

The SDK provides three main services:

### 1. SensoryInputBridge

Handles input from various sources (mouse, touch, motion sensors, AR tracking).

```javascript
import { SensoryInputBridge } from 'vib34d-xr-quaternion-sdk';

const bridge = new SensoryInputBridge();

// Publish pose data from any input source
bridge.publishPose(quaternion, position, confidence, source);
```

### 2. QuaternionFieldService

Manages the 4D rotation state and field computations.

```javascript
import { QuaternionFieldService } from 'vib34d-xr-quaternion-sdk';

const fieldService = new QuaternionFieldService();
```

### 3. ShaderQuaternionSynchronizer

Synchronizes input with shader parameters.

```javascript
import { ShaderQuaternionSynchronizer } from 'vib34d-xr-quaternion-sdk';

const synchronizer = new ShaderQuaternionSynchronizer({
    bridge,
    fieldService,
    onSystemUpdate: (rotationParams) => {
        // Receive 6D rotation values
        // rotationParams: { rot4dXY, rot4dXZ, rot4dYZ, rot4dXW, rot4dYW, rot4dZW }
    }
});
```

## Complete Setup Example

```javascript
import {
    SensoryInputBridge,
    QuaternionFieldService,
    ShaderQuaternionSynchronizer
} from 'vib34d-xr-quaternion-sdk';

// Initialize services
const bridge = new SensoryInputBridge();
const fieldService = new QuaternionFieldService();

// Create synchronizer with callback
const synchronizer = new ShaderQuaternionSynchronizer({
    bridge,
    fieldService,
    onSystemUpdate: (params) => {
        // Update your WebGL uniforms or canvas renderer
        updateVisualization(params);
    }
});

// Connect mouse input
document.addEventListener('mousemove', (e) => {
    const quaternion = mouseToQuaternion(e);
    const position = { x: e.clientX, y: e.clientY, z: 0 };
    bridge.publishPose(quaternion, position, 1.0, 'mouse');
});
```

## SDK Modules

| Module | Purpose |
|--------|---------|
| `./quaternion` | 4D rotation mathematics |
| `./sensors` | Input adapters for various sources |
| `./geometry` | 24-geometry library definitions |
| `./rotations` | 6-plane rotation composition |

## React Integration

```jsx
import { useEffect, useRef } from 'react';
import { SensoryInputBridge, QuaternionFieldService, ShaderQuaternionSynchronizer } from 'vib34d-xr-quaternion-sdk';

function VIB3Visualization() {
    const canvasRef = useRef(null);

    useEffect(() => {
        const bridge = new SensoryInputBridge();
        const fieldService = new QuaternionFieldService();

        const synchronizer = new ShaderQuaternionSynchronizer({
            bridge,
            fieldService,
            onSystemUpdate: (params) => {
                // Render to canvas
            }
        });

        return () => {
            // Cleanup
        };
    }, []);

    return <canvas ref={canvasRef} />;
}
```

## 6D Rotation Parameters

The SDK outputs these rotation parameters:

| Parameter | Range | Description |
|-----------|-------|-------------|
| `rot4dXY` | -π to π | XY plane (heading) |
| `rot4dXZ` | -π to π | XZ plane (pitch) |
| `rot4dYZ` | -π to π | YZ plane (roll) |
| `rot4dXW` | -2π to 2π | X into 4th dimension |
| `rot4dYW` | -2π to 2π | Y into 4th dimension |
| `rot4dZW` | -2π to 2π | Z into 4th dimension |

## AR/XR Input Adapters

The SDK includes adapters for:
- ARCore/ARKit pose tracking
- WebXR device orientation
- Gamepad/controller input

```javascript
import { ARKitAdapter } from 'vib34d-xr-quaternion-sdk/sensors';

const arAdapter = new ARKitAdapter(bridge);
arAdapter.start();
```

## Working Systems

The SDK supports these visualization systems:
- **Faceted** - 2D geometric patterns ✅
- **Quantum** - 3D lattice effects ✅
- **Holographic** - Audio-reactive ✅

**Note:** Polychora is NOT implemented in the SDK - it's a placeholder.

## Source Repository

https://github.com/Domusgpt/vib34d-vib3plus

Documentation in the repository:
- `QUICKSTART.md` - Minimal setup
- `WEB_APP_GUIDE.md` - Web integration details
- `DEVELOPER_GUIDE.md` - Full API reference
- `DOCS/` folder - 20+ detailed guides
