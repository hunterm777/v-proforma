// Returns tab: unlevered and levered IRR, XIRR, multiple, profit, peak
// equity, payback and cash-on-cash by year; with the sale and sources & uses.
import { mult, pct, usd } from "../../format";
import type { Result, Series } from "../../types";
import { facts, money, section, viewTable } from "./view";
import { h } from "../../dom";

const payback = (s: Series): string => (s.payback_month === null ? "Not within the hold" : `Month ${s.payback_month}`);

export function returnsView(r: Result): HTMLElement {
  const { unlevered: u, levered: l, sources_uses: su } = r.returns;
  const x = r.exit;
  const years = Math.max(u.cash_on_cash.length, l.cash_on_cash.length);
  return h(
    "div",
    { class: "stack" },
    section(
      "Returns",
      viewTable(
        ["Measure", "Unlevered", "Levered"],
        [
          ["IRR (annualized monthly)", pct(u.irr), pct(l.irr)],
          ["XIRR (actual dates)", pct(u.xirr), pct(l.xirr)],
          ["Equity multiple", mult(u.equity_multiple), mult(l.equity_multiple)],
          ["Profit", money(u.profit), money(l.profit)],
          ["Peak equity", money(u.peak_equity), money(l.peak_equity)],
          ["Payback", payback(u), payback(l)],
        ],
      ),
    ),
    section(
      "Cash-on-cash by year",
      h("p", { class: "note" }, "The year's operating cash flow over the year-0 investment."),
      viewTable(
        ["Year", "Unlevered", "Levered"],
        Array.from({ length: years }, (_, i) => [`Year ${i + 1}`, pct(u.cash_on_cash[i]), pct(l.cash_on_cash[i])]),
      ),
    ),
    section(
      "Sale",
      facts([
        ["Sale month", String(x.month)],
        ["Forward 12-month NOI", usd(x.forward_noi)],
        ["Exit cap rate", pct(x.cap_rate)],
        ["Gross sale", usd(x.gross_sale)],
        ["Selling costs", usd(x.selling_costs)],
        ["Net sale", usd(x.net_sale)],
      ]),
    ),
    section(
      "Sources and uses at close",
      facts([
        ["Equity", usd(su.equity)],
        ["Loan", usd(su.loan)],
        ["Price", usd(su.price)],
        ["Closing costs", usd(su.closing_costs)],
        ["Loan fees", usd(su.loan_fees)],
      ]),
    ),
  );
}
