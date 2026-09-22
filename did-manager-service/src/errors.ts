export type ManagerHttpError = {
  statusCode: number;
  errorCode: string;
  message: string;
};

export class ManagerServiceError extends Error {
  constructor(
    readonly errorCode: string,
    readonly statusCode: number,
    message: string,
  ) {
    super(message);
    this.name = new.target.name;
  }
}

export class ManagerInvalidRequestError extends ManagerServiceError {
  constructor(message: string) {
    super("invalidRequest", 400, message);
  }
}

export class ManagerInvalidSeedError extends ManagerServiceError {
  constructor(message: string) {
    super("invalidSeed", 400, message);
  }
}

export class ManagerConflictError extends ManagerServiceError {
  constructor(errorCode: "operationBusy" | "sessionLocked", message: string) {
    super(errorCode, 409, message);
  }
}

export class ManagerNotFoundError extends ManagerServiceError {
  constructor(
    errorCode: "contractNotFound" | "operationNotFound" | "secretNotFound",
    message: string,
  ) {
    super(errorCode, 404, message);
  }
}

export class ManagerUpstreamUnavailableError extends ManagerServiceError {
  constructor(message: string) {
    super("upstreamUnavailable", 503, message);
  }
}

const zodLikeMessage = (error: unknown): string | null => {
  if (
    typeof error !== "object" ||
    error === null ||
    !("issues" in error) ||
    !Array.isArray((error as { issues?: unknown[] }).issues)
  ) {
    return null;
  }

  const messages = (error as { issues: Array<{ message?: unknown }> }).issues
    .map((issue) => (typeof issue.message === "string" ? issue.message : null))
    .filter((message): message is string => message !== null);

  if (messages.length === 0) return null;
  return Array.from(new Set(messages)).join("; ");
};

const upstreamFailurePatterns = [
  "ECONNREFUSED",
  "timed out",
  "Resolution timed out",
  "fetch failed",
  "socket hang up",
];

const invalidRequestMessagePatterns = [
  "verificationMethod",
  "serviceEndpoint",
  "relation ",
];

export const classifyManagerHttpError = (error: unknown): ManagerHttpError => {
  if (error instanceof ManagerServiceError) {
    return {
      statusCode: error.statusCode,
      errorCode: error.errorCode,
      message: error.message,
    };
  }

  const zodMessage = zodLikeMessage(error);
  const message =
    zodMessage ??
    (error instanceof Error ? error.message : "Unexpected manager error");
  const errorName = error instanceof Error ? error.name : "Error";
  const fastifyValidationError =
    typeof error === "object" &&
    error !== null &&
    "validation" in error &&
    Array.isArray((error as { validation?: unknown[] }).validation);

  if (fastifyValidationError) {
    return {
      statusCode: 400,
      errorCode: "invalidRequest",
      message,
    };
  }

  if (zodMessage !== null || errorName === "ZodError") {
    return {
      statusCode: 400,
      errorCode: "invalidSeed",
      message,
    };
  }

  if (message.startsWith("Seed ")) {
    return {
      statusCode: 400,
      errorCode: "invalidSeed",
      message,
    };
  }

  if (errorName === "SecretNotFoundError") {
    return {
      statusCode: 404,
      errorCode: "secretNotFound",
      message,
    };
  }

  if (errorName === "SecretStoreLockedError") {
    return {
      statusCode: 400,
      errorCode: "secretStoreLocked",
      message,
    };
  }

  if (upstreamFailurePatterns.some((pattern) => message.includes(pattern))) {
    return {
      statusCode: 503,
      errorCode: "upstreamUnavailable",
      message,
    };
  }

  if (message.includes("Session is locked") || message.includes("Session is closed")) {
    return {
      statusCode: 409,
      errorCode: "sessionLocked",
      message,
    };
  }

  if (
    message.includes("Profile name") ||
    message.includes("No stored seed") ||
    message.includes("Funding is not prepared") ||
    message.includes("does not match the prepared funding seed") ||
    message.includes("Prepared funding state is inconsistent") ||
    message.includes("Secret-store passphrase is required") ||
    message.includes("Seed mode generated is not allowed for Start session") ||
    invalidRequestMessagePatterns.some((pattern) => message.includes(pattern))
  ) {
    return {
      statusCode: 400,
      errorCode: "invalidRequest",
      message,
    };
  }

  if (
    message.startsWith("Active DID contract could not be resolved") ||
    message.includes("was not found on")
  ) {
    return {
      statusCode: 404,
      errorCode: "contractNotFound",
      message,
    };
  }

  return {
    statusCode: 500,
    errorCode: "internalError",
    message,
  };
};
