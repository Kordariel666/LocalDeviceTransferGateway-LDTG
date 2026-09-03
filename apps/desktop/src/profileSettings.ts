import type {
  AppSettings,
  LimitSettings,
  NetworkSettings,
  ShareProfile,
} from "@dmdc/shared";

export type EffectiveProfileSettings = LimitSettings & NetworkSettings & {
  port: number;
  downloadShare: ShareProfile["downloadShare"];
  uploadShare: ShareProfile["uploadShare"];
};

export function activeProfile(settings: AppSettings): ShareProfile {
  return settings.profiles.find((profile) => profile.id === settings.activeProfileId)
    ?? settings.profiles[0];
}

export function effectiveProfileSettings(settings: AppSettings): EffectiveProfileSettings {
  const profile = activeProfile(settings);
  const limits = profile.overrides.limits;
  return {
    downloadShare: profile.downloadShare,
    uploadShare: profile.uploadShare,
    preferredAdapterId: profile.overrides.network
      ? profile.overrides.network.preferredAdapterId
      : settings.preferredAdapterId,
    port: profile.overrides.port ?? settings.port,
    maxUploadBytes: limits ? limits.maxUploadBytes : settings.maxUploadBytes,
    maxInboxBytes: limits ? limits.maxInboxBytes : settings.maxInboxBytes,
    maxInboxFiles: limits ? limits.maxInboxFiles : settings.maxInboxFiles,
    idleTimeoutMinutes: limits ? limits.idleTimeoutMinutes : settings.idleTimeoutMinutes,
  };
}

export function updateActiveProfile(
  settings: AppSettings,
  update: (profile: ShareProfile) => ShareProfile,
): AppSettings {
  return {
    ...settings,
    profiles: settings.profiles.map((profile) => (
      profile.id === settings.activeProfileId ? update(profile) : profile
    )),
  };
}

export function uniqueProfileName(settings: AppSettings, requested: string): string {
  const names = new Set(settings.profiles.map((profile) => profile.name.trim().toLocaleLowerCase("de")));
  const base = requested.trim().slice(0, 64) || "Profil";
  if (!names.has(base.toLocaleLowerCase("de"))) return base;
  for (let index = 2; index <= 99; index += 1) {
    const suffix = ` ${index}`;
    const candidate = `${base.slice(0, 64 - suffix.length)}${suffix}`;
    if (!names.has(candidate.toLocaleLowerCase("de"))) return candidate;
  }
  return `Profil ${settings.profiles.length + 1}`;
}

export function duplicateActiveProfile(settings: AppSettings): AppSettings {
  const source = activeProfile(settings);
  const copy: ShareProfile = {
    ...source,
    id: crypto.randomUUID(),
    name: uniqueProfileName(settings, `${source.name} Kopie`),
    downloadShare: { ...source.downloadShare },
    uploadShare: { ...source.uploadShare },
    overrides: {
      network: source.overrides.network ? { ...source.overrides.network } : null,
      port: source.overrides.port,
      limits: source.overrides.limits ? { ...source.overrides.limits } : null,
    },
  };
  return {
    ...settings,
    profiles: [...settings.profiles, copy],
    activeProfileId: copy.id,
  };
}

export function deleteActiveProfile(settings: AppSettings): AppSettings {
  if (settings.profiles.length <= 1) return settings;
  const index = settings.profiles.findIndex((profile) => profile.id === settings.activeProfileId);
  const profiles = settings.profiles.filter((profile) => profile.id !== settings.activeProfileId);
  return {
    ...settings,
    profiles,
    activeProfileId: profiles[Math.min(Math.max(index, 0), profiles.length - 1)].id,
  };
}

export function setProfileOverride(
  settings: AppSettings,
  kind: "network" | "port" | "limits",
  enabled: boolean,
): AppSettings {
  const effective = effectiveProfileSettings(settings);
  return updateActiveProfile(settings, (profile) => ({
    ...profile,
    overrides: {
      ...profile.overrides,
      [kind]: enabled
        ? kind === "network"
          ? { preferredAdapterId: effective.preferredAdapterId }
          : kind === "port"
            ? effective.port
            : {
                maxUploadBytes: effective.maxUploadBytes,
                maxInboxBytes: effective.maxInboxBytes,
                maxInboxFiles: effective.maxInboxFiles,
                idleTimeoutMinutes: effective.idleTimeoutMinutes,
              }
        : null,
    },
  }));
}
