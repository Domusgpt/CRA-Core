# VIB3+ Development Atlas

Context injection for building with the VIB3+ shader visualization system.

## What VIB3+ Is

VIB3+ creates **shader-based animated backgrounds and visual effects** for web design. It is NOT a 3D/4D geometry renderer - the "4D" refers to additional rotation parameters that create morphing visual effects.

## Working Systems

| System | Description | Status |
|--------|-------------|--------|
| Faceted | 2D geometric patterns | ✅ Working |
| Quantum | 3D lattice/interference effects | ✅ Working |
| Holographic | Audio-reactive dimensional blending | ✅ Working |
| Polychora | Complex polytope patterns | ❌ Placeholder - DO NOT USE |

## Use Cases

- Landing page animated backgrounds
- Portfolio site visuals
- Music visualizers (holographic system)
- Interactive art installations
- Creative web experiences

## Integration Methods

### 1. Iframe Embedding (Simplest)
For adding backgrounds to existing sites. No build tools needed.
→ See: [Embedding Guide](./embedding.md)

### 2. Cloning & Customizing
For modifying UI, adding features, or removing elements.
→ See: [Customization Guide](./customizing.md)

### 3. SDK Development
For React/Vue/Flutter apps, AR/XR, or custom input methods.
→ See: [SDK Guide](./sdk.md)

## Quick Start

```html
<iframe
  src="https://domusgpt.github.io/vib3-plus-engine/?system=holographic&geometry=5&hue=280"
  style="position:fixed;inset:0;width:100%;height:100%;border:none;z-index:-1">
</iframe>
```

## Source Repositories

| Repository | Purpose | URL |
|------------|---------|-----|
| vib3-plus-engine | Deployed app, embedding | https://github.com/Domusgpt/vib3-plus-engine |
| vib34d-vib3plus | npm SDK | https://github.com/Domusgpt/vib34d-vib3plus |

## Live Demo

https://domusgpt.github.io/vib3-plus-engine/

## Atlas Files

- `atlases/vib3-webpage-development.json` - CRA atlas definition
- `docs/atlases/vib3/` - This documentation
