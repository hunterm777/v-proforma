import sys, json, math, datetime as dt
from openpyxl import load_workbook

def cents(v):
    assert isinstance(v, (int, float)) and not isinstance(v, bool), v
    return int(round(v * 100))

def export(path, out, debt):
    wb = load_workbook(path, data_only=True)
    ms = wb["Monthly"]
    hdr = [c.value for c in ms[1]]
    rows = [dict(zip(hdr, [c.value for c in r])) for r in ms.iter_rows(min_row=2)]
    # every cached cell must be a real value (no None / error strings)
    for r in rows:
        for h, v in r.items():
            assert v is not None and not (isinstance(v, str) and v.startswith("#")), (r["m"], h, v)
    assert any(r["NOI"] != 0 for r in rows)
    key = {"base_rent": "base_rent", "recoveries": "recoveries_L1_nnn", "opex": "opex_total",
           "management_fee": "management_fee", "general_vacancy": "general_vacancy", "egi": "EGI",
           "noi": "NOI", "reserves": "capital_reserves", "cash_flow_before_debt": "cash_flow_before_debt"}
    res = {"months": [r["m"] for r in rows]}
    for k, h in key.items():
        res[k] = [cents(r[h]) for r in rows]
    # recoveries alternative reading: share * (sum of rounded opex cells)
    alt = [int(round(r["opex_total"] * 100)) for r in rows]
    res["_memo_recoveries_minus_sum_of_rounded_opex_cents"] = sorted(set(a - b for a, b in zip(res["recoveries"], alt)))
    rs = wb["Returns"]
    g = {rs.cell(i, 1).value: rs.cell(i, 2).value for i in range(2, 12)}
    res["exit_month"] = g["exit month X"]
    res["forward_noi"] = cents(g["forward NOI (months X+1..X+12)"])
    res["gross_sale"] = cents(g["gross sale = fwd NOI / cap"])
    res["selling_costs"] = cents(g["selling costs"])
    res["net_sale"] = cents(g["net sale"])
    res["year1_noi"] = cents(g["year-1 NOI (months 1-12)"])
    cf = [rs.cell(i, 7).value for i in range(2, 2 + res["exit_month"] + 1)]
    res["unlevered_cf"] = [cents(v) for v in cf]
    res["unlevered_irr_monthly"] = g["unlevered IRR monthly"]
    res["unlevered_irr"] = g["unlevered IRR annualised (1+r)^12-1"]
    res["unlevered_equity_multiple"] = g["unlevered equity multiple"]
    for k in ("unlevered_irr", "unlevered_irr_monthly", "unlevered_equity_multiple"):
        assert isinstance(res[k], float) and res[k] != 0, (k, res[k])
    if debt:
        ds = wb["Debt"]
        b = {ds.cell(i, 1).value: ds.cell(i, 2).value for i in range(2, 23)}
        res["noi1"] = cents(b["NOI1 (year-1 NOI, months 1-12)"])
        res["amortizing_constant"] = b["amortizing constant k = r/(1-(1+r)^-n)"]
        res["loan_candidates"] = {
            "ltv": b["candidate LTV = max_ltv * price"],
            "dscr": b["candidate DSCR = NOI1/(min_dscr*12*k)"],
            "debt_yield": b["candidate debt yield = NOI1/min_debt_yield"]}
        res["loan_candidates_floor_cents"] = {k: math.floor(round(v * 100, 6)) for k, v in res["loan_candidates"].items()}
        res["binding_constraint"] = b["binding constraint"]
        res["loan"] = cents(b["loan (rounded DOWN to cent)"])
        res["fee"] = cents(b["origination fee"])
        res["amortizing_payment"] = cents(b["amortizing payment = ROUND(PMT(r,n,-loan),2)"])
        res["payoff"] = cents(b["payoff (closing balance at X)"])
        res["prepayment_loan_year"] = b["loan year at X = INT((X-1)/12)+1"]
        res["prepayment_penalty_pct"] = b["prepayment penalty pct (last repeats)"]
        res["prepayment_penalty"] = cents(b["prepayment penalty"])
        res["textbook_pmt_1m_6pct_30y"] = cents(b["TEXTBOOK: ROUND(PMT(0.06/12,360,-1000000),2)"])
        res["textbook_match"] = b["TEXTBOOK match"]
        sched = []
        for r in ds.iter_rows(min_row=2, min_col=4, max_col=9, values_only=True):
            if r[0] is None:
                break
            sched.append({"month": r[0], "opening": cents(r[1]), "interest": cents(r[2]),
                          "principal": cents(r[3]), "payment": cents(r[4]), "closing": cents(r[5])})
        res["schedule"] = sched
    json.dump(res, open(out, "w"), indent=1)
    return res

if __name__ == "__main__":
    # python3 export.py <recalculated.xlsx> <out.json> [--debt]
    r = export(sys.argv[1], sys.argv[2], "--debt" in sys.argv[3:])
    print({k: v for k, v in r.items() if not isinstance(v, list)})
