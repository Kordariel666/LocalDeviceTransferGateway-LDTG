import type { AppSettings, ShareValidation } from "@dmdc/shared";
import { text } from "./i18n";

export type DraftValidationErrors = {
  port?: string;
  maxUploadBytes?: string;
  maxInboxBytes?: string;
  maxInboxFiles?: string;
  downloadShare?: string;
  uploadShare?: string;
  shareOverlap?: string;
};

export function settingsEqual(left: AppSettings | null, right: AppSettings | null): boolean {
  if (!left || !right) return left === right;
  return left.version === right.version
    && left.downloadShare.enabled === right.downloadShare.enabled
    && left.downloadShare.path === right.downloadShare.path
    && left.uploadShare.enabled === right.uploadShare.enabled
    && left.uploadShare.path === right.uploadShare.path
    && left.preferredAdapterId === right.preferredAdapterId
    && left.port === right.port
    && left.maxUploadBytes === right.maxUploadBytes
    && left.maxInboxBytes === right.maxInboxBytes
    && left.maxInboxFiles === right.maxInboxFiles
    && left.idleTimeoutMinutes === right.idleTimeoutMinutes
    && left.trustedNetworks.length === right.trustedNetworks.length
    && left.trustedNetworks.every((network, index) => network === right.trustedNetworks[index]);
}

export function validateDraft(settings: AppSettings): DraftValidationErrors {
  const errors: DraftValidationErrors = {};
  if (!Number.isSafeInteger(settings.port) || settings.port < 1024 || settings.port > 65535) {
    errors.port = text.portValidation;
  }
  if (settings.maxUploadBytes !== null
    && (!Number.isSafeInteger(settings.maxUploadBytes) || settings.maxUploadBytes <= 0)) {
    errors.maxUploadBytes = text.positiveUploadLimit;
  }
  if (!Number.isSafeInteger(settings.maxInboxBytes) || settings.maxInboxBytes <= 0) {
    errors.maxInboxBytes = text.positiveInboxLimit;
  }
  if (!Number.isSafeInteger(settings.maxInboxFiles)
    || settings.maxInboxFiles <= 0
    || settings.maxInboxFiles > 4_294_967_295) {
    errors.maxInboxFiles = text.fileLimitValidation;
  }
  if (settings.maxUploadBytes !== null
    && settings.maxUploadBytes > settings.maxInboxBytes) {
    errors.maxUploadBytes = text.uploadExceedsInbox;
    errors.maxInboxBytes = text.inboxBelowUpload;
  }
  if (settings.downloadShare.enabled && !settings.downloadShare.path.trim()) {
    errors.downloadShare = text.downloadFolderRequired;
  }
  if (settings.uploadShare.enabled && !settings.uploadShare.path.trim()) {
    errors.uploadShare = text.uploadFolderRequired;
  }
  if (settings.downloadShare.enabled
    && settings.uploadShare.enabled
    && settings.downloadShare.path
    && settings.downloadShare.path.localeCompare(
      settings.uploadShare.path,
      undefined,
      { sensitivity: "accent" },
    ) === 0) {
    errors.shareOverlap = text.sameFolderWarning;
  }
  return errors;
}

export function hasErrors(errors: DraftValidationErrors | ShareValidation): boolean {
  return Object.values(errors).some(Boolean);
}

export function shareSignature(settings: AppSettings): string {
  return JSON.stringify([
    settings.downloadShare.enabled,
    settings.downloadShare.path,
    settings.uploadShare.enabled,
    settings.uploadShare.path,
  ]);
}
