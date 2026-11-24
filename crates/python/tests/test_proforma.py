"""Python package tests: parity with the native golden files, the PRD API, and
no network use. Run from the repo root: pytest crates/python/tests"""

from __future__ import annotations

import json
import socket
from typing import cast
from pathlib import Path

import pytest

import proforma as pf
from proforma import _proforma

ROOT = Path(__file__).resolve().parents[3]
DEALS = sorted([*ROOT.glob("fixtures/deals/*.yaml"), *ROOT.glob("deals/*.yaml")])


@pytest.fixture(autouse=True)
def no_network(monkeypatch: pytest.MonkeyPatch) -> None:
    def blocked(*args: object, **kwargs: object) -> None:
        raise AssertionError("proforma tried to open a network connection")

    monkeypatch.setattr(socket.socket, "connect", blocked)
    monkeypatch.setattr(socket, "create_connection", blocked)


def test_all_fixtures_and_examples_present() -> None:
    names = {p.stem for p in DEALS}
    assert {"industrial_example", "office_example", "retail_example", "multifamily_example"} <= names
    assert {"m1_nnn_industrial", "m2_office_rollover", "m3_fixed_sized", "m3_floating", "m4_waterfall"} <= names


@pytest.mark.parametrize("path", DEALS, ids=lambda p: p.stem)
def test_python_matches_native_golden_file(path: Path) -> None:
    golden = (ROOT / "fixtures/expected" / f"{path.stem}.json").read_text().rstrip("\n")
    assert _proforma.run(path.read_text()) == golden


@pytest.mark.parametrize("path", DEALS, ids=lambda p: p.stem)
def test_invariants(path: Path) -> None:
    for c in pf.run(pf.load(path)).checks():
        assert c["passed"], c


def test_prd_example() -> None:
    deal = pf.load(str(ROOT / "deals/industrial_example.yaml"))
    result = pf.run(deal)
    r = result.returns()
    assert set(r) == {"unlevered", "levered", "sources_uses"}
    assert r["levered"]["irr"] == result.raw["returns"]["levered"]["irr"]
    assert len(r["levered"]["cash_on_cash"]) == 10

    t = result.cash_flow("annual")
    assert t["line"][0] == "base_rent" and "Year 10" in t
    df = t.to_pandas()
    assert df.shape[0] == len(result.raw["annual"]["rows"])

    tr = result.trace("noi", year=3)
    assert (tr.from_month, tr.to_month) == (25, 36)
    noi = result.cash_flow("annual", unit="cents")
    assert tr.total * 100 == pytest.approx(noi["Year 3"][noi["line"].index("noi")], abs=0.5)
    assert {row["kind"] for row in tr.rows} >= {"lease", "expense"}

    g = pf.sensitivity(deal, x="exit.cap_rate", y="market.rent_growth",
                       x_values=[0.055, 0.06, 0.065], y_values=[0.02, 0.03, 0.04])
    assert len(g.levered_irr) == 3 and all(len(row) == 3 for row in g.levered_irr)
    # Higher exit cap, lower IRR; higher rent growth, higher IRR.
    irr = [[cast(float, v) for v in row] for row in g.levered_irr]
    assert irr[1][0] > irr[1][2]
    assert irr[2][1] > irr[0][1]


def test_cents_are_exact() -> None:
    result = pf.run(ROOT / "fixtures/deals/m1_nnn_industrial.yaml")
    cents = result.cash_flow("monthly", unit="cents")
    assert all(isinstance(v, int) for v in cents["2027-01"])
    trace = result.trace("base_rent", month=1, unit="cents")
    assert trace.total == 15557292  # $7.25 x 1.03 x 250,000 SF / 12


def test_load_from_text_and_round_trip(tmp_path: Path) -> None:
    src = ROOT / "deals/office_example.yaml"
    a = pf.run(pf.load(src))
    b = pf.run(pf.load(src.read_text()))
    assert a.to_json() == b.to_json()
    out = tmp_path / "deal.json"
    pf.load(src).save(out)
    assert pf.run(pf.load(out)).to_json() == a.to_json()


def test_csv_and_schema() -> None:
    result = pf.run(ROOT / "deals/multifamily_example.yaml")
    assert result.to_csv("annual").startswith("line,")
    assert pf.schema()["title"]
    assert pf.__version__ == result.engine


def test_errors_are_value_errors() -> None:
    text = (ROOT / "fixtures/deals/m1_nnn_industrial.yaml").read_text().replace("2027-01-01", "2027-01-15")
    with pytest.raises(ValueError, match="start_date"):
        pf.load(text)
    with pytest.raises(FileNotFoundError):
        pf.load("no_such_deal.yaml")
    with pytest.raises(ValueError):
        pf.run(ROOT / "deals/industrial_example.yaml").trace("nope", year=1)


def test_result_json_is_the_core_output() -> None:
    p = ROOT / "deals/retail_example.yaml"
    assert json.loads(pf.run(p).to_json()) == json.loads(_proforma.run(p.read_text()))
