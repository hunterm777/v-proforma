"""proforma: an auditable CRE pro forma you can version, diff and run anywhere.

All math runs in the Rust core (``proforma._proforma``). This module holds none:
it passes the deal file text to the core and wraps the JSON it returns.
"""

from __future__ import annotations

import json
import os
from collections.abc import Iterator, Mapping, Sequence
from typing import Any, Literal, Union, cast

from . import _proforma

__all__ = [
    "Deal",
    "Grid",
    "Result",
    "Table",
    "Trace",
    "__version__",
    "load",
    "run",
    "schema",
    "sensitivity",
]

__version__: str = _proforma.__version__

Unit = Literal["dollars", "cents"]
Report = Literal["annual", "monthly", "rent_roll", "tenants", "debt", "returns", "waterfall", "entries"]


def _money(cents: int, unit: Unit) -> Union[int, float]:
    if unit == "cents":
        return cents
    if unit == "dollars":
        return cents / 100
    raise ValueError(f'unit must be "dollars" or "cents", not {unit!r}')


class Deal:
    """A deal file: its text (YAML or JSON), checked by the core on load."""

    def __init__(self, text: str, path: str | None = None) -> None:
        _proforma.validate(text)
        self.text = text
        self.path = path

    def to_dict(self) -> dict[str, Any]:
        """The deal as plain Python data, as the core reads it."""
        return cast(dict[str, Any], json.loads(_proforma.to_json(self.text)))

    def to_yaml(self) -> str:
        return _proforma.to_yaml(self.text)

    def to_json(self) -> str:
        return _proforma.to_json(self.text)

    def save(self, path: str | os.PathLike[str]) -> None:
        """Writes the deal as YAML, or JSON when the path ends in .json."""
        p = os.fspath(path)
        out = self.to_json() if p.endswith(".json") else self.to_yaml()
        with open(p, "w", encoding="utf-8") as f:
            f.write(out)

    def __repr__(self) -> str:
        return f"Deal({self.path or 'text'})"


def load(source: str | os.PathLike[str]) -> Deal:
    """Loads a deal from a .yaml/.yml/.json path, or from deal file text."""
    s = os.fspath(source)
    if "\n" not in s and os.path.exists(s):
        with open(s, encoding="utf-8") as f:
            return Deal(f.read(), s)
    if "\n" not in s and s.endswith((".yaml", ".yml", ".json")):
        raise FileNotFoundError(s)
    return Deal(s)


def _text(deal: Deal | str | os.PathLike[str]) -> str:
    return deal.text if isinstance(deal, Deal) else load(deal).text


class Table(Mapping[str, list[Any]]):
    """A column-oriented table: ``pandas.DataFrame(table)`` and
    ``polars.DataFrame(dict(table))`` both work. The first column is ``line``."""

    def __init__(self, columns: dict[str, list[Any]]) -> None:
        self._columns = columns

    def __getitem__(self, key: str) -> list[Any]:
        return self._columns[key]

    def __iter__(self) -> Iterator[str]:
        return iter(self._columns)

    def __len__(self) -> int:
        return len(self._columns)

    @property
    def rows(self) -> list[dict[str, Any]]:
        keys = list(self._columns)
        n = len(self._columns[keys[0]]) if keys else 0
        return [{k: self._columns[k][i] for k in keys} for i in range(n)]

    def to_pandas(self) -> Any:
        import pandas as pd  # type: ignore[import-untyped, unused-ignore]

        return pd.DataFrame(self._columns)

    def __repr__(self) -> str:
        return f"Table({list(self._columns)})"


class Trace:
    """The entries behind one line over a span of months."""

    def __init__(self, data: dict[str, Any], unit: Unit) -> None:
        self.line: str = data["line"]
        self.from_month: int = data["from_month"]
        self.to_month: int = data["to_month"]
        self.total = _money(data["total"], unit)
        self.rows: list[dict[str, Any]] = [
            {
                "kind": r["source"]["kind"],
                "id": r["source"]["id"],
                "line": r["line"],
                "step": r["step"],
                "detail": r["detail"],
                "amount": _money(r["cents"], unit),
            }
            for r in data["rows"]
        ]

    def table(self) -> Table:
        keys = ["kind", "id", "line", "step", "detail", "amount"]
        return Table({k: [r[k] for r in self.rows] for k in keys})

    def __repr__(self) -> str:
        return f"Trace({self.line}, months {self.from_month}-{self.to_month}, total {self.total}, {len(self.rows)} rows)"


class Result:
    """The core's full output for one deal. ``raw`` is the result JSON as data."""

    def __init__(self, deal_text: str, raw: dict[str, Any]) -> None:
        self._deal = deal_text
        self.raw = raw

    @property
    def engine(self) -> str:
        return str(self.raw["engine"])

    def deal_bar(self) -> dict[str, Any]:
        return dict(self.raw["deal_bar"])

    def returns(self, unit: Unit = "dollars") -> dict[str, dict[str, Any]]:
        """Unlevered and levered IRR, XIRR, multiple, cash-on-cash by year,
        peak equity, payback month and the cash flow series."""
        out: dict[str, dict[str, Any]] = {}
        for k in ("unlevered", "levered"):
            s = dict(self.raw["returns"][k])
            s["cash_flows"] = [_money(c, unit) for c in s["cash_flows"]]
            for m in ("profit", "peak_equity"):
                s[m] = _money(s[m], unit)
            out[k] = s
        out["sources_uses"] = {k: _money(v, unit) for k, v in self.raw["returns"]["sources_uses"].items()}
        return out

    def cash_flow(self, period: Literal["annual", "monthly"] = "annual", unit: Unit = "dollars") -> Table:
        """PGR to levered cash flow by year (Year 0 = close) or by month."""
        if period not in ("annual", "monthly"):
            raise ValueError('period must be "annual" or "monthly"')
        t = self.raw[period]
        cols: dict[str, list[Any]] = {"line": [r["key"] for r in t["rows"]], "label": [r["label"] for r in t["rows"]]}
        for i, c in enumerate(t["columns"]):
            cols[c] = [_money(r["values"][i], unit) for r in t["rows"]]
        return Table(cols)

    def trace(self, line: str, year: int | None = None, month: int | None = None, unit: Unit = "dollars") -> Trace:
        """Which leases and assumptions produced ``line`` in analysis ``year``
        (1-based) or model ``month`` (1-based); neither means the whole hold."""
        return Trace(json.loads(_proforma.trace(self._deal, line, year, month)), unit)

    def rent_roll(self) -> dict[str, Any]:
        """WALT, occupancy by year and lease expirations by year."""
        return cast("dict[str, Any]", self.raw["rent_roll"])

    def tenants(self) -> list[dict[str, Any]]:
        """Per lease: rent, recoveries, renewal/new blend, TI/LC by year and month."""
        return cast("list[dict[str, Any]]", self.raw["tenants"])

    def debt(self) -> dict[str, Any] | None:
        """Sizing (which constraint bound), payment schedule, DSCR and debt yield by year."""
        return cast("dict[str, Any] | None", self.raw["debt"])

    def exit(self) -> dict[str, Any]:
        return cast("dict[str, Any]", self.raw["exit"])

    def waterfall(self) -> dict[str, Any] | None:
        """Distributions by tier and partner, LP and GP returns, promote."""
        return cast("dict[str, Any] | None", self.raw["waterfall"])

    def checks(self) -> list[dict[str, Any]]:
        """The four invariants, checked on this run."""
        return cast("list[dict[str, Any]]", self.raw["checks"])

    def to_csv(self, report: Report) -> str:
        return _proforma.csv(self._deal, report)

    def to_json(self) -> str:
        return json.dumps(self.raw, separators=(",", ":"))

    def __repr__(self) -> str:
        b = self.raw["deal_bar"]
        return f"Result({b['name']}, levered IRR {b['levered_irr']})"


def run(deal: Deal | str | os.PathLike[str]) -> Result:
    """Runs a deal (a Deal, a path, or deal file text)."""
    text = _text(deal)
    return Result(text, json.loads(_proforma.run(text)))


class Grid:
    """A two-way sensitivity: ``levered_irr[y][x]`` and ``equity_multiple[y][x]``."""

    def __init__(self, data: dict[str, Any]) -> None:
        self.x: str = data["x"]
        self.y: str = data["y"]
        self.x_values: list[float] = data["x_values"]
        self.y_values: list[float] = data["y_values"]
        self.levered_irr: list[list[float | None]] = data["levered_irr"]
        self.equity_multiple: list[list[float | None]] = data["equity_multiple"]
        self.errors: list[list[str]] = data["errors"]

    def table(self, measure: Literal["levered_irr", "equity_multiple"] = "levered_irr") -> Table:
        """Rows are y values, columns are x values."""
        m = getattr(self, measure)
        cols: dict[str, list[Any]] = {self.y: list(self.y_values)}
        for i, xv in enumerate(self.x_values):
            cols[f"{self.x}={xv}"] = [row[i] for row in m]
        return Table(cols)

    def __repr__(self) -> str:
        return f"Grid({self.y} x {self.x}, {len(self.y_values)}x{len(self.x_values)})"


def sensitivity(
    deal: Deal | str | os.PathLike[str],
    x: str,
    y: str,
    x_values: Sequence[float],
    y_values: Sequence[float],
) -> Grid:
    """Two-way levered IRR and equity multiple across two inputs, by dotted
    path (``exit.cap_rate``, ``market.rent_growth`` sets every market profile)."""
    spec = json.dumps({"x": x, "y": y, "x_values": list(x_values), "y_values": list(y_values)})
    return Grid(json.loads(_proforma.sensitivity(_text(deal), spec)))


def schema() -> dict[str, Any]:
    """The deal file JSON Schema."""
    return cast(dict[str, Any], json.loads(_proforma.schema()))
