# Embedding VIB3+ in Your Webpage

The simplest way to use VIB3+ is embedding the deployed application via iframe.

## When to Use This Method

- Adding animated backgrounds to existing sites
- Quick prototypes without build tools
- Static configurations that don't need runtime changes

## Basic Embedding

```html
<iframe
  src="https://domusgpt.github.io/vib3-plus-engine/?system=holographic&geometry=5&hue=280"
  style="position:fixed;inset:0;width:100%;height:100%;border:none;z-index:-1">
</iframe>
```

## URL Parameters

### Required: System Selection

| Value | Description |
|-------|-------------|
| `faceted` | 2D geometric patterns |
| `quantum` | 3D lattice/interference effects |
| `holographic` | Audio-reactive dimensional blending |

**Note:** Do NOT use `polychora` - it is a placeholder and does not work.

### Geometry (0-23)

Formula: `geometry = coreIndex * 8 + baseIndex`

**Base shapes (0-7):**
| Index | Shape |
|-------|-------|
| 0 | Tetrahedron |
| 1 | Hypercube |
| 2 | Sphere |
| 3 | Torus |
| 4 | Klein Bottle |
| 5 | Fractal |
| 6 | Wave |
| 7 | Crystal |

**Core warps (0-2):**
| Index | Effect |
|-------|--------|
| 0 | Base (no warp) |
| 1 | Hypersphere warp |
| 2 | Hypertetrahedron warp |

### Visual Parameters

| Parameter | Range | Default | Purpose |
|-----------|-------|---------|---------|
| `gridDensity` | 5-100 | 15 | Pattern detail level |
| `morphFactor` | 0-2 | 1.0 | Shape distortion |
| `chaos` | 0-1 | 0.2 | Randomness |
| `speed` | 0.1-3 | 1.0 | Animation rate |
| `intensity` | 0-1 | 0.5 | Brightness |

### Color Parameters

| Parameter | Range | Default | Purpose |
|-----------|-------|---------|---------|
| `hue` | 0-360 | 200 | Base color (degrees) |
| `saturation` | 0-1 | 0.8 | Color purity |

## Common Patterns

### Full-page Background

```html
<!DOCTYPE html>
<html>
<head>
    <style>
        body { margin: 0; min-height: 100vh; }
        .background {
            position: fixed;
            inset: 0;
            z-index: -1;
        }
        .background iframe {
            width: 100%;
            height: 100%;
            border: none;
        }
        .content {
            position: relative;
            z-index: 1;
            color: white;
            padding: 2rem;
        }
    </style>
</head>
<body>
    <div class="background">
        <iframe src="https://domusgpt.github.io/vib3-plus-engine/?system=holographic&geometry=3&hue=280&intensity=0.6"></iframe>
    </div>
    <div class="content">
        <h1>Your Content Here</h1>
    </div>
</body>
</html>
```

### Card/Section Background

```html
<div style="position:relative;height:400px;overflow:hidden;border-radius:12px">
    <iframe
        src="https://domusgpt.github.io/vib3-plus-engine/?system=quantum&geometry=1&hue=200"
        style="position:absolute;inset:0;width:100%;height:100%;border:none">
    </iframe>
    <div style="position:relative;z-index:1;padding:2rem;color:white">
        Card content
    </div>
</div>
```

### Music Visualizer

Use the holographic system with audio enabled:

```html
<iframe src="https://domusgpt.github.io/vib3-plus-engine/?system=holographic&geometry=5&audioEnabled=true"></iframe>
```

The holographic system responds to bass, mid, and treble frequencies.

## Recommended Attributes

```html
<iframe
    src="..."
    loading="lazy"
    allow="accelerometer; autoplay; gyroscope"
    title="VIB3+ visualization">
</iframe>
```

## Live Demo

Test parameters interactively: https://domusgpt.github.io/vib3-plus-engine/

## Source Repository

https://github.com/Domusgpt/vib3-plus-engine
