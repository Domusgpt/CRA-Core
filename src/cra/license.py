from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, Tuple


class LicenseManager:
    """Minimal license checker for Atlases.

    Licenses are expected in `config/licenses.json` with structure:
    {"atlases": {"atlas.id": {"status": "active", "key": "...", "model": "subscription"}}}
    """

    def __init__(self, config_dir: Path | str = Path("config")) -> None:
        self.config_dir = Path(config_dir)
        self.config_dir.mkdir(parents=True, exist_ok=True)
        self.license_file = self.config_dir / "licenses.json"

    def check(self, atlas_manifest: Dict) -> Tuple[bool, str]:
        licensing = atlas_manifest.get("licensing", {})
        model = licensing.get("model", "free")
        atlas_id = atlas_manifest.get("id", "unknown.atlas")

        if model == "free":
            return True, "free model"

        licenses = self._load()
        atlas_license = licenses.get("atlases", {}).get(atlas_id)
        if atlas_license and atlas_license.get("status") == "active":
            return True, atlas_license.get("model", model)

        return False, f"license required for {atlas_id} (model={model})"

    def register(self, atlas_id: str, key: str, model: str) -> Dict:
        licenses = self._load()
        atlases = licenses.setdefault("atlases", {})
        atlases[atlas_id] = {"key": key, "status": "active", "model": model}
        self._save(licenses)
        return atlases[atlas_id]

    def _load(self) -> Dict:
        if not self.license_file.exists():
            return {}
        try:
            return json.loads(self.license_file.read_text())
        except json.JSONDecodeError:
            return {}

    def _save(self, payload: Dict) -> None:
        self.license_file.write_text(json.dumps(payload, indent=2))


__all__ = ["LicenseManager"]
