import { describe, expect, it } from "vitest";
import type { AppSettings } from "@ldtg/shared";
import {
  deleteActiveProfile,
  duplicateActiveProfile,
  effectiveProfileSettings,
  setProfileOverride,
  updateActiveProfile,
} from "./profileSettings";

function settings(): AppSettings {
  return {
    version: 4,
    profiles: [{
      id: "00000000-0000-4000-8000-000000000001",
      name: "Standard",
      downloadShare: { enabled: true, path: "C:\\Download" },
      uploadShare: { enabled: false, path: "" },
      overrides: { network: null, port: null, limits: null },
    }],
    activeProfileId: "00000000-0000-4000-8000-000000000001",
    preferredAdapterId: "global-lan",
    port: 8765,
    maxUploadBytes: 20,
    maxInboxBytes: 100,
    maxInboxFiles: 10_000,
    idleTimeoutMinutes: null,
    trustedNetworks: [],
  };
}

describe("profile settings", () => {
  it("resolves inherited and explicit profile values", () => {
    let value = settings();
    expect(effectiveProfileSettings(value).port).toBe(8765);

    value = updateActiveProfile(value, (profile) => ({
      ...profile,
      overrides: {
        network: { preferredAdapterId: null },
        port: 9123,
        limits: {
          maxUploadBytes: null,
          maxInboxBytes: 250,
          maxInboxFiles: 5_000,
          idleTimeoutMinutes: 30,
        },
      },
    }));

    expect(effectiveProfileSettings(value)).toMatchObject({
      preferredAdapterId: null,
      port: 9123,
      maxUploadBytes: null,
      maxInboxBytes: 250,
      maxInboxFiles: 5_000,
      idleTimeoutMinutes: 30,
    });
    expect(value.port).toBe(8765);
    expect(value.preferredAdapterId).toBe("global-lan");
  });

  it("copies the complete active profile under a fresh identity", () => {
    const original = settings();
    const duplicated = duplicateActiveProfile(original);

    expect(duplicated.profiles).toHaveLength(2);
    expect(duplicated.activeProfileId).not.toBe(original.activeProfileId);
    expect(duplicated.profiles[1].name).toBe("Standard Kopie");
    expect(duplicated.profiles[1].downloadShare).toEqual(original.profiles[0].downloadShare);
    expect(duplicated.profiles[1].downloadShare).not.toBe(original.profiles[0].downloadShare);
  });

  it("selects a neighboring profile after deletion and preserves the last profile", () => {
    const duplicated = duplicateActiveProfile(settings());
    const deleted = deleteActiveProfile(duplicated);

    expect(deleted.profiles).toHaveLength(1);
    expect(deleted.activeProfileId).toBe(settings().activeProfileId);
    expect(deleteActiveProfile(deleted)).toBe(deleted);
  });

  it("seeds an override from the currently effective inherited value", () => {
    const overridden = setProfileOverride(settings(), "limits", true);
    expect(overridden.profiles[0].overrides.limits).toEqual({
      maxUploadBytes: 20,
      maxInboxBytes: 100,
      maxInboxFiles: 10_000,
      idleTimeoutMinutes: null,
    });
    expect(setProfileOverride(overridden, "limits", false).profiles[0].overrides.limits).toBeNull();
  });
});
