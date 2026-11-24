"""Build hand-check workbooks (formulas only) from the deal YAML files."""
import sys, datetime as dt
import yaml
from openpyxl import Workbook
from openpyxl.utils import get_column_letter as CL

def build(yaml_path, out_path):
    d = yaml.safe_load(open(yaml_path))
    wb = Workbook()
    inp = wb.active
    inp.title = "Inputs"
    names = {}
    r = [1]

    def put(key, label, value, fmt=None):
        row = r[0]
        inp.cell(row, 1, key)
        inp.cell(row, 2, label)
        c = inp.cell(row, 3, value)
        if fmt:
            c.number_format = fmt
        names[key] = f"Inputs!$C${row}"
        r[0] += 1

    def head(t):
        inp.cell(r[0], 1, t)
        r[0] += 1

    p = d["property"]
    head("PROPERTY")
    put("prop_name", "name", p["name"])
    put("prop_type", "type", p["type"])
    put("rsf", "rentable_sf", p["rentable_sf"])
    put("start", "start_date", p["start_date"], "yyyy-mm-dd")
    put("hold", "hold_months", p["hold_months"])
    exit_month = d.get("exit", {}).get("month", p["hold_months"])
    put("X", "exit month X (exit.month, else hold_months)", "=" + names["hold"] if "month" not in d.get("exit", {}) else exit_month)
    put("nmonths", "model months = X + 12", f"={names['X']}+12")

    leases = d["rent_roll"]
    assert len(leases) == 1
    L = leases[0]
    head("RENT ROLL L1")
    put("l_id", "id", L["id"])
    put("l_tenant", "tenant", L["tenant"])
    put("l_suite", "suite", L["suite"])
    put("l_sf", "sf", L["sf"])
    put("l_start", "start", L["start"], "yyyy-mm-dd")
    put("l_exp", "expiry (inclusive)", L["expiry"], "yyyy-mm-dd")
    put("l_rent", "rent_psf_yr", L["rent_psf_yr"])
    assert L["steps"]["type"] == "percent"
    put("l_steptype", "steps.type", L["steps"]["type"])
    put("l_step", "steps.rate", L["steps"]["rate"])
    put("l_recovery", "recovery", L["recovery"])
    assert d["recoveries"][L["recovery"]]["type"] == "nnn"
    put("l_rectype", "recovery structure type", d["recoveries"][L["recovery"]]["type"])
    if "pro_rata_share" in L:
        put("l_share", "pro_rata_share", L["pro_rata_share"])
    else:
        put("l_share", "pro_rata_share = sf / rentable_sf", f"={names['l_sf']}/{names['rsf']}")
    assert "free_rent_months" not in L

    o = d["opex"]
    head("OPEX")
    put("mgmt", "management_fee_pct", o["management_fee_pct"])
    lines = o["lines"]
    for ln in lines:
        i = ln["id"]
        put(f"{i}_name", f"{i}.name", ln["name"])
        put(f"{i}_amt", f"{i}.amount_yr", ln["amount_yr"])
        put(f"{i}_g", f"{i}.growth", ln["growth"])
        put(f"{i}_rec", f"{i}.recoverable", bool(ln.get("recoverable", False)))
        put(f"{i}_ctl", f"{i}.controllable", bool(ln.get("controllable", False)))

    rv = d["revenue"]
    head("REVENUE")
    put("gv", "general_vacancy_pct", rv["general_vacancy_pct"])
    put("cl", "credit_loss_pct", rv["credit_loss_pct"])

    c = d["capital"]
    head("CAPITAL")
    put("price", "price", c["price"])
    put("closing", "closing_costs", c["closing_costs"])
    put("res", "reserves_per_sf_yr", c["reserves_per_sf_yr"])

    e = d["exit"]
    head("EXIT")
    put("cap", "cap_rate", e["cap_rate"])
    put("sell", "selling_costs_pct", e["selling_costs_pct"])

    debt = d.get("debt")
    if debt:
        head("DEBT")
        s = debt["sizing"]
        put("ltv", "sizing.max_ltv", s["max_ltv"])
        put("dscr", "sizing.min_dscr", s["min_dscr"])
        put("dy", "sizing.min_debt_yield", s["min_debt_yield"])
        put("rate", "rate.fixed", debt["rate"]["fixed"])
        put("io", "io_months", debt["io_months"])
        put("amyrs", "amortization_years", debt["amortization_years"])
        put("ofee", "origination_fee_pct", debt["origination_fee_pct"])
        pp = debt["prepayment_penalty_pct"]
        pp = pp if isinstance(pp, list) else [pp]
        first = r[0]
        for k, v in enumerate(pp):
            put(f"pp{k+1}", f"prepayment_penalty_pct loan year {k+1}", v)
        names["pp_range"] = f"Inputs!$C${first}:$C${r[0]-1}"
    inp.column_dimensions["A"].width = 14
    inp.column_dimensions["B"].width = 44
    inp.column_dimensions["C"].width = 26

    N = names
    nm = p["hold_months"] + 12 if "month" not in e else e["month"] + 12

    # ---------------- Monthly ----------------
    ms = wb.create_sheet("Monthly")
    cols = [
        ("m", None),
        ("month_start", None), ("next_month_start", None), ("days_D", None), ("analysis_year_y", None),
        ("L1_anniversaries_at_month_start_k", None), ("L1_next_anniversary", None),
        ("L1_rate_k", None), ("L1_rate_k+1", None),
        ("L1_active_from", None), ("L1_active_to_excl", None),
        ("L1_days_at_rate_k", None), ("L1_days_at_rate_k+1", None),
        ("L1_rate_in_effect_psf_yr(day-wtd)", None), ("L1_occupied_fraction", None),
        ("base_rent", None),
    ]
    hdr = [h for h, _ in cols]
    for ln in lines:
        hdr.append(f"opex_{ln['id']}")
    hdr += ["opex_total", "recoverable_pool_P_f64", "recoveries_L1_nnn", "PGR",
            "general_vacancy", "credit_loss", "EGI", "management_fee", "NOI",
            "capital_reserves", "cash_flow_before_debt"]
    col = {h: CL(i + 1) for i, h in enumerate(hdr)}
    for i, h in enumerate(hdr):
        ms.cell(1, i + 1, h)
    for m in range(1, nm + 1):
        rr = m + 1
        def C(h): return f"{col[h]}{rr}"
        f = {}
        f["m"] = m
        f["month_start"] = f"=EDATE({N['start']},{C('m')}-1)"
        f["next_month_start"] = f"=EDATE({N['start']},{C('m')})"
        f["days_D"] = f"={C('next_month_start')}-{C('month_start')}"
        f["analysis_year_y"] = f"=INT(({C('m')}-1)/12)"
        B = C("month_start")
        f["L1_anniversaries_at_month_start_k"] = (
            f"=IF({B}<{N['l_start']},0,INT(((YEAR({B})-YEAR({N['l_start']}))*12+MONTH({B})-MONTH({N['l_start']})"
            f"-IF(DAY({B})<DAY({N['l_start']}),1,0))/12))")
        k = C("L1_anniversaries_at_month_start_k")
        f["L1_next_anniversary"] = f"=EDATE({N['l_start']},12*({k}+1))"
        f["L1_rate_k"] = f"={N['l_rent']}*(1+{N['l_step']})^{k}"
        f["L1_rate_k+1"] = f"={N['l_rent']}*(1+{N['l_step']})^({k}+1)"
        f["L1_active_from"] = f"=MAX({B},{N['l_start']})"
        f["L1_active_to_excl"] = f"=MIN({C('next_month_start')},{N['l_exp']}+1)"
        AF, AT, NA = C("L1_active_from"), C("L1_active_to_excl"), C("L1_next_anniversary")
        f["L1_days_at_rate_k"] = f"=MAX(0,MIN({AT},{NA})-{AF})"
        f["L1_days_at_rate_k+1"] = f"=MAX(0,{AT}-MAX({AF},{NA}))"
        d1, d2, r1, r2 = C("L1_days_at_rate_k"), C("L1_days_at_rate_k+1"), C("L1_rate_k"), C("L1_rate_k+1")
        f["L1_rate_in_effect_psf_yr(day-wtd)"] = f"=IF({d1}+{d2}=0,0,({d1}*{r1}+{d2}*{r2})/({d1}+{d2}))"
        f["L1_occupied_fraction"] = f"=({d1}+{d2})/{C('days_D')}"
        f["base_rent"] = f"=ROUND(({d1}*{r1}+{d2}*{r2})*{N['l_sf']}/12/{C('days_D')},2)"
        y = C("analysis_year_y")
        pool = []
        for ln in lines:
            i = ln["id"]
            f[f"opex_{i}"] = f"=ROUND({N[i+'_amt']}*(1+{N[i+'_g']})^{y}/12,2)"
            pool.append(f"IF({N[i+'_rec']},{N[i+'_amt']}*(1+{N[i+'_g']})^{y}/12,0)")
        f["opex_total"] = f"=SUM({col['opex_'+lines[0]['id']]}{rr}:{col['opex_'+lines[-1]['id']]}{rr})"
        f["recoverable_pool_P_f64"] = "=" + "+".join(pool)
        f["recoveries_L1_nnn"] = f"=ROUND({N['l_share']}*{C('recoverable_pool_P_f64')}*{C('L1_occupied_fraction')},2)"
        f["PGR"] = f"={C('base_rent')}+{C('recoveries_L1_nnn')}"
        f["general_vacancy"] = f"=ROUND(MAX(0,{N['gv']}*{C('PGR')}-0),2)"
        f["credit_loss"] = f"=ROUND({N['cl']}*{C('PGR')},2)"
        f["EGI"] = f"={C('PGR')}-{C('general_vacancy')}-{C('credit_loss')}"
        f["management_fee"] = f"=ROUND({N['mgmt']}*{C('EGI')},2)"
        f["NOI"] = f"={C('EGI')}-{C('opex_total')}-{C('management_fee')}"
        f["capital_reserves"] = f"=ROUND({N['res']}*{N['rsf']}/12,2)"
        f["cash_flow_before_debt"] = f"={C('NOI')}-{C('capital_reserves')}"
        for h in hdr:
            cell = ms.cell(rr, hdr.index(h) + 1, f[h])
            if h in ("month_start", "next_month_start", "L1_next_anniversary", "L1_active_from", "L1_active_to_excl"):
                cell.number_format = "yyyy-mm-dd"
    last = nm + 1
    def rng(h): return f"Monthly!${col[h]}$2:${col[h]}${last}"

    # ---------------- Returns ----------------
    rs = wb.create_sheet("Returns")
    rs["A1"], rs["B1"] = "item", "value"
    rs["A2"], rs["B2"] = "exit month X", f"={N['X']}"
    rs["A3"], rs["B3"] = "forward NOI (months X+1..X+12)", (
        f'=SUMIFS({rng("NOI")},{rng("m")},">"&B2,{rng("m")},"<="&(B2+12))')
    rs["A4"], rs["B4"] = "gross sale = fwd NOI / cap", f"=ROUND(B3/{N['cap']},2)"
    rs["A5"], rs["B5"] = "selling costs", f"=ROUND({N['sell']}*B4,2)"
    rs["A6"], rs["B6"] = "net sale", "=B4-B5"
    rs["A7"], rs["B7"] = "total cost (price + closing)", f"={N['price']}+{N['closing']}"
    rs["A8"], rs["B8"] = "unlevered IRR monthly", None
    rs["A9"], rs["B9"] = "unlevered IRR annualised (1+r)^12-1", "=(1+B8)^12-1"
    rs["A10"], rs["B10"] = "unlevered equity multiple", None
    rs["A11"], rs["B11"] = "year-1 NOI (months 1-12)", f'=SUMIFS({rng("NOI")},{rng("m")},">=1",{rng("m")},"<=12")'
    rs["D1"], rs["E1"], rs["F1"], rs["G1"] = "t", "cash_flow_before_debt", "net_sale", "unlevered_cf"
    X = p["hold_months"] if "month" not in e else e["month"]
    for t in range(0, X + 1):
        rr = t + 2
        rs.cell(rr, 4, t)
        if t == 0:
            rs.cell(rr, 5, 0)
            rs.cell(rr, 6, 0)
            rs.cell(rr, 7, f"=-B7")
        else:
            rs.cell(rr, 5, f"=INDEX({rng('cash_flow_before_debt')},MATCH(D{rr},{rng('m')},0))")
            rs.cell(rr, 6, f"=IF(D{rr}=$B$2,$B$6,0)")
            rs.cell(rr, 7, f"=E{rr}+F{rr}")
    cfr = f"G2:G{X+2}"
    rs["B8"] = f"=IRR({cfr},0.005)"
    rs["B10"] = f'=SUMIF({cfr},">0")/-SUMIF({cfr},"<0")'
    rs.column_dimensions["A"].width = 40
    rs.column_dimensions["B"].width = 20

    # ---------------- Debt ----------------
    if debt:
        ds = wb.create_sheet("Debt")
        ds["A1"], ds["B1"] = "item", "value"
        rows = [
            ("NOI1 (year-1 NOI, months 1-12)", "=Returns!B11"),
            ("monthly rate r = rate/12", f"={N['rate']}/12"),
            ("amortization months n", f"={N['amyrs']}*12"),
            ("amortizing constant k = r/(1-(1+r)^-n)", "=B3/(1-(1+B3)^-B4)"),
            ("candidate LTV = max_ltv * price", f"={N['ltv']}*{N['price']}"),
            ("candidate DSCR = NOI1/(min_dscr*12*k)", f"=B2/({N['dscr']}*12*B5)"),
            ("candidate debt yield = NOI1/min_debt_yield", f"=B2/{N['dy']}"),
            ("lowest candidate", "=MIN(B6:B8)"),
            ("binding constraint", '=IF(B9=B6,"ltv",IF(B9=B7,"dscr","debt_yield"))'),
            ("loan (rounded DOWN to cent)", "=ROUNDDOWN(B9,2)"),
            ("origination fee", f"=ROUND({N['ofee']}*B11,2)"),
            ("amortizing payment = ROUND(PMT(r,n,-loan),2)", "=ROUND(PMT(B3,B4,-B11),2)"),
            ("io months", f"={N['io']}"),
            ("exit month X", "=Returns!B2"),
            ("payoff (closing balance at X)", None),
            ("loan year at X = INT((X-1)/12)+1", "=INT((B15-1)/12)+1"),
            ("prepayment penalty pct (last repeats)", f"=INDEX({N['pp_range']},MIN(B17,ROWS({N['pp_range']})))"),
            ("prepayment penalty", "=ROUND(B18*B16,2)"),
            ("TEXTBOOK: ROUND(PMT(0.06/12,360,-1000000),2)", "=ROUND(PMT(0.06/12,360,-1000000),2)"),
            ("TEXTBOOK expected", 5995.51),
            ("TEXTBOOK match", "=B20=B21"),
        ]
        for i, (a, b) in enumerate(rows):
            ds.cell(i + 2, 1, a)
            if b is not None:
                ds.cell(i + 2, 2, b)
        for j, h in enumerate(["month", "opening", "interest", "principal", "payment", "closing"]):
            ds.cell(1, 4 + j, h)
        for m in range(1, X + 1):
            rr = m + 1
            ds.cell(rr, 4, m)
            ds.cell(rr, 5, "=$B$11" if m == 1 else f"=I{rr-1}")
            ds.cell(rr, 6, f"=ROUND(E{rr}*{N['rate']}/12,2)")
            ds.cell(rr, 8, f"=IF(D{rr}<=$B$14,F{rr},$B$13)")
            ds.cell(rr, 7, f"=H{rr}-F{rr}")
            ds.cell(rr, 9, f"=E{rr}-G{rr}")
        ds["B16"] = f"=INDEX(I2:I{X+1},MATCH(B15,D2:D{X+1},0))"
        ds.column_dimensions["A"].width = 48
        ds.column_dimensions["B"].width = 20
    wb.save(out_path)
    return col, hdr, X, nm

if __name__ == "__main__":
    print(build(sys.argv[1], sys.argv[2])[2:])
