import { useState, useEffect } from 'react';
import type { GlobalSettings, Algorithm, RatingScale, MatchingMode } from '@jirehs-flashcards/shared-types';
import { useSettings } from '../hooks/useSettings';
import { useFileWatcher } from '../hooks/useFileWatcher';
import { useSync, useDeviceRegistration, useLocalSyncState } from '../hooks/useSync';
import { WatchedDirectoriesSection } from '../components/Settings/WatchedDirectoriesSection';
import { ToastContainer } from '../components/Notifications/Toast';

export function Settings() {
  const { settings, isLoading, save, isSaving } = useSettings();
  const {
    watchedDirectories,
    startWatching,
    stopWatching,
    isStartingWatch,
    isStoppingWatch,
    toasts,
    dismissToast,
  } = useFileWatcher();
  const {
    status: syncStatus,
    startSync,
    confirmOrphans,
    skipOrphans,
    isSyncing,
    isAwaitingConfirmation,
    isPending: isSyncPending,
    backendUrl,
    setBackendUrl,
  } = useSync();
  const { deviceInfo, register, isRegistering, refetch: refetchDevice } = useDeviceRegistration();
  const { data: syncState } = useLocalSyncState();
  const [formData, setFormData] = useState<GlobalSettings | null>(null);
  const [saved, setSaved] = useState(false);

  // Initialize form data when settings load
  useEffect(() => {
    if (settings && !formData) {
      setFormData(settings);
    }
  }, [settings, formData]);

  // Reset saved indicator after 2 seconds
  useEffect(() => {
    if (saved) {
      const timer = setTimeout(() => setSaved(false), 2000);
      return () => clearTimeout(timer);
    }
  }, [saved]);

  if (isLoading || !formData) {
    return (
      <div className="settings-page">
        <h1>Settings</h1>
        <div className="loading">Loading settings...</div>
      </div>
    );
  }

  const handleChange = <K extends keyof GlobalSettings>(
    key: K,
    value: GlobalSettings[K]
  ) => {
    setFormData((prev) => (prev ? { ...prev, [key]: value } : prev));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (formData) {
      save(formData, {
        onSuccess: () => setSaved(true),
      });
    }
  };

  return (
    <div className="settings-page">
      <h1>Settings</h1>
      <p className="settings-description">
        Configure your study preferences. These settings apply globally and can be overridden per deck.
      </p>

      <form className="settings-form" onSubmit={handleSubmit}>
        {/* Algorithm Section */}
        <section className="settings-section">
          <h2>Spaced Repetition Algorithm</h2>
          <div className="form-group">
            <label className="form-label">Algorithm</label>
            <div className="radio-group">
              <label className="radio-option">
                <input
                  type="radio"
                  name="algorithm"
                  value="sm2"
                  checked={formData.algorithm === 'sm2'}
                  onChange={() => handleChange('algorithm', 'sm2' as Algorithm)}
                />
                <span>SM-2</span>
              </label>
              <label className="radio-option">
                <input
                  type="radio"
                  name="algorithm"
                  value="fsrs"
                  checked={formData.algorithm === 'fsrs'}
                  onChange={() => handleChange('algorithm', 'fsrs' as Algorithm)}
                />
                <span>FSRS</span>
              </label>
            </div>
            <span className="form-hint">
              SM-2 is the classic algorithm. FSRS is a modern algorithm based on memory research.
            </span>
          </div>
        </section>

        {/* Study Mode Section */}
        <section className="settings-section">
          <h2>Study Mode</h2>
          <div className="form-group">
            <label className="form-label">Rating Scale</label>
            <div className="radio-group">
              <label className="radio-option">
                <input
                  type="radio"
                  name="rating_scale"
                  value="4point"
                  checked={formData.rating_scale === '4point'}
                  onChange={() => handleChange('rating_scale', '4point' as RatingScale)}
                />
                <span>4-Point (Again, Hard, Good, Easy)</span>
              </label>
              <label className="radio-option">
                <input
                  type="radio"
                  name="rating_scale"
                  value="2point"
                  checked={formData.rating_scale === '2point'}
                  onChange={() => handleChange('rating_scale', '2point' as RatingScale)}
                />
                <span>2-Point (Wrong, Correct)</span>
              </label>
            </div>
          </div>
        </section>

        {/* Answer Matching Section */}
        <section className="settings-section">
          <h2>Answer Matching (Typed Mode)</h2>
          <div className="form-group">
            <label className="form-label">Matching Mode</label>
            <select
              className="form-select"
              value={formData.matching_mode}
              onChange={(e) => handleChange('matching_mode', e.target.value as MatchingMode)}
            >
              <option value="exact">Exact Match</option>
              <option value="case_insensitive">Case Insensitive</option>
              <option value="fuzzy">Fuzzy Match</option>
            </select>
            <span className="form-hint">
              How strictly typed answers are compared to the correct answer.
            </span>
          </div>

          {formData.matching_mode === 'fuzzy' && (
            <div className="form-group">
              <label className="form-label">Fuzzy Threshold</label>
              <input
                type="range"
                className="form-range"
                min="0.5"
                max="1"
                step="0.05"
                value={formData.fuzzy_threshold}
                onChange={(e) => handleChange('fuzzy_threshold', parseFloat(e.target.value))}
              />
              <div className="range-value">
                {Math.round(formData.fuzzy_threshold * 100)}% similarity required
              </div>
            </div>
          )}
        </section>

        {/* Daily Limits Section */}
        <section className="settings-section">
          <h2>Daily Limits</h2>
          <div className="form-group">
            <label className="form-label">New Cards Per Day</label>
            <input
              type="number"
              className="form-input"
              min="0"
              max="999"
              value={formData.new_cards_per_day}
              onChange={(e) => handleChange('new_cards_per_day', parseInt(e.target.value) || 0)}
            />
            <span className="form-hint">Maximum number of new cards to introduce each day.</span>
          </div>

          <div className="form-group">
            <label className="form-label">Reviews Per Day</label>
            <input
              type="number"
              className="form-input"
              min="0"
              max="9999"
              value={formData.reviews_per_day}
              onChange={(e) => handleChange('reviews_per_day', parseInt(e.target.value) || 0)}
            />
            <span className="form-hint">Maximum number of review cards per day.</span>
          </div>

          <div className="form-group">
            <label className="form-label">Daily Reset Hour</label>
            <select
              className="form-select"
              value={formData.daily_reset_hour}
              onChange={(e) => handleChange('daily_reset_hour', parseInt(e.target.value))}
            >
              {Array.from({ length: 24 }, (_, i) => (
                <option key={i} value={i}>
                  {i === 0 ? '12:00 AM (Midnight)' : i < 12 ? `${i}:00 AM` : i === 12 ? '12:00 PM (Noon)' : `${i - 12}:00 PM`}
                </option>
              ))}
            </select>
            <span className="form-hint">When daily card counts reset (local time).</span>
          </div>
        </section>

        {/* Form Actions */}
        <div className="form-actions">
          {saved && <span className="save-indicator">Settings saved!</span>}
          <button type="submit" disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save Settings'}
          </button>
        </div>
      </form>

      {/* Cloud Sync Section */}
      <section className="settings-section">
        <h2>Cloud Sync</h2>
        <p className="settings-description">
          Sync your flashcards and study progress with the cloud.
        </p>

        <div className="form-group">
          <label className="form-label">Backend URL</label>
          <input
            type="url"
            className="form-input"
            placeholder="https://your-backend.com"
            value={backendUrl}
            onChange={(e) => setBackendUrl(e.target.value)}
          />
          <span className="form-hint">The URL of your sync server.</span>
        </div>

        {/* Device Registration */}
        <div className="form-group">
          <label className="form-label">Device Status</label>
          {deviceInfo ? (
            <div className="device-status">
              <span className="status-badge status-connected">Registered</span>
              <span className="device-id">Device ID: {deviceInfo.device_id?.slice(0, 8)}...</span>
            </div>
          ) : (
            <div className="device-status">
              <span className="status-badge status-disconnected">Not Registered</span>
              <button
                type="button"
                className="btn-secondary"
                onClick={() => register({ backendUrl })}
                disabled={!backendUrl || isRegistering}
              >
                {isRegistering ? 'Registering...' : 'Register Device'}
              </button>
            </div>
          )}
        </div>

        {/* Sync Status */}
        <div className="form-group">
          <label className="form-label">Sync Status</label>
          <div className="sync-status">
            {syncStatus.type === 'Idle' && (
              <span className="status-text">Ready to sync</span>
            )}
            {syncStatus.type === 'Syncing' && (
              <div className="sync-progress">
                <span className="status-text">
                  Syncing: {syncStatus.stage?.name}
                  {syncStatus.stage?.current !== undefined && ` (${syncStatus.stage.current}/${syncStatus.stage.total})`}
                </span>
                <div className="progress-bar">
                  <div
                    className="progress-fill"
                    style={{ width: `${(syncStatus.progress || 0) * 100}%` }}
                  />
                </div>
              </div>
            )}
            {syncStatus.type === 'Completed' && (
              <span className="status-text status-success">
                Synced successfully ({syncStatus.stats?.files_uploaded || 0} files, {syncStatus.stats?.cards_created || 0} new cards)
              </span>
            )}
            {syncStatus.type === 'Failed' && (
              <span className="status-text status-error">
                Sync failed: {syncStatus.error}
              </span>
            )}
            {isAwaitingConfirmation && syncStatus.orphans && (
              <div className="orphan-confirmation">
                <p>Found {syncStatus.orphans.length} cards that are no longer in your files:</p>
                <ul className="orphan-list">
                  {syncStatus.orphans.slice(0, 5).map((orphan) => (
                    <li key={orphan.card_id}>{orphan.question_preview}</li>
                  ))}
                  {syncStatus.orphans.length > 5 && (
                    <li>...and {syncStatus.orphans.length - 5} more</li>
                  )}
                </ul>
                <div className="orphan-actions">
                  <button
                    type="button"
                    className="btn-danger"
                    onClick={() => confirmOrphans(syncStatus.orphans!.map(o => o.card_id))}
                  >
                    Delete All
                  </button>
                  <button
                    type="button"
                    className="btn-secondary"
                    onClick={() => skipOrphans()}
                  >
                    Keep All
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Last Sync Info */}
        {syncState?.last_sync_at && (
          <div className="form-group">
            <label className="form-label">Last Synced</label>
            <span className="last-sync-time">
              {new Date(syncState.last_sync_at).toLocaleString()}
            </span>
            {syncState.pending_changes > 0 && (
              <span className="pending-changes">
                ({syncState.pending_changes} pending changes)
              </span>
            )}
          </div>
        )}

        {/* Sync Button */}
        <div className="form-actions">
          <button
            type="button"
            className="btn-primary"
            onClick={() => startSync({ url: backendUrl, watchedDirs: watchedDirectories })}
            disabled={!backendUrl || !deviceInfo || isSyncing || isSyncPending}
          >
            {isSyncing ? 'Syncing...' : 'Sync Now'}
          </button>
        </div>
      </section>

      {/* Watched Directories */}
      <WatchedDirectoriesSection
        watchedDirectories={watchedDirectories}
        onAddDirectory={startWatching}
        onRemoveDirectory={stopWatching}
        isAddPending={isStartingWatch}
        isRemovePending={isStoppingWatch}
      />

      {/* Toast Notifications */}
      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
