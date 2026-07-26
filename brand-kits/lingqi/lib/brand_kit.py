"""Re-export shared brand-kit loader (legacy path)."""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "lib"))
from brand_kit import *  # noqa: F403
