import overlay from "./overlay";
import shell from "./shell";
import controls from "./controls";
import onboarding from "./onboarding";
import home from "./home";

export const EXTRA = [overlay, shell, controls, onboarding, home] as const;

export type ExtraKey =
  | keyof (typeof overlay)["pt"]
  | keyof (typeof shell)["pt"]
  | keyof (typeof controls)["pt"]
  | keyof (typeof onboarding)["pt"]
  | keyof (typeof home)["pt"];
