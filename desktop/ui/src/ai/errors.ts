// Typed error for AI surfaces that haven't been wired to a real provider yet.
// All current AI entry points (board generation, theme tweaks) reject with
// this so the UI can render a consistent "coming soon" affordance without
// special-casing each call site. Foundry Local integration lands in a
// follow-up.
export class AiNotImplementedError extends Error {
  constructor() {
    super(
      "AI integration is not wired up yet. Foundry Local support is coming in a follow-up.",
    );
    this.name = "AiNotImplementedError";
  }
}
