import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ask, save } from "@tauri-apps/plugin-dialog";
import type {
  AppSettings,
  AppSnapshot,
  FirewallStatus,
  LimitSettings,
  ServiceStatus,
  SessionChangedEvent,
  ShareProfile,
  ShareValidation,
  TransferChangedEvent,
} from "@ldtg/shared";
import { text } from "./i18n";
import {
  DiagnosticsPage,
  OverviewPage,
  SecurityPage,
  SharesPage,
  TransfersPage,
} from "./pages/DesktopPages";
import { hasErrors, settingsEqual, shareSignature, validateDraft } from "./settingsDraft";
import {
  type BatchSettlement,
  createTransferNotificationState,
  notifyDesktop,
  trackTransferNotification,
} from "./notifications";
import { applySessionsChanged, applyTransferChanged } from "./statusEvents";
import { commandError, errorMessage, invoke } from "./tauriClient";
import { useLifecycle } from "./useLifecycle";
import {
  activeProfile,
  deleteActiveProfile,
  duplicateActiveProfile,
  effectiveProfileSettings,
  setProfileOverride,
  updateActiveProfile,
} from "./profileSettings";

type View = "overview" | "shares" | "transfers" | "security" | "diagnostics";
type BusyAction = "start" | "stop" | "save" | "firewall" | "diagnostics" | "command" | null;

const brandIconUrl = new URL("../../../assets/ldtg-ui-icon.png", import.meta.url).href;
const batchQuietPeriodMs = 750;

const emptyService: ServiceStatus = {
  state: "stopped",
  serviceId: null,
  url: null,
  accessCode: null,
  startedAt: null,
  activeTransfers: 0,
  sessions: [],
  transfers: [],
  error: null,
};

const emptyShareValidation: ShareValidation = {
  downloadError: null,
  uploadError: null,
  overlapError: null,
};

const navigation: { id: View; label: string }[] = [
  { id: "overview", label: text.overview },
  { id: "shares", label: text.shares },
  { id: "transfers", label: text.transfers },
  { id: "security", label: text.networkSecurity },
  { id: "diagnostics", label: text.diagnostics },
];

function serviceHostname(url: string | null): string | null {
  if (!url) return null;
  try {
    return new URL(url).hostname;
  } catch {
    return null;
  }
}

function mergeBatchSettlement(
  current: BatchSettlement | null,
  next: BatchSettlement,
): BatchSettlement {
  if (current === "failed" || next === "failed") return "failed";
  if (current === "cancelled" || next === "cancelled") return "cancelled";
  return "complete";
}

export function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [draft, setDraft] = useState<AppSettings | null>(null);
  const [view, setView] = useState<View>("overview");
  const [busyAction, setBusyAction] = useState<BusyAction>(null);
  const [notice, setNotice] = useState<{ kind: "info" | "error"; text: string } | null>(null);
  const [stopAfterBatch, setStopAfterBatch] = useState(false);
  const [shareCheck, setShareCheck] = useState<{
    signature: string;
    result: ShareValidation;
    pending: boolean;
  }>({ signature: "", result: emptyShareValidation, pending: false });
  const snapshotRequest = useRef(0);
  const serviceRequest = useRef(0);
  const shareValidationRequest = useRef(0);
  const snapshotRetryAttempt = useRef(0);
  const snapshotAvailable = useRef(false);
  const transferNotifications = useRef(createTransferNotificationState());
  const pendingBatchSettlement = useRef<BatchSettlement | null>(null);
  const batchNotificationTimer = useRef<number | undefined>(undefined);
  const stopAfterBatchService = useRef<string | null>(null);
  const autoStopTimer = useRef<number | undefined>(undefined);
  const autoStopInFlight = useRef(false);
  const previousTransferActivity = useRef({ serviceId: null as string | null, active: 0 });

  const refreshSnapshot = useCallback(async (synchronizeDraft = false) => {
    const request = ++snapshotRequest.current;
    try {
      const next = await invoke<AppSnapshot>("get_app_snapshot");
      if (request !== snapshotRequest.current) return;
      snapshotAvailable.current = true;
      setSnapshot(next);
      snapshotRetryAttempt.current = 0;
      setDraft((current) => (synchronizeDraft || current === null ? next.settings : current));
      setNotice((current) => current?.kind === "error" ? null : current);
    } catch (error) {
      if (request !== snapshotRequest.current) return;
      setNotice({ kind: "error", text: errorMessage(error) });
    }
  }, []);

  const refreshService = useCallback(async () => {
    if (!snapshotAvailable.current) {
      await refreshSnapshot(true);
      return;
    }
    const request = ++serviceRequest.current;
    try {
      const service = await invoke<ServiceStatus>("get_service_status");
      if (request !== serviceRequest.current) return;
      setSnapshot((current) => (current ? { ...current, service } : current));
    } catch (error) {
      if (request !== serviceRequest.current) return;
      setNotice({ kind: "error", text: errorMessage(error) });
    }
  }, [refreshSnapshot]);

  const handleSessionChanged = useCallback((event: SessionChangedEvent) => {
    setSnapshot((current) => current
      ? { ...current, service: applySessionsChanged(current.service, event) }
      : current);
  }, []);

  const handleTransferChanged = useCallback((event: TransferChangedEvent) => {
    if (
      transferNotifications.current.serviceId !== null
      && transferNotifications.current.serviceId !== event.serviceId
    ) {
      if (batchNotificationTimer.current !== undefined) {
        window.clearTimeout(batchNotificationTimer.current);
        batchNotificationTimer.current = undefined;
      }
      pendingBatchSettlement.current = null;
    }
    const tracked = trackTransferNotification(transferNotifications.current, event);
    transferNotifications.current = tracked.state;
    if (event.transfer.state === "active" && batchNotificationTimer.current !== undefined) {
      window.clearTimeout(batchNotificationTimer.current);
      batchNotificationTimer.current = undefined;
    }
    if (tracked.outcome) {
      pendingBatchSettlement.current = mergeBatchSettlement(
        pendingBatchSettlement.current,
        tracked.outcome,
      );
      if (batchNotificationTimer.current !== undefined) {
        window.clearTimeout(batchNotificationTimer.current);
      }
      batchNotificationTimer.current = window.setTimeout(() => {
        const outcome = pendingBatchSettlement.current;
        pendingBatchSettlement.current = null;
        batchNotificationTimer.current = undefined;
        transferNotifications.current = createTransferNotificationState();
        if (outcome === "complete" || outcome === "failed") void notifyDesktop(outcome);
      }, batchQuietPeriodMs);
    }
    setSnapshot((current) => current
      ? { ...current, service: applyTransferChanged(current.service, event) }
      : current);
  }, []);

  const handleNetworkChanged = useCallback((available: boolean) => {
    if (!available) void notifyDesktop("network-lost");
  }, []);

  useEffect(() => { void refreshSnapshot(); }, [refreshSnapshot]);

  useEffect(() => () => {
    if (batchNotificationTimer.current !== undefined) {
      window.clearTimeout(batchNotificationTimer.current);
    }
    if (autoStopTimer.current !== undefined) window.clearTimeout(autoStopTimer.current);
  }, []);

  useEffect(() => {
    if (snapshot || notice?.kind !== "error" || snapshotRetryAttempt.current >= 3) return;
    const delay = 1000 * 2 ** snapshotRetryAttempt.current;
    snapshotRetryAttempt.current += 1;
    const timer = window.setTimeout(() => void refreshSnapshot(true), delay);
    return () => window.clearTimeout(timer);
  }, [notice, refreshSnapshot, snapshot]);

  const service = snapshot?.service ?? emptyService;
  const running = service.state === "running";
  const activeTransfers = useMemo(() => service.transfers.filter((transfer) => transfer.state === "active"), [service.transfers]);
  const busy = busyAction !== null;
  const dirty = useMemo(
    () => snapshot !== null && draft !== null && !settingsEqual(snapshot.settings, draft),
    [draft, snapshot],
  );
  const recoveryPending = snapshot?.configurationWarning !== null && snapshot?.configurationWarning !== undefined;
  const saveAvailable = dirty || recoveryPending;
  const draftErrors = useMemo(() => draft ? validateDraft(draft) : {}, [draft]);
  const effectiveDraft = useMemo(() => draft ? effectiveProfileSettings(draft) : null, [draft]);
  const currentShareSignature = useMemo(() => draft ? shareSignature(draft) : "", [draft]);
  const activeShareValidation = shareCheck.signature === currentShareSignature
    ? shareCheck.result
    : emptyShareValidation;
  const sharesNeedBackendValidation = Boolean(draft && (
    (effectiveDraft?.downloadShare.enabled && effectiveDraft.downloadShare.path.trim())
    || (effectiveDraft?.uploadShare.enabled && effectiveDraft.uploadShare.path.trim())
  ));
  const shareValidationPending = sharesNeedBackendValidation
    && (shareCheck.signature !== currentShareSignature || shareCheck.pending);
  const persistBlocked = hasErrors(draftErrors)
    || hasErrors(activeShareValidation)
    || shareValidationPending;

  const allowUnload = useLifecycle({
    dirty,
    running,
    snapshotAvailable: snapshotAvailable.current,
    refreshService,
    stop,
    quit,
    onError: (message) => setNotice({ kind: "error", text: message }),
    onSessionChanged: handleSessionChanged,
    onTransferChanged: handleTransferChanged,
    onNetworkChanged: handleNetworkChanged,
  });

  useEffect(() => {
    const serviceId = service.serviceId;
    const active = activeTransfers.length;
    const previous = previousTransferActivity.current;

    if (!running || stopAfterBatchService.current !== serviceId) {
      if (autoStopTimer.current !== undefined) {
        window.clearTimeout(autoStopTimer.current);
        autoStopTimer.current = undefined;
      }
      if (stopAfterBatch) setStopAfterBatch(false);
      stopAfterBatchService.current = null;
    } else if (active > 0 && autoStopTimer.current !== undefined) {
      window.clearTimeout(autoStopTimer.current);
      autoStopTimer.current = undefined;
    } else if (
      stopAfterBatch
      && previous.serviceId === serviceId
      && previous.active > 0
      && active === 0
      && autoStopTimer.current === undefined
      && !autoStopInFlight.current
    ) {
      autoStopTimer.current = window.setTimeout(() => {
        autoStopTimer.current = undefined;
        autoStopInFlight.current = true;
        stopAfterBatchService.current = null;
        setStopAfterBatch(false);
        setBusyAction("stop");
        setNotice(null);
        void invoke("stop_service", { force: false })
          .then(refreshService)
          .then(() => setNotice({ kind: "info", text: text.stoppedAfterBatch }))
          .catch((error) => setNotice({ kind: "error", text: errorMessage(error) }))
          .finally(() => {
            autoStopInFlight.current = false;
            setBusyAction(null);
          });
      }, batchQuietPeriodMs);
    }

    previousTransferActivity.current = { serviceId, active };
  }, [activeTransfers.length, refreshService, running, service.serviceId, stopAfterBatch]);

  useEffect(() => {
    if (!draft || running || !sharesNeedBackendValidation
      || draftErrors.downloadShare || draftErrors.uploadShare || draftErrors.shareOverlap) {
      shareValidationRequest.current += 1;
      setShareCheck({
        signature: currentShareSignature,
        result: emptyShareValidation,
        pending: false,
      });
      return;
    }
    const request = ++shareValidationRequest.current;
    setShareCheck({
      signature: currentShareSignature,
      result: emptyShareValidation,
      pending: true,
    });
    const timer = window.setTimeout(() => {
      void invoke<ShareValidation>("validate_share_settings", { settings: draft })
        .then((result) => {
          if (request !== shareValidationRequest.current) return;
          setShareCheck({
            signature: currentShareSignature,
            result: result ?? emptyShareValidation,
            pending: false,
          });
        })
        .catch((error) => {
          if (request !== shareValidationRequest.current) return;
          setShareCheck({
            signature: currentShareSignature,
            result: { ...emptyShareValidation, overlapError: errorMessage(error) },
            pending: false,
          });
        });
    }, 250);
    return () => window.clearTimeout(timer);
  }, [
    currentShareSignature,
    draft,
    draftErrors.downloadShare,
    draftErrors.shareOverlap,
    draftErrors.uploadShare,
    running,
    sharesNeedBackendValidation,
  ]);

  const selectedNetwork = useMemo(() => {
    if (!snapshot || !draft || !effectiveDraft) return undefined;
    const activeAddress = serviceHostname(service.url);
    return snapshot.networks.find((item) => activeAddress === item.address)
      ?? snapshot.networks.find((item) => item.id === effectiveDraft.preferredAdapterId)
      ?? snapshot.networks.find((item) => item.preferred)
      ?? snapshot.networks[0];
  }, [snapshot, draft, effectiveDraft, service.url]);

  const transferHistory = useMemo(() => service.transfers.filter((transfer) => transfer.state !== "active").slice().reverse(), [service.transfers]);

  async function validateSharesNow(settings: AppSettings): Promise<ShareValidation> {
    const signature = shareSignature(settings);
    const request = ++shareValidationRequest.current;
    setShareCheck({ signature, result: emptyShareValidation, pending: true });
    const result = await invoke<ShareValidation>("validate_share_settings", { settings })
      ?? emptyShareValidation;
    if (request === shareValidationRequest.current) {
      setShareCheck({ signature, result, pending: false });
    }
    return result;
  }

  async function saveSettings(next = draft, refreshAfterSave = true): Promise<AppSettings | null> {
    if (!next) return null;
    if (running) {
      setNotice({ kind: "info", text: text.settingsLocked });
      return null;
    }
    const localErrors = validateDraft(next);
    if (hasErrors(localErrors)) {
      setView(localErrors.profileName || localErrors.downloadShare || localErrors.uploadShare || localErrors.shareOverlap ? "shares" : "security");
      setNotice({ kind: "error", text: text.correctInvalidFields });
      return null;
    }
    const shareValidation = await validateSharesNow(next);
    if (hasErrors(shareValidation)) {
      setView("shares");
      setNotice({ kind: "error", text: text.correctInvalidShares });
      return null;
    }
    const saved = await invoke<AppSettings>("save_settings", { settings: next });
    setSnapshot((current) => current ? { ...current, settings: saved, configurationWarning: null } : current);
    setDraft((current) => settingsEqual(current, next) ? saved : current);
    if (refreshAfterSave) await refreshSnapshot();
    return saved;
  }

  function updateProfile(patch: Partial<ShareProfile>) {
    setDraft((current) => current
      ? updateActiveProfile(current, (profile) => ({ ...profile, ...patch }))
      : current);
  }

  function updateProfileOverride(
    kind: "network" | "port" | "limits",
    enabled: boolean,
  ) {
    setDraft((current) => current ? setProfileOverride(current, kind, enabled) : current);
  }

  function updateEffectiveNetwork(preferredAdapterId: string | null) {
    setDraft((current) => {
      if (!current) return current;
      const profile = activeProfile(current);
      if (!profile.overrides.network) return { ...current, preferredAdapterId };
      return updateActiveProfile(current, (entry) => ({
        ...entry,
        overrides: {
          ...entry.overrides,
          network: { preferredAdapterId },
        },
      }));
    });
  }

  function updateEffectivePort(port: number) {
    setDraft((current) => {
      if (!current) return current;
      const profile = activeProfile(current);
      if (profile.overrides.port === null) return { ...current, port };
      return updateActiveProfile(current, (entry) => ({
        ...entry,
        overrides: { ...entry.overrides, port },
      }));
    });
  }

  function updateEffectiveLimits(patch: Partial<LimitSettings>) {
    setDraft((current) => {
      if (!current) return current;
      const profile = activeProfile(current);
      if (!profile.overrides.limits) return { ...current, ...patch };
      return updateActiveProfile(current, (entry) => ({
        ...entry,
        overrides: {
          ...entry.overrides,
          limits: { ...entry.overrides.limits!, ...patch },
        },
      }));
    });
  }

  function selectProfile(profileId: string) {
    if (draftErrors.profileName) {
      setNotice({ kind: "error", text: text.correctProfileNameBeforeSwitch });
      return;
    }
    setDraft((current) => current ? { ...current, activeProfileId: profileId } : current);
  }

  function duplicateProfile() {
    setDraft((current) => current ? duplicateActiveProfile(current) : current);
  }

  async function deleteProfile() {
    if (!draft || draft.profiles.length <= 1) return;
    const profile = activeProfile(draft);
    const accepted = await ask(text.deleteProfileWarning(profile.name), {
      title: text.deleteProfile,
      kind: "warning",
      okLabel: text.deleteProfile,
      cancelLabel: text.cancel,
    });
    if (accepted) setDraft((current) => current ? deleteActiveProfile(current) : current);
  }

  function updateStopAfterBatch(enabled: boolean) {
    if (!enabled) {
      if (autoStopTimer.current !== undefined) {
        window.clearTimeout(autoStopTimer.current);
        autoStopTimer.current = undefined;
      }
      stopAfterBatchService.current = null;
      setStopAfterBatch(false);
      return;
    }
    if (!running || !service.serviceId || activeTransfers.length === 0) return;
    stopAfterBatchService.current = service.serviceId;
    previousTransferActivity.current = {
      serviceId: service.serviceId,
      active: activeTransfers.length,
    };
    setStopAfterBatch(true);
  }

  async function saveCurrentSettings() {
    setBusyAction("save");
    setNotice(null);
    try {
      const saved = await saveSettings();
      if (saved) setNotice({ kind: "info", text: text.settingsSaved });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function start() {
    if (!draft || !effectiveDraft) return;
    if (!effectiveDraft.downloadShare.enabled && !effectiveDraft.uploadShare.enabled) {
      setView("shares");
      setNotice({ kind: "error", text: text.oneShareRequired });
      return;
    }
    setBusyAction("start");
    updateStopAfterBatch(false);
    setNotice(null);
    try {
      const saved = await saveSettings(draft, false);
      if (!saved) return;
      const prepared = await invoke<AppSnapshot>("get_app_snapshot");
      setSnapshot(prepared);
      setDraft(prepared.settings);
      if (!prepared.firewall.configured) {
        const continueWithoutRule = await ask(text.firewallWarning, {
          title: text.firewallMissing,
          kind: "warning",
          okLabel: text.startAnyway,
          cancelLabel: text.cancel,
        });
        if (!continueWithoutRule) return;
      }
      let networkApproval: string | null = null;
      let broadShareApproval: string | null = null;
      for (;;) {
        try {
          await invoke("start_service", { networkApproval, broadShareApproval });
          break;
        } catch (error) {
          const failure = commandError(error);
          if (failure?.code === "NETWORK_UNTRUSTED" && failure.context?.kind === "networkApproval") {
            const approved = await ask(text.trustNetwork(failure.context.networkName), {
              title: text.networkConfirm,
              kind: "warning",
              okLabel: text.trustAndStart,
              cancelLabel: text.cancel,
            });
            if (!approved) return;
            networkApproval = failure.context.token;
            continue;
          }
          if (failure?.code === "BROAD_SHARE" && failure.context?.kind === "broadShareApproval") {
            const approved = await ask(text.broadShareWarning(failure.context.path), {
              title: text.broadShare,
              kind: "warning",
              okLabel: text.confirmBroadShare,
              cancelLabel: text.cancel,
            });
            if (!approved) return;
            broadShareApproval = failure.context.token;
            continue;
          }
          throw error;
        }
      }
      await refreshSnapshot(true);
      setView("overview");
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function stop(force = false) {
    setBusyAction("stop");
    setNotice(null);
    try {
      await invoke("stop_service", { force });
      updateStopAfterBatch(false);
      await refreshService();
    } catch (error) {
      const failure = commandError(error);
      if (failure?.code === "ACTIVE_TRANSFERS" && failure.context?.kind === "activeTransfers") {
        const accepted = await ask(text.activeStopWarning(failure.context.count), {
          title: text.transferRunning,
          kind: "warning",
          okLabel: text.stopAnyway,
          cancelLabel: text.keepRunning,
        });
        if (accepted) await stop(true);
      } else setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function quit(force = false, discardUnsaved = false) {
    if (discardUnsaved) allowUnload.current = true;
    try {
      await invoke("quit_app", { force, discardUnsaved });
    } catch (error) {
      const failure = commandError(error);
      if (failure?.code === "UNSAVED_CHANGES") {
        const accepted = await ask(text.unsavedQuitWarning, {
          title: text.unsavedChanges,
          kind: "warning",
          okLabel: text.discardAndQuit,
          cancelLabel: text.keepEditing,
        });
        if (accepted) await quit(force, true);
        else allowUnload.current = false;
      } else if (failure?.code === "ACTIVE_TRANSFERS" && failure.context?.kind === "activeTransfers") {
        const accepted = await ask(text.activeQuitWarning(failure.context.count), {
          title: text.quit,
          kind: "warning",
          okLabel: text.quitNow,
          cancelLabel: text.keepRunning,
        });
        if (accepted) await quit(true, discardUnsaved);
        else allowUnload.current = false;
      } else {
        allowUnload.current = false;
        setNotice({ kind: "error", text: errorMessage(error) });
      }
    }
  }

  async function configureFirewall() {
    if (!draft || running) return;
    setBusyAction("firewall");
    setNotice(null);
    try {
      const saved = await saveSettings(draft, false);
      if (!saved) return;
      const firewall = await invoke<FirewallStatus>("configure_firewall");
      setSnapshot((current) => (current ? { ...current, firewall } : current));
      setNotice({ kind: "info", text: text.firewallConfigured });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function forgetTrustedNetwork(networkId: string | null) {
    if (running) {
      setNotice({ kind: "info", text: text.trustedNetworksLocked });
      return;
    }
    if (networkId === null) {
      const accepted = await ask(text.forgetAllNetworksWarning, {
        title: text.forgetAllNetworks,
        kind: "warning",
        okLabel: text.forgetAll,
        cancelLabel: text.cancel,
      });
      if (!accepted) return;
    }
    setBusyAction("command");
    setNotice(null);
    try {
      const saved = await invoke<AppSettings>("forget_trusted_network", { networkId });
      setSnapshot((current) => current ? { ...current, settings: saved } : current);
      setDraft((current) => current ? {
        ...current,
        version: saved.version,
        trustedNetworks: saved.trustedNetworks,
      } : saved);
      setNotice({
        kind: "info",
        text: networkId === null ? text.allNetworksForgotten : text.networkForgotten,
      });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function exportDiagnostics() {
    const destination = await save({
      title: text.diagnosticTitle,
      defaultPath: `LDTG-Diagnose-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: text.diagnosticFilter, extensions: ["json"] }],
    });
    if (!destination) return;
    setBusyAction("diagnostics");
    setNotice(null);
    try {
      await invoke("export_diagnostics", { destination });
      setNotice({ kind: "info", text: text.diagnosticSaved });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function clearTransferHistory() {
    setBusyAction("command");
    setNotice(null);
    try {
      const nextService = await invoke<ServiceStatus>("clear_transfer_history");
      setSnapshot((current) => current ? { ...current, service: nextService } : current);
      setNotice({ kind: "info", text: text.historyCleared });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function simpleCommand(command: string, args: Record<string, unknown> = {}) {
    setBusyAction("command");
    setNotice(null);
    try {
      await invoke(command, args);
      await refreshService();
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function copyAccessCode() {
    if (!service.accessCode) return;
    try {
      await navigator.clipboard.writeText(service.accessCode);
      setNotice({ kind: "info", text: text.codeCopied });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    }
  }

  if (!snapshot || !draft) {
    return (
      <main className="loading">
        <p>{notice?.kind === "error" ? notice.text : text.preparing}</p>
        {notice?.kind === "error" && <button className="button primary" type="button" onClick={() => { snapshotRetryAttempt.current = 0; void refreshSnapshot(true); }}>{text.retry}</button>}
      </main>
    );
  }

  const currentSnapshot: AppSnapshot = snapshot;
  const currentDraft: AppSettings = draft;
  const currentEffective = effectiveProfileSettings(currentDraft);
  const currentProfile = activeProfile(currentDraft);
  const downloadShareError = draftErrors.downloadShare ?? activeShareValidation.downloadError;
  const uploadShareError = draftErrors.uploadShare ?? activeShareValidation.uploadError;
  const shareOverlapError = draftErrors.shareOverlap ?? activeShareValidation.overlapError;
  const noEnabledShares = !currentEffective.downloadShare.enabled && !currentEffective.uploadShare.enabled;
  const enabledShares = Number(currentEffective.downloadShare.enabled) + Number(currentEffective.uploadShare.enabled);
  const statusLabel = running ? text.serviceOnline : service.state === "error" ? text.serviceError : text.serviceOffline;

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <img src={brandIconUrl} alt="" />
          <div><strong>LDTG</strong><span>{text.appSubtitle}</span></div>
        </div>
        <nav aria-label={text.mainNavigation}>
          {navigation.map((item) => (
            <button key={item.id} type="button" className={view === item.id ? "active" : ""} aria-current={view === item.id ? "page" : undefined} onClick={() => setView(item.id)}>
              <span>{item.label}</span>
              {item.id === "transfers" && service.activeTransfers > 0 && <b>{service.activeTransfers}</b>}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <p className="sidebar-note">{text.localOnly}</p>
          <p className="sidebar-version"><span>{text.version}</span><strong>{currentSnapshot.appVersion}</strong></p>
        </div>
      </aside>
      <div className="workspace">
        <header className="service-bar">
          <div className={`service-state ${running ? "running" : service.state === "error" ? "error" : "stopped"}`}>
            <i aria-hidden="true" /><strong>{statusLabel}</strong><span>{selectedNetwork?.name ?? text.noNetworkShort}</span><span>{selectedNetwork?.address ?? "—"}</span>
            {dirty && <span className="dirty-state" role="status">{text.unsavedChanges}</span>}
          </div>
          <div className="service-counters" aria-label={text.runtimeSummary}><span>{text.deviceCount(service.sessions.length)}</span><span>{text.transferCount(service.activeTransfers)}</span></div>
          {running ? (
            <button className="button danger service-action" type="button" disabled={busy} onClick={() => void stop(false)}>{busyAction === "stop" ? text.stopping : text.stop}</button>
          ) : (
            <button className="button primary service-action" type="button" disabled={busy} onClick={() => void start()}>{busyAction === "start" ? text.starting : text.start}</button>
          )}
        </header>
        <div className="notice-region">
          {notice && <div className={`notice ${notice.kind}`} role={notice.kind === "error" ? "alert" : "status"}>{notice.text}</div>}
          {!notice && currentSnapshot.configurationWarning && <div className="notice error" role="alert">{currentSnapshot.configurationWarning}</div>}
          {!notice && !currentSnapshot.configurationWarning && service.error && <div className="notice error" role="alert">{service.error}</div>}
        </div>
        <main className="page-content">
          {view === "overview" && (
            <OverviewPage
              service={service}
              running={running}
              busy={busy}
              activeTransfers={activeTransfers}
              enabledShares={enabledShares}
              profile={currentProfile}
              effectiveSettings={currentEffective}
              snapshot={currentSnapshot}
              selectedNetwork={selectedNetwork}
              onCommand={simpleCommand}
              onCopyAccessCode={copyAccessCode}
              onNavigate={setView}
            />
          )}
          {view === "shares" && (
            <SharesPage
              running={running}
              busy={busy}
              noEnabledShares={noEnabledShares}
              shareOverlapError={shareOverlapError}
              shareValidationPending={shareValidationPending}
              draft={currentDraft}
              profile={currentProfile}
              downloadShareError={downloadShareError}
              uploadShareError={uploadShareError}
              profileNameError={draftErrors.profileName}
              dirty={dirty}
              recoveryPending={recoveryPending}
              saveAvailable={saveAvailable}
              persistBlocked={persistBlocked}
              onUpdateProfile={updateProfile}
              onSelectProfile={selectProfile}
              onDuplicateProfile={duplicateProfile}
              onDeleteProfile={deleteProfile}
              onSave={saveCurrentSettings}
            />
          )}
          {view === "transfers" && (
            <TransfersPage
              running={running}
              busy={busy}
              activeTransfers={activeTransfers}
              transferHistory={transferHistory}
              stopAfterBatch={stopAfterBatch}
              onStopAfterBatchChange={updateStopAfterBatch}
              onClearHistory={clearTransferHistory}
            />
          )}
          {view === "security" && (
            <SecurityPage
              running={running}
              busy={busy}
              draft={currentDraft}
              profile={currentProfile}
              effectiveSettings={currentEffective}
              snapshot={currentSnapshot}
              selectedNetwork={selectedNetwork}
              draftErrors={draftErrors}
              persistBlocked={persistBlocked}
              dirty={dirty}
              recoveryPending={recoveryPending}
              saveAvailable={saveAvailable}
              onUpdateNetwork={updateEffectiveNetwork}
              onUpdatePort={updateEffectivePort}
              onUpdateLimits={updateEffectiveLimits}
              onSetOverride={updateProfileOverride}
              onConfigureFirewall={configureFirewall}
              onForgetTrustedNetwork={forgetTrustedNetwork}
              onSave={saveCurrentSettings}
            />
          )}
          {view === "diagnostics" && (
            <DiagnosticsPage
              statusLabel={statusLabel}
              selectedNetwork={selectedNetwork}
              snapshot={currentSnapshot}
              busy={busy}
              onExport={exportDiagnostics}
            />
          )}
        </main>
      </div>
    </div>
  );
}
