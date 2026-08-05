import { type ResolutionErrorCode } from "./resolution-errors.js";

export const didResolutionErrorPayload = (error: ResolutionErrorCode) => ({
  didDocument: null,
  didDocumentMetadata: {},
  didResolutionMetadata: {
    contentType: null,
    error,
  },
});
