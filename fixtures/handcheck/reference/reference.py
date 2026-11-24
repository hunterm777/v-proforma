#!/usr/bin/env python3
"""
Independent reference model for the proforma CRE underwriting method.

Written only from docs/methodology.md and the fixture files, as a hand-check of the engine.
It reads one deal YAML file and writes one JSON file of results.

    python3 reference.py ../../deals/m2_office_rollover.yaml out.json
    python3 reference.py --all            # every fixtures/deals/*.yaml -> out/<name>.json

Conventions used throughout (see methodology.md section numbers in comments):

* Money is integer cents. Dollar inputs are converted on load with
  round-half-away-from-zero (dollars * 100). Per-sf dollar rates (rent_psf_yr,
  market rent, ti_psf, stop_psf_yr, reserves_per_sf_yr, step amounts) are
  converted the same way, so they are cents per sf.
* Every monthly cell of every source (one lease or rollover path, one expense
  line, one income line, one capex item, one assumption) is computed in float
  and rounded ONCE with rha() below. Totals are exact integer sums of cells.
* Sign convention: every line is reported as a positive amount. Deductions
  (general_vacancy, credit_loss, free_rent, opex, management_fee, ti, lc,
  reserves, capex) are positive numbers that are SUBTRACTED. egi, noi and
  cash_flow_before_debt are the signed results of those subtractions.
  Investor cash flows (unlevered, levered, LP, GP) use the investor's sign:
  negative = money in, positive = money out to the investor.
"""

import calendar
import datetime as dt
import glob
import json
import math
import os
import sys

import yaml

HERE = os.path.dirname(os.path.abspath(__file__))


# ---------------------------------------------------------------------------
# Small helpers: rounding and dates
# ---------------------------------------------------------------------------

def rha(x):
    """Round half away from zero to an integer (not Python's banker's round)."""
    a = abs(x)
    f = math.floor(a)
    if a - f >= 0.5:
        f += 1
    return int(f) if x >= 0 else -int(f)


def cents(dollars):
    """Dollars from the deal file -> integer cents (methodology 1)."""
    return rha(float(dollars) * 100.0)


def days_in_month(y, m):
    return calendar.monthrange(y, m)[1]


def add_months(d, n):
    """d + n calendar months. If the day does not exist in the target month
    (e.g. Jan 31 + 1 month) it is clamped to the month's last day."""
    total = d.year * 12 + (d.month - 1) + n
    y, m = divmod(total, 12)
    m += 1
    return dt.date(y, m, min(d.day, days_in_month(y, m)))


ONE_DAY = dt.timedelta(days=1)


class Calendar:
    """Model months (methodology 2). Month m = 1 is the start month; month m
    covers [start + (m-1) months, start + m months)."""

    def __init__(self, start, n_months):
        if start.day != 1:
            raise ValueError("property.start_date must be the first of a month")
        self.start = start
        self.n = n_months
        self.end = add_months(start, n_months) - ONE_DAY  # last modelled day

    def month_of(self, d):
        """Model month containing date d (can be <= 0 or > n)."""
        return (d.year - self.start.year) * 12 + (d.month - self.start.month) + 1

    def first_day(self, m):
        return add_months(self.start, m - 1)

    def last_day(self, m):
        return add_months(self.start, m) - ONE_DAY

    def days(self, m):
        d = self.first_day(m)
        return days_in_month(d.year, d.month)

    @staticmethod
    def year_index(m):
        """Analysis year y(m) = floor((m - 1) / 12), 0-based, negative before start."""
        return (m - 1) // 12

    def months_of_calendar_year(self, year):
        first = self.month_of(dt.date(year, 1, 1))
        return list(range(first, first + 12))


def grown(value, growth, m):
    """value * (1 + g)^y(m): growth compounds once per analysis year."""
    return value * (1.0 + growth) ** Calendar.year_index(m)


# ---------------------------------------------------------------------------
# Loading
# ---------------------------------------------------------------------------

def deep_merge(base, over):
    out = dict(base)
    for k, v in (over or {}).items():
        if isinstance(v, dict) and isinstance(out.get(k), dict):
            out[k] = deep_merge(out[k], v)
        else:
            out[k] = v
    return out


def load_deal(path):
    with open(path) as fh:
        return yaml.safe_load(fh)


# ---------------------------------------------------------------------------
# Expenses, pool and other income (methodology 4, 5)
# ---------------------------------------------------------------------------

class Expenses:
    def __init__(self, deal, cal):
        self.cal = cal
        self.lines = []
        for ln in deal.get("opex", {}).get("lines", []):
            self.lines.append({
                "id": ln["id"],
                "amount_c": cents(ln["amount_yr"]),
                "growth": float(ln.get("growth", 0.0)),
                "recoverable": bool(ln.get("recoverable", False)),
                "controllable": bool(ln.get("controllable", False)),
            })

    def line_float(self, ln, m):
        """Unrounded monthly amount of one line in month m (any m, incl. <= 0)."""
        return grown(ln["amount_c"], ln["growth"], m) / 12.0

    def cells(self):
        """Rounded monthly cells per line, months 1..N."""
        return {ln["id"]: [rha(self.line_float(ln, m)) for m in range(1, self.cal.n + 1)]
                for ln in self.lines}

    def pool(self, m, only=None):
        """(P, C) in month m as floats: recoverable pool and its controllable part.
        `only` restricts the pool to a list of line ids (modified gross)."""
        p = c = 0.0
        for ln in self.lines:
            if not ln["recoverable"]:
                continue
            if only is not None and ln["id"] not in only:
                continue
            v = self.line_float(ln, m)
            p += v
            if ln["controllable"]:
                c += v
        return p, c


def other_income_cells(deal, cal):
    out = {}
    for ln in deal.get("other_income", []) or []:
        a = cents(ln["amount_yr"])
        g = float(ln.get("growth", 0.0))
        out[ln["id"]] = [rha(grown(a, g, m) / 12.0) for m in range(1, cal.n + 1)]
    return out


# ---------------------------------------------------------------------------
# Rent schedules (methodology 3)
# ---------------------------------------------------------------------------

class ContractRent:
    """Rent in effect on a date for a contractual lease with steps on each
    lease anniversary start + 12k months."""

    def __init__(self, start, end, rent_c, steps):
        self.rent = float(rent_c)
        self.steps = steps or {}
        self.anniversaries = []
        k = 1
        while True:
            a = add_months(start, 12 * k)
            if a > end:
                break
            self.anniversaries.append(a)
            k += 1

    def lease_year_k(self, d):
        """k = number of anniversaries on or before d (lease year k + 1)."""
        k = 0
        for a in self.anniversaries:
            if a <= d:
                k += 1
            else:
                break
        return k

    def rate(self, d):
        k = self.lease_year_k(d)
        t = self.steps.get("type")
        if k == 0 or not t:
            return self.rent
        if t == "percent":
            return self.rent * (1.0 + float(self.steps["rate"])) ** k
        if t == "fixed":
            return self.rent + k * cents(self.steps["amount_psf_yr"])
        if t == "cpi":
            rates = [float(r) for r in self.steps["rates"]]
            v = self.rent
            for i in range(k):
                v *= 1.0 + rates[min(i, len(rates) - 1)]
            return v
        raise ValueError("unknown step type %r" % t)


class FlatRent:
    """Speculative leases: market rent on the start date, flat for the term."""

    def __init__(self, rate_c):
        self.r = float(rate_c)

    def rate(self, d):
        return self.r


# ---------------------------------------------------------------------------
# Tenancies: one contractual lease or one weighted rollover path
# ---------------------------------------------------------------------------

class Tenancy:
    def __init__(self, lease_id, scenario, start, end, sf, share, rent, free_months,
                 recovery, weight, ti_psf_c=0, lc_pct=0.0, term_months=None):
        self.lease_id = lease_id
        self.scenario = scenario          # "contract", "renewal" or "new"
        self.start = start
        self.end = end                    # inclusive
        self.sf = float(sf)
        self.share = float(share)
        self.rent = rent
        self.free_end = add_months(start, int(free_months or 0))  # exclusive
        self.free_months = int(free_months or 0)
        self.recovery = recovery          # dict, base year already resolved
        self.weight = float(weight)
        self.ti_psf_c = ti_psf_c
        self.lc_pct = float(lc_pct)
        self.term_months = term_months


def recovery_float(ten, exp, cal, m, m0):
    """Full-month recovery for month m before occupancy fraction and weight
    (methodology 4). Returns a float in cents."""
    rec = ten.recovery
    t = rec.get("type", "gross")
    if t == "gross":
        return 0.0
    cap = rec.get("cap_controllable_pct")
    s = ten.share

    def capped_pool(only=None):
        p, c = exp.pool(m, only)
        if cap is None:
            return p
        _, c0 = exp.pool(m0, only)          # C(m0), projected backwards if m0 <= 0
        k = max(0, (m - m0) // 12)          # whole lease years elapsed at month m
        return p - c + min(c, c0 * (1.0 + float(cap)) ** k)

    if t == "nnn":
        return s * capped_pool()
    if t == "base_year":
        B = sum(exp.pool(mm)[0] for mm in cal.months_of_calendar_year(int(rec["year"])))
        return s * max(0.0, capped_pool() - B / 12.0)
    if t == "expense_stop":
        stop_c = cents(rec["stop_psf_yr"])
        return max(0.0, s * capped_pool() - stop_c * ten.sf / 12.0)
    if t == "modified_gross":
        return s * capped_pool(only=list(rec["lines"]))
    raise ValueError("unknown recovery type %r" % t)


def tenancy_cells(ten, exp, cal):
    """Monthly cells (each rounded once) for base rent, free rent, recoveries,
    TI and LC of one tenancy, months 1..N."""
    n = cal.n
    out = {k: [0] * n for k in ("base_rent", "free_rent", "recoveries", "ti", "lc")}
    w = ten.weight
    m0 = cal.month_of(ten.start)
    for m in range(1, n + 1):
        ms, me = cal.first_day(m), cal.last_day(m)
        a, b = max(ms, ten.start), min(me, ten.end)
        if a > b:
            continue
        D = cal.days(m)
        # Group the days in effect by (rate, abated).
        groups = {}
        d = a
        while d <= b:
            key = (ten.rent.rate(d), d < ten.free_end)
            groups[key] = groups.get(key, 0) + 1
            d += ONE_DAY
        base = free = 0.0
        occ = 0
        for (rate, abated), days in groups.items():
            amt = rate * ten.sf / 12.0 / D * days
            base += amt
            if abated:
                free += amt
            occ += days
        out["base_rent"][m - 1] = rha(w * base)
        out["free_rent"][m - 1] = rha(w * free)
        out["recoveries"][m - 1] = rha(w * recovery_float(ten, exp, cal, m, m0) * occ / D)
    # TI and LC: paid in the model month the (speculative) lease starts.
    if 1 <= m0 <= n and ten.scenario != "contract":
        out["ti"][m0 - 1] = rha(w * ten.ti_psf_c * ten.sf)
        monthly_rent = ten.rent.rate(ten.start) * ten.sf / 12.0
        out["lc"][m0 - 1] = rha(w * ten.lc_pct * monthly_rent
                                * (ten.term_months - ten.free_months))
    return out


# ---------------------------------------------------------------------------
# Rent roll and rollover (methodology 3, rollover)
# ---------------------------------------------------------------------------

def resolve_recovery(deal, rec):
    if rec is None:
        return {"type": "gross"}
    if isinstance(rec, str):
        return dict(deal.get("recoveries", {})[rec])
    return dict(rec)


def build_tenancies(deal, cal):
    """All tenancies (contract leases plus weighted rollover paths) and the
    turnover-vacancy memo entries, grouped by original lease id."""
    rentable = float(deal["property"]["rentable_sf"])
    profiles = {p["id"]: p for p in deal.get("market", []) or []}
    tenancies, tv_entries, paths = [], [], []

    for L in deal["rent_roll"]:
        lid = L["id"]
        sf = float(L["sf"])
        share = float(L.get("pro_rata_share", sf / rentable))
        rec = resolve_recovery(deal, L.get("recovery"))
        start, end = L["start"], L["expiry"]
        tenancies.append(Tenancy(
            lid, "contract", start, end, sf, share,
            ContractRent(start, end, cents(L["rent_psf_yr"]), L.get("steps")),
            L.get("free_rent_months", 0), rec, 1.0))

        # Rollover only if the space becomes free inside the model.
        free0 = end + ONE_DAY
        if free0 > cal.end or "space_type" not in L:
            continue
        prof = deep_merge(profiles[L["space_type"]], L.get("market"))
        p = float(prof["renewal_probability"])
        downtime = int(prof.get("downtime_months", 0))
        term = int(prof["term_months"])
        mkt_c = cents(prof["market_rent_psf_yr"])
        g = float(prof.get("rent_growth", 0.0))

        pending = {free0: 1.0}      # free date -> probability mass (merged by date)
        spec = {}                   # (start, scenario) -> weight (merged)
        while pending:
            d = min(pending)
            w = pending.pop(d)
            if d > cal.end:
                continue
            # Renewal: starts on the free date. New tenant: after the downtime.
            branches = [("renewal", d, w * p), ("new", add_months(d, downtime), w * (1.0 - p))]
            if downtime > 0 and w * (1.0 - p) > 0:
                tv_entries.append({"lease_id": lid, "from": d, "to": add_months(d, downtime),
                                   "weight": w * (1.0 - p), "sf": sf,
                                   "market_c": mkt_c, "growth": g})
            for scen, s, ws in branches:
                if ws <= 0 or s > cal.end:
                    continue
                spec[(s, scen)] = spec.get((s, scen), 0.0) + ws
                nxt = add_months(s, term)
                pending[nxt] = pending.get(nxt, 0.0) + ws

        for (s, scen), ws in sorted(spec.items()):
            blk = prof.get(scen, {})
            rate_c = mkt_c * (1.0 + g) ** Calendar.year_index(cal.month_of(s))
            srec = dict(rec)
            if srec.get("type") == "base_year":
                srec["year"] = s.year       # base year = calendar year of spec start
            e = add_months(s, term) - ONE_DAY
            tenancies.append(Tenancy(
                lid, scen, s, e, sf, share, FlatRent(rate_c),
                blk.get("free_rent_months", 0), srec, ws,
                ti_psf_c=cents(blk.get("ti_psf", 0)), lc_pct=blk.get("lc_pct", 0.0),
                term_months=term))
            paths.append({"lease_id": lid, "scenario": scen, "start": s.isoformat(),
                          "end": e.isoformat(), "weight": ws, "rent_psf_yr_cents": rate_c})
    return tenancies, tv_entries, paths


def turnover_vacancy_cells(e, cal):
    """Memo: market rent the space would have earned during downtime, weighted.
    Market rent in each downtime month is market * (1 + g)^y(m)."""
    out = [0] * cal.n
    last = e["to"] - ONE_DAY
    for m in range(1, cal.n + 1):
        a, b = max(cal.first_day(m), e["from"]), min(cal.last_day(m), last)
        if a > b:
            continue
        days = (b - a).days + 1
        rate = grown(e["market_c"], e["growth"], m)
        out[m - 1] = rha(e["weight"] * rate * e["sf"] / 12.0 / cal.days(m) * days)
    return out


# ---------------------------------------------------------------------------
# Debt (methodology 7)
# ---------------------------------------------------------------------------

def annuity_constant(r, n):
    return r / (1.0 - (1.0 + r) ** (-n))


def rate_fn(debt):
    r = debt["rate"]
    if "fixed" in r:
        f = float(r["fixed"])
        return lambda m: f
    fl = r["floating"]
    spread = float(fl["spread"])
    curve = sorted(fl["curve"], key=lambda p: p["month"])

    def rate(m):
        idx = curve[0]["index"]
        for pt in curve:                 # step function, first point at month 1
            if pt["month"] <= m:
                idx = pt["index"]
        return float(idx) + spread
    return rate


def run_debt(deal, noi, X):
    debt = deal["debt"]
    rate = rate_fn(debt)
    floating = "floating" in debt["rate"]
    io = int(debt.get("io_months", 0))
    amort_n = int(debt["amortization_years"]) * 12 if debt.get("amortization_years") else None
    price_c = cents(deal["capital"]["price"])
    noi1 = sum(noi[0:12])

    # Sizing
    candidates = {}
    if "amount" in debt:
        candidates["amount"] = float(cents(debt["amount"]))
    else:
        sz = debt["sizing"]
        if "max_ltv" in sz:
            candidates["ltv"] = float(sz["max_ltv"]) * price_c
        if "min_debt_yield" in sz:
            candidates["debt_yield"] = noi1 / float(sz["min_debt_yield"])
        if "min_dscr" in sz:
            r1 = rate(1) / 12.0
            k = annuity_constant(r1, amort_n) if amort_n else r1
            candidates["dscr"] = noi1 / (float(sz["min_dscr"]) * 12.0 * k)
    binding = min(candidates, key=lambda key: candidates[key])
    loan = int(math.floor(candidates[binding]))
    fee = rha(float(debt.get("origination_fee_pct", 0.0)) * loan)

    # Schedule to the exit month
    rows = []
    bal = loan
    fixed_payment = None
    for m in range(1, X + 1):
        r = rate(m)
        interest = rha(bal * r / 12.0)
        if m <= io or amort_n is None:
            payment = interest
        else:
            rm = r / 12.0
            if floating:
                n_left = amort_n - (m - io - 1)
                payment = rha(bal * annuity_constant(rm, n_left))
            else:
                if fixed_payment is None:  # computed once, on the balance at amortization start
                    fixed_payment = rha(bal * annuity_constant(rm, amort_n))
                payment = fixed_payment
        principal = payment - interest
        rows.append({"month": m, "rate": r, "opening": bal, "interest": interest,
                     "principal": principal, "payment": payment, "closing": bal - principal})
        bal -= principal
    payoff = bal

    pp = debt.get("prepayment_penalty_pct", 0.0)
    if isinstance(pp, list):
        loan_year = (X - 1) // 12
        pp_rate = float(pp[min(loan_year, len(pp) - 1)]) if pp else 0.0
    else:
        pp_rate = float(pp)
    prepay = rha(pp_rate * payoff)
    return {"noi_year1": noi1, "candidates": candidates, "binding": binding, "loan": loan,
            "fee": fee, "schedule": rows, "payoff": payoff,
            "prepayment_rate": pp_rate, "prepayment": prepay}


# ---------------------------------------------------------------------------
# Returns (methodology 9)
# ---------------------------------------------------------------------------

def npv(r, cfs):
    return sum(cf / (1.0 + r) ** t for t, cf in enumerate(cfs))


def irr_monthly(cfs):
    """Monthly IRR by bisection on NPV. None if there is no sign change."""
    if not (any(c < 0 for c in cfs) and any(c > 0 for c in cfs)):
        return None
    lo, hi = -0.99, 1.0
    f_lo, f_hi = npv(lo, cfs), npv(hi, cfs)
    while f_lo * f_hi > 0 and hi < 1e6:
        hi *= 2.0
        f_hi = npv(hi, cfs)
    if f_lo * f_hi > 0:
        return None
    for _ in range(400):
        mid = 0.5 * (lo + hi)
        f_mid = npv(mid, cfs)
        if f_mid == 0:
            return mid
        if (f_mid > 0) == (f_lo > 0):
            lo, f_lo = mid, f_mid
        else:
            hi = mid
        if hi - lo < 1e-16:
            break
    return 0.5 * (lo + hi)


def returns(cfs):
    rm = irr_monthly(cfs)
    pos = sum(c for c in cfs if c > 0)
    neg = -sum(c for c in cfs if c < 0)
    return {"cash_flows": cfs,
            "irr_monthly": rm,
            "irr": None if rm is None else (1.0 + rm) ** 12 - 1.0,
            "equity_multiple": pos / neg if neg else None}


# ---------------------------------------------------------------------------
# Waterfall (methodology 10)
# ---------------------------------------------------------------------------

class IrrAccount:
    """LP hurdle account: capital, capitalized interest, accrued interest."""
    PERIOD = {"monthly": 1, "quarterly": 3, "annual": 12, "none": None}

    def __init__(self, rate, compounding):
        self.rate = float(rate)
        self.period = self.PERIOD[compounding]
        self.capital = self.capitalized = self.accrued = 0.0

    def accrue(self, t):
        base = self.capital + (self.capitalized if self.period else 0.0)
        self.accrued += base * self.rate / 12.0
        if self.period and t % self.period == 0:
            self.capitalized += self.accrued
            self.accrued = 0.0

    def balance(self):
        return self.capital + self.capitalized + self.accrued

    def pay(self, amount):
        """Distributions pay accrued, then capitalized interest, then capital."""
        for attr in ("accrued", "capitalized", "capital"):
            take = min(amount, getattr(self, attr))
            setattr(self, attr, getattr(self, attr) - take)
            amount -= take


def run_waterfall(eq, levered):
    lp_c, gp_c = float(eq["contributions"]["lp"]), float(eq["contributions"]["gp"])
    tiers = eq["tiers"]
    T = len(levered)

    def split_of(tier):
        s = tier.get("split", "pro_rata")
        return lp_c if s == "pro_rata" else float(s["lp"])

    accounts = {}
    for i, tier in enumerate(tiers):
        h = tier.get("hurdle")
        if h and h["type"] == "irr":
            accounts[i] = IrrAccount(h["rate"], h.get("compounding", "monthly"))

    contrib = {"lp": [0] * T, "gp": [0] * T}
    tier_out = [{"name": t.get("name", "tier %d" % (i + 1)), "lp": [0] * T, "gp": [0] * T}
                for i, t in enumerate(tiers)]
    LPC = GPC = LPD = GPD = 0  # cumulative contributions / distributions (positive cents)

    for t, cf in enumerate(levered):
        if t > 0:
            for acc in accounts.values():
                acc.accrue(t)
        if cf < 0:
            lp = rha(-cf * lp_c)
            gp = -cf - lp
            contrib["lp"][t], contrib["gp"][t] = -lp, -gp
            LPC += lp
            GPC += gp
            for acc in accounts.values():
                acc.capital += lp
            continue
        left = cf
        for i, tier in enumerate(tiers):
            if left <= 0:
                break
            lps = split_of(tier)
            last = i == len(tiers) - 1
            h, until = tier.get("hurdle"), tier.get("until")
            if last and not h and not until:
                x = left                                   # residual
            elif until:
                tgt = float(until["gp_share_of_profits"])
                gp_profit = GPD - GPC
                tot_profit = (LPD + GPD) - (LPC + GPC)
                gps = 1.0 - lps
                # (gp_profit + gps*x) / (tot_profit + x) = tgt
                need = (tgt * tot_profit - gp_profit) / (gps - tgt) if gps != tgt else 0.0
                x = math.ceil(need) if need > 0 else 0
            else:
                if h and h["type"] == "irr":
                    need = accounts[i].balance()
                else:
                    mult = float(h["multiple"]) if h else 1.0  # no hurdle: return capital
                    need = mult * LPC - LPD
                x = math.ceil(need / lps) if need > 0 else 0
            x = min(x, left)
            lp = rha(x * lps)
            gp = x - lp
            tier_out[i]["lp"][t] += lp
            tier_out[i]["gp"][t] += gp
            LPD += lp
            GPD += gp
            for acc in accounts.values():              # hurdles measured on LP flows
                acc.pay(lp)
            left -= x

    lp_cf = [contrib["lp"][t] + sum(tr["lp"][t] for tr in tier_out) for t in range(T)]
    gp_cf = [contrib["gp"][t] + sum(tr["gp"][t] for tr in tier_out) for t in range(T)]
    total_dist = LPD + GPD
    gp_share = gp_c  # the GP's contribution share as given in `contributions`
    lp_r, gp_r = returns(lp_cf), returns(gp_cf)
    return {
        "contributions": contrib, "tiers": tier_out,
        "lp_cash_flows": lp_cf, "gp_cash_flows": gp_cf,
        "lp_contributions": LPC, "gp_contributions": GPC,
        "lp_distributions": LPD, "gp_distributions": GPD,
        "lp_irr": lp_r["irr"], "lp_irr_monthly": lp_r["irr_monthly"],
        "lp_multiple": lp_r["equity_multiple"],
        "gp_irr": gp_r["irr"], "gp_irr_monthly": gp_r["irr_monthly"],
        "gp_multiple": gp_r["equity_multiple"],
        "gp_contribution_share": gp_share,
        "gp_promote": rha(GPD - gp_share * total_dist),
    }


# ---------------------------------------------------------------------------
# Whole model
# ---------------------------------------------------------------------------

def add(a, b):
    return [x + y for x, y in zip(a, b)]


def run(deal):
    X = int(deal["exit"].get("month", deal["property"]["hold_months"]))
    N = X + 12
    cal = Calendar(deal["property"]["start_date"], N)
    zeros = [0] * N
    exp = Expenses(deal, cal)

    # Step 1 and 5: leases, rollover paths, recoveries
    tenancies, tv_entries, paths = build_tenancies(deal, cal)
    by_lease = {}
    for ten in tenancies:
        cells = tenancy_cells(ten, exp, cal)
        bl = by_lease.setdefault(ten.lease_id, {k: list(zeros) for k in cells})
        for k, v in cells.items():
            bl[k] = add(bl[k], v)
    tv = list(zeros)
    for e in tv_entries:
        tv = add(tv, turnover_vacancy_cells(e, cal))

    def total(key):
        out = list(zeros)
        for bl in by_lease.values():
            out = add(out, bl[key])
        return out

    L = {}
    L["base_rent"] = total("base_rent")
    L["recoveries"] = total("recoveries")
    L["free_rent"] = total("free_rent")
    L["ti"] = total("ti")
    L["lc"] = total("lc")
    oi = other_income_cells(deal, cal)
    L["other_income"] = list(zeros)
    for v in oi.values():
        L["other_income"] = add(L["other_income"], v)
    L["turnover_vacancy"] = tv

    # Steps 3 and 4: revenue
    rv = deal.get("revenue", {})
    gvp, clp = float(rv.get("general_vacancy_pct", 0.0)), float(rv.get("credit_loss_pct", 0.0))
    on_top = bool(rv.get("vacancy_on_top_of_downtime", False))
    L["pgr"] = [L["base_rent"][i] + L["recoveries"][i] + L["other_income"][i] for i in range(N)]
    L["general_vacancy"] = [rha(gvp * L["pgr"][i]) if on_top
                            else rha(max(0.0, gvp * L["pgr"][i] - tv[i])) for i in range(N)]
    L["credit_loss"] = [rha(clp * L["pgr"][i]) for i in range(N)]
    L["egi"] = [L["pgr"][i] - L["general_vacancy"][i] - L["credit_loss"][i] - L["free_rent"][i]
                for i in range(N)]

    # Step 2: expenses; management fee on EGI
    opex_lines = exp.cells()
    L["opex"] = list(zeros)
    for v in opex_lines.values():
        L["opex"] = add(L["opex"], v)
    mfp = float(deal.get("opex", {}).get("management_fee_pct", 0.0))
    L["management_fee"] = [rha(mfp * L["egi"][i]) for i in range(N)]
    L["noi"] = [L["egi"][i] - L["opex"][i] - L["management_fee"][i] for i in range(N)]

    # Capital items
    capd = deal["capital"]
    res = rha(cents(capd.get("reserves_per_sf_yr", 0)) * float(deal["property"]["rentable_sf"]) / 12.0)
    L["reserves"] = [res] * N
    L["capex"] = list(zeros)
    for item in capd.get("capex", []) or []:
        mm = int(item["month"])
        if 1 <= mm <= N:
            L["capex"][mm - 1] += cents(item["amount"])
    L["cash_flow_before_debt"] = [L["noi"][i] - L["ti"][i] - L["lc"][i] - L["reserves"][i]
                                  - L["capex"][i] for i in range(N)]

    # Step 8: exit
    cap_rate = float(deal["exit"]["cap_rate"])
    fwd = sum(L["noi"][X:X + 12])
    gross = rha(fwd / cap_rate)
    selling = rha(float(deal["exit"].get("selling_costs_pct", 0.0)) * gross)
    ex = {"month": X, "forward_noi": fwd, "gross_sale": gross,
          "selling_costs": selling, "net_sale": gross - selling}

    # Step 7 and 9: debt and returns
    price, closing = cents(capd["price"]), cents(capd.get("closing_costs", 0))
    cfbd = L["cash_flow_before_debt"]
    unlev = [-(price + closing)] + cfbd[:X]
    unlev[X] += ex["net_sale"]
    out = {"fixture": deal["property"].get("name"), "months": N, "exit_month": X,
           "sign_convention": "lines positive; deductions subtracted. Investor flows: "
                              "negative = contribution, positive = distribution.",
           "lines": L, "opex_by_line": opex_lines, "other_income_by_line": oi,
           "by_lease": by_lease, "rollover_paths": paths, "exit": ex}

    if "debt" in deal:
        d = run_debt(deal, L["noi"], X)
        equity = price + closing + d["fee"] - d["loan"]
        d["equity"] = equity
        d["sources"] = {"equity": equity, "loan": d["loan"]}
        d["uses"] = {"price": price, "closing_costs": closing, "loan_fees": d["fee"]}
        lev = [-equity] + [cfbd[m - 1] - d["schedule"][m - 1]["payment"] for m in range(1, X + 1)]
        lev[X] += ex["net_sale"] - d["payoff"] - d["prepayment"]
        out["debt"] = d
    else:
        lev = list(unlev)
    out["unlevered"] = returns(unlev)
    out["levered"] = returns(lev)

    if "equity" in deal and deal["equity"].get("tiers"):
        out["waterfall"] = run_waterfall(deal["equity"], lev)

    out["checks"] = self_check(out, tenancies, cal)
    return out


# ---------------------------------------------------------------------------
# Self-checks
# ---------------------------------------------------------------------------

def self_check(out, tenancies, cal):
    L, N = out["lines"], out["months"]
    checks = {}
    for key in ("base_rent", "free_rent", "recoveries", "ti", "lc"):
        s = [sum(bl[key][i] for bl in out["by_lease"].values()) for i in range(N)]
        checks["by_lease_sums_to_property_" + key] = s == L[key]
    checks["pgr_identity"] = all(L["pgr"][i] == L["base_rent"][i] + L["recoveries"][i]
                                 + L["other_income"][i] for i in range(N))
    if "debt" in out:
        d = out["debt"]
        checks["sources_equal_uses"] = sum(d["sources"].values()) == sum(d["uses"].values())
        checks["schedule_rolls_forward"] = all(
            r["closing"] == r["opening"] - r["principal"] for r in d["schedule"])
    for k in ("unlevered", "levered"):
        r = out[k]
        if r["irr_monthly"] is not None:
            scale = sum(abs(c) for c in r["cash_flows"])
            checks[k + "_npv_at_irr_rel"] = abs(npv(r["irr_monthly"], r["cash_flows"])) / scale
    if "waterfall" in out:
        w = out["waterfall"]
        lev = out["levered"]["cash_flows"]
        checks["lp_plus_gp_equals_levered"] = all(
            w["lp_cash_flows"][t] + w["gp_cash_flows"][t] == lev[t] for t in range(len(lev)))
        for who in ("lp", "gp"):
            cfs, rm = w[who + "_cash_flows"], w[who + "_irr_monthly"]
            if rm is not None:
                checks[who + "_npv_at_irr_rel"] = abs(npv(rm, cfs)) / sum(abs(c) for c in cfs)
    return checks


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def run_file(src, dst):
    out = run(load_deal(src))
    os.makedirs(os.path.dirname(dst) or ".", exist_ok=True)
    with open(dst, "w") as fh:
        json.dump(out, fh, indent=1, default=str)
    return out


def main(argv):
    if len(argv) == 2 and argv[1] == "--all":
        for src in sorted(glob.glob(os.path.join(HERE, "..", "..", "deals", "*.yaml"))):
            name = os.path.splitext(os.path.basename(src))[0]
            out = run_file(src, os.path.join(HERE, "out", name + ".json"))
            print(name, json.dumps(out["checks"]))
    elif len(argv) == 3:
        out = run_file(argv[1], argv[2])
        print(json.dumps(out["checks"]))
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
