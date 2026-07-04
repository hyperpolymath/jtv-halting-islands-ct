import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

EXAMPLES_DIR = ROOT / "examples"


@pytest.fixture
def example_source():
    def _load(name: str) -> str:
        return (EXAMPLES_DIR / name).read_text(encoding="utf-8")

    return _load
