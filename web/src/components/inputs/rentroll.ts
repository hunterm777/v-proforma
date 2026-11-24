// Rent roll tab: the lease table, or the unit mix for multifamily.
import { h } from "../../dom";
import type { Deal, Lease, UnitType } from "../../types";
import { copyLast, dateCol, numCol, put, textCol } from "../cols";
import { editTable } from "../table";
import { capCol, inline, leaseRecoveryCol, paramCol, stepTypeCol, stepValueCol } from "./recovery";

const PASTE_NOTE = "Paste rows from a spreadsheet (tab-separated) into any cell; columns fill left to right from there.";

function leases(deal: Deal, changed: () => void): HTMLElement {
  const rows = (deal.rent_roll ??= []);
  const blank: Lease = {
    id: "L1",
    tenant: "New tenant",
    sf: 1000,
    start: deal.property.start_date,
    expiry: deal.property.start_date,
    rent_psf_yr: 0,
  };
  return editTable<Lease>({
    rows: () => rows,
    make: (rs) => {
      const r = copyLast(rs, blank, "L");
      r.tenant = "New tenant";
      return r;
    },
    errorKey: (r, i) => `rent_roll[${i}] (${r.id})`,
    addLabel: "Add lease",
    onChange: changed,
    cols: [
      textCol("ID", (r) => r.id, (r, v) => (r.id = v ?? r.id), { required: true }),
      textCol("Tenant", (r) => r.tenant, (r, v) => (r.tenant = v ?? ""), { wide: true }),
      textCol("Suite", (r) => r.suite, (r, v) => put(r, "suite", v)),
      numCol("SF", (r) => r.sf, (r, v) => (r.sf = v ?? r.sf), { required: true }),
      dateCol("Start", (r) => r.start, (r, v) => (r.start = v)),
      dateCol("Expiry", (r) => r.expiry, (r, v) => (r.expiry = v)),
      numCol("Rent $/SF/yr", (r) => r.rent_psf_yr, (r, v) => (r.rent_psf_yr = v ?? r.rent_psf_yr), { required: true }),
      stepTypeCol<Lease>(),
      stepValueCol<Lease>(),
      numCol("Free mo", (r) => r.free_rent_months, (r, v) => put(r, "free_rent_months", v), { integer: true }),
      leaseRecoveryCol<Lease>(() => deal),
      paramCol<Lease>(inline),
      capCol<Lease>(inline),
      numCol("Pro rata %", (r) => r.pro_rata_share, (r, v) => put(r, "pro_rata_share", v), {
        scale: 100,
        title: "Override of SF / rentable SF; blank uses that",
      }),
      textCol("Space type", (r) => r.space_type, (r, v) => put(r, "space_type", v), {
        suggest: () => (deal.market ?? []).map((m) => m.id),
      }),
      numCol("Deposit $", (r) => r.security_deposit, (r, v) => put(r, "security_deposit", v)),
    ],
  });
}

function unitMix(deal: Deal, changed: () => void): HTMLElement {
  const rows = (deal.unit_mix ??= []);
  const blank: UnitType = {
    id: "u1",
    units: 1,
    in_place_rent_mo: 0,
    market_rent_mo: 0,
    rent_growth: 0,
    loss_to_lease_burn_off_months: 0,
    turnover_rate: 0,
  };
  return editTable<UnitType>({
    rows: () => rows,
    make: (rs) => copyLast(rs, blank, "u"),
    errorKey: (r, i) => `unit_mix[${i}] (${r.id})`,
    addLabel: "Add unit type",
    onChange: changed,
    cols: [
      textCol("ID", (r) => r.id, (r, v) => (r.id = v ?? r.id), { required: true }),
      textCol("Name", (r) => r.name, (r, v) => put(r, "name", v), { wide: true }),
      numCol("Units", (r) => r.units, (r, v) => (r.units = v ?? r.units), { required: true, integer: true }),
      numCol("In-place $/mo", (r) => r.in_place_rent_mo, (r, v) => (r.in_place_rent_mo = v ?? r.in_place_rent_mo), { required: true }),
      numCol("Market $/mo", (r) => r.market_rent_mo, (r, v) => (r.market_rent_mo = v ?? r.market_rent_mo), { required: true }),
      numCol("Growth %", (r) => r.rent_growth, (r, v) => (r.rent_growth = v ?? r.rent_growth), { scale: 100, required: true }),
      numCol("Burn-off mo", (r) => r.loss_to_lease_burn_off_months, (r, v) => (r.loss_to_lease_burn_off_months = v ?? 0), {
        integer: true,
        required: true,
        title: "Months for in-place rents to reach market; 0 = none",
      }),
      numCol("Turnover %", (r) => r.turnover_rate, (r, v) => (r.turnover_rate = v ?? r.turnover_rate), { scale: 100, required: true }),
      numCol("Concession mo", (r) => r.concession_months, (r, v) => put(r, "concession_months", v)),
    ],
  });
}

export function rentRollTab(deal: Deal, changed: () => void): HTMLElement {
  const mf = deal.property.type === "multifamily";
  return h(
    "section",
    { class: "tab-body" },
    h("h3", {}, mf ? "Unit mix" : "Rent roll"),
    mf ? unitMix(deal, changed) : leases(deal, changed),
    h("p", { class: "note" }, PASTE_NOTE),
  );
}
