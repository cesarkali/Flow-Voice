import overlay from "./overlay";
import shell from "./shell";
import controls from "./controls";
import onboarding from "./onboarding";
import home from "./home";
import models from "./models";
import features from "./features";

export const EXTRA = [overlay, shell, controls, onboarding, home, models, features] as const;

export type ExtraKey =
  | keyof (typeof overlay)["pt"]
  | keyof (typeof shell)["pt"]
  | keyof (typeof controls)["pt"]
  | keyof (typeof onboarding)["pt"]
  | keyof (typeof home)["pt"]
  | keyof (typeof models)["pt"]
  | keyof (typeof features)["pt"];
