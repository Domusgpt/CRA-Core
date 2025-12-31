# VIB3 Atlas Verification Plan

## Purpose

Verify that the VIB3 atlas (`atlases/vib3-webpage-development.json`) contains accurate information by reading the actual VIB3 repositories.

---

## Claims To Verify

### From `vib3-essential-facts` block:

| Claim | How To Verify | Status |
|-------|---------------|--------|
| Faceted system is WORKING | Find faceted code, check if functional | ? |
| Quantum system is WORKING | Find quantum code, check if functional | ? |
| Holographic system is WORKING | Find holographic code, check if functional | ? |
| Polychora is PLACEHOLDER ONLY | Find polychora code, check if placeholder | ? |
| VIB3+ creates shader backgrounds | Read what the system actually does | ? |
| Does NOT render literal 3D/4D shapes | Verify shader output, not geometry | ? |

### From `vib3-workflow-embed` block:

| Claim | How To Verify | Status |
|-------|---------------|--------|
| Geometry formula: `coreIndex * 8 + baseIndex` | Find geometry code, verify formula | ? |
| Base 0-7: tetra, hypercube, sphere, torus, klein, fractal, wave, crystal | Find geometry definitions | ? |
| Core 0-2: base, hypersphere warp, hypertetra warp | Find core definitions | ? |
| URL params: system, geometry, hue, gridDensity, intensity, speed | Check actual URL parsing | ? |
| iframe embed approach works | Verify demo URL works | ? |

### From `vib3-workflow-customize` block:

| Claim | How To Verify | Status |
|-------|---------------|--------|
| Everything in index.html (~42KB) | Check file structure | ? |
| Global API: switchSystem, selectGeometry, updateParameter, randomizeAll | Find these functions | ? |
| Keyboard shortcuts 1-3, Alt+keys, Space, ? | Find keyboard handlers | ? |

### From `vib3-workflow-sdk` block:

| Claim | How To Verify | Status |
|-------|---------------|--------|
| npm package: vib34d-xr-quaternion-sdk | Check if package exists | ? |
| SensoryInputBridge, QuaternionFieldService, ShaderQuaternionSynchronizer exist | Find these classes | ? |
| publishPose method exists | Find method | ? |

### Repository URLs:

| URL | Verify Exists |
|-----|---------------|
| https://github.com/Domusgpt/vib3-plus-engine | ? |
| https://github.com/Domusgpt/vib34d-vib3plus | ? |
| https://domusgpt.github.io/vib3-plus-engine/ (demo) | ? |

---

## Verification Process

### Step 1: Fetch Repository Information

```
WebFetch: https://github.com/Domusgpt/vib3-plus-engine
- Does repo exist?
- What's in README?
- What files exist?

WebFetch: https://github.com/Domusgpt/vib34d-vib3plus
- Does repo exist?
- What's in README?
- What's the npm package name?
```

### Step 2: Read Key Files

For vib3-plus-engine:
- README.md - What does it say the system does?
- index.html - What systems exist? What's the geometry code?
- Any CLAUDE.md or docs - Authoritative info

For vib34d-vib3plus:
- README.md / QUICKSTART.md
- package.json - What's the npm name?
- src/ - What classes exist?

### Step 3: Verify Each Claim

Go through table above, mark each as:
- ✅ CORRECT - Verified in code
- ❌ WRONG - Code says something different
- ⚠️ PARTIAL - Partially correct
- ❓ UNVERIFIED - Couldn't find evidence

### Step 4: Document Findings

For each wrong/partial claim:
- What did I claim?
- What does the code actually say?
- What should the atlas say instead?

### Step 5: Fix Atlas

Update `atlases/vib3-webpage-development.json` with correct information.

---

## Red Team Checkpoints

### Checkpoint 1: After fetching repos
- Did I actually read the content or just skim?
- Am I making assumptions about what files contain?
- Did I find the authoritative source (CLAUDE.md, README)?

### Checkpoint 2: After verifying claims
- For each ✅, do I have specific evidence (file:line)?
- For each ❌, am I being honest about what's wrong?
- Am I tempted to say "close enough"? That's bullshit.

### Checkpoint 3: Before fixing atlas
- Am I copying from verified sources or making up content?
- Does the new content match what I found?
- Would someone reading the repo agree with my atlas?

### Checkpoint 4: Final review
- Read the atlas out loud - does it sound like real documentation?
- Would this actually help an agent use VIB3?
- Is anything still assumed/unverified?

---

## Output Format

After verification, produce:

```markdown
## Verification Results

### Correct Claims
- [claim]: verified at [file:line or URL]

### Wrong Claims
- [claim]: I said X, but code says Y. Source: [file:line]

### Missing Information
- [important thing I didn't include]

### Atlas Corrections
- [specific change to make]
```

---

## Timeline

1. Fetch repos and read READMEs
2. Read key source files
3. Fill in verification table
4. Document findings
5. Fix atlas
6. Commit with honest summary

No simulations. No assumptions. Just read the code and report what it says.
