import type { AppSettings, ShareValidation } from "@dmdc/shared";
import { text } from "./i18n";
import { activeProfile, effectiveProfileSettings } from "./profileSettings";

export type DraftValidationErrors = {
  port?: string;
  maxUploadBytes?: string;
  maxInboxBytes?: string;
  maxInboxFiles?: string;
  downloadShare?: string;
  uploadShare?: string;
  shareOverlap?: string;
  profileName?: string;
};

function profileNameCharacterIsUnsafe(character: string): boolean {
  const code = character.codePointAt(0) ?? 0;
  return code <= 0x1f
    || (code >= 0x7f && code <= 0x9f)
    || code === 0x061c
    || code === 0x200e
    || code === 0x200f
    || (code >= 0x202a && code <= 0x202e)
    || (code >= 0x2066 && code <= 0x2069);
}

export function settingsEqual(left: AppSettings | null, right: AppSettings | null): boolean {
  if (!left || !right) return left === right;
  return JSON.stringify(left) === JSON.stringify(right);
}

export function validateDraft(settings: AppSettings): DraftValidationErrors {
  const errors: DraftValidationErrors = {};
  const profile = activeProfile(settings);
  const effective = effectiveProfileSettings(settings);
  const profileName = profile.name.trim();
  const duplicateName = settings.profiles.some((candidate) => (
    candidate.id !== profile.id
    && candidate.name.trim().localeCompare(profileName, "de", { sensitivity: "base" }) === 0
  ));
  if (!profileName
    || [...profileName].length > 64
    || [...profileName].some(profileNameCharacterIsUnsafe)
    || duplicateName) {
    errors.profileName = duplicateName ? text.profileNameDuplicate : text.profileNameValidation;
  }
  if (!Number.isSafeInteger(effective.port) || effective.port < 1024 || effective.port > 65535) {
    errors.port = text.portValidation;
  }
  if (effective.maxUploadBytes !== null
    && (!Number.isSafeInteger(effective.maxUploadBytes) || effective.maxUploadBytes <= 0)) {
    errors.maxUploadBytes = text.positiveUploadLimit;
  }
  if (!Number.isSafeInteger(effective.maxInboxBytes) || effective.maxInboxBytes <= 0) {
    errors.maxInboxBytes = text.positiveInboxLimit;
  }
  if (!Number.isSafeInteger(effective.maxInboxFiles)
    || effective.maxInboxFiles <= 0
    || effective.maxInboxFiles > 4_294_967_295) {
    errors.maxInboxFiles = text.fileLimitValidation;
  }
  if (effective.maxUploadBytes !== null
    && effective.maxUploadBytes > effective.maxInboxBytes) {
    errors.maxUploadBytes = text.uploadExceedsInbox;
    errors.maxInboxBytes = text.inboxBelowUpload;
  }
  if (effective.downloadShare.enabled && !effective.downloadShare.path.trim()) {
    errors.downloadShare = text.downloadFolderRequired;
  }
  if (effective.uploadShare.enabled && !effective.uploadShare.path.trim()) {
    errors.uploadShare = text.uploadFolderRequired;
  }
  if (effective.downloadShare.enabled
    && effective.uploadShare.enabled
    && effective.downloadShare.path
    && effective.downloadShare.path.localeCompare(
      effective.uploadShare.path,
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
  const effective = effectiveProfileSettings(settings);
  return JSON.stringify([
    settings.activeProfileId,
    effective.downloadShare.enabled,
    effective.downloadShare.path,
    effective.uploadShare.enabled,
    effective.uploadShare.path,
  ]);
}
