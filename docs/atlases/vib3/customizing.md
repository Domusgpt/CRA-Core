# Customizing VIB3+ Application

Clone and modify the VIB3+ application when you need to change UI, add features, or remove elements.

## When to Use This Method

- Modify UI controls or layout
- Add new features or interactions
- Change default behavior
- Remove unwanted elements
- White-label the application

## Getting Started

```bash
git clone https://github.com/Domusgpt/vib3-plus-engine
cd vib3-plus-engine
```

The entire application is in `index.html` (~42KB). No build tools required.

## Application Structure

### Key Sections in index.html

1. **CSS styles** (inline `<style>` block)
2. **HTML structure** (nav, canvas containers, control bezels)
3. **JS modules** (loaded in order)
4. **Global state initialization**

### Canvas Layers

VIB3+ uses 5 stacked canvas layers for the glassmorphic effect:

| Layer | Purpose |
|-------|---------|
| background | Ambient depth |
| shadow | Depth shadows |
| content | Main visualization |
| highlight | Detail emphasis |
| accent | Fine accents |

## Global API

### System Control

```javascript
// Switch visualization system
window.switchSystem('faceted');   // or 'quantum', 'holographic'
// NOTE: Do NOT use 'polychora' - it's a placeholder

// Select geometry (0-23)
window.selectGeometry(5);

// Update any parameter
window.updateParameter('hue', 280);
window.updateParameter('gridDensity', 25);

// Randomize all parameters
window.randomizeAll();

// Toggle audio reactivity
window.toggleAudio();
```

### Reading State

```javascript
// Current active system
window.currentSystem  // 'faceted', 'quantum', or 'holographic'

// All current parameters
window.userParameterState  // { hue, gridDensity, rot4dXW, ... }

// Initialization status
window.moduleReady  // true when ready
```

## Common Customizations

### Hide UI Controls

```javascript
// Hide the control panel
document.querySelector('.control-bezel').style.display = 'none';

// Hide navigation
document.querySelector('nav').style.display = 'none';
```

### Set Default Parameters

Modify the initialization in the script section:

```javascript
window.userParameterState = {
    geometry: 1,          // Start with hypercube
    hue: 280,             // Purple base color
    gridDensity: 20,
    intensity: 0.7,
    // ... other parameters
};

window.currentSystem = 'holographic';  // Default system
```

### Add Custom Mouse Interaction

```javascript
document.addEventListener('mousemove', (e) => {
    const x = (e.clientX / window.innerWidth - 0.5) * 6.28;
    const y = (e.clientY / window.innerHeight - 0.5) * 6.28;

    window.updateParameter('rot4dXW', x);
    window.updateParameter('rot4dYW', y);
});
```

### Cycle Through Geometries

```javascript
let currentGeo = 0;
setInterval(() => {
    window.selectGeometry(currentGeo);
    currentGeo = (currentGeo + 1) % 24;
}, 3000);
```

## Keyboard Shortcuts

For testing during development:

| Key | Action |
|-----|--------|
| `1` | Switch to Faceted |
| `2` | Switch to Quantum |
| `3` | Switch to Holographic |
| `Alt+Q/W/E/R/A/S/D/F` | Select geometries 0-7 |
| `Space` | Randomize all |
| `G` | Open gallery |
| `?` | Show help modal |

**Note:** Key `4` would select Polychora but it's a placeholder - don't use.

## Deploying

The application is static HTML - deploy to any static host:

- GitHub Pages
- Netlify
- Vercel
- Any web server

```bash
# Example: Deploy to GitHub Pages
git add .
git commit -m "Customized VIB3+"
git push origin main
# Enable Pages in repository settings
```

## Source Repository

https://github.com/Domusgpt/vib3-plus-engine

Key files to read:
- `CLAUDE.md` - Development status
- `TESTING_GUIDE.md` - Testing workflow
- `24-GEOMETRY-6D-ROTATION-SUMMARY.md` - Technical details
