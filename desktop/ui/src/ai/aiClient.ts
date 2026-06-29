import { AiNotImplementedError } from "./errors";

// Stub providers. They return Promise<string> so the ChatPanel API surface
// is shaped for the real implementation; the rejection path is what gets
// exercised today. When Foundry Local lands, swap these out without
// touching ChatPanel or its callers.
//
// The `prompt` parameter is intentionally ignored — typed as `string` so
// the future signature is preserved, with an underscore prefix and an
// eslint-disable to keep the placeholder honest.

/* eslint-disable @typescript-eslint/no-unused-vars */
export const generateBoardFromPrompt = (_prompt: string): Promise<string> =>
  Promise.reject(new AiNotImplementedError());

export const adjustThemeFromPrompt = (_prompt: string): Promise<string> =>
  Promise.reject(new AiNotImplementedError());
/* eslint-enable @typescript-eslint/no-unused-vars */
