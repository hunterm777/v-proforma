// The example deals, bundled into the page at build time.
import industrial from "../../deals/industrial_example.yaml?raw";
import office from "../../deals/office_example.yaml?raw";
import retail from "../../deals/retail_example.yaml?raw";
import multifamily from "../../deals/multifamily_example.yaml?raw";

export const EXAMPLES: readonly { id: string; label: string; yaml: string }[] = [
  { id: "office", label: "Office example", yaml: office },
  { id: "industrial", label: "Industrial example", yaml: industrial },
  { id: "retail", label: "Retail example", yaml: retail },
  { id: "multifamily", label: "Multifamily example", yaml: multifamily },
];
