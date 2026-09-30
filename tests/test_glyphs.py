import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_glyph_reference_matches_symbols(monkeypatch):
    monkeypatch.chdir(ROOT)
    assert runpy.run_path('scripts/prep.py')['check']() == []
