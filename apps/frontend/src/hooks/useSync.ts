import { useState, useEffect, useCallback } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { tauri, SyncStatus, SyncStats, OrphanInfo, DeviceInfo, LocalSyncState } from '../lib/tauri';

export function useSync() {
  const queryClient = useQueryClient();
  const [status, setStatus] = useState<SyncStatus>({ type: 'Idle' });
  const [backendUrl, setBackendUrl] = useState<string>('');

  // Poll sync status when syncing
  useEffect(() => {
    if (status.type === 'Syncing') {
      const interval = setInterval(async () => {
        try {
          const newStatus = await tauri.getSyncStatus();
          setStatus(newStatus);

          // Invalidate queries when sync completes
          if (newStatus.type === 'Completed') {
            queryClient.invalidateQueries({ queryKey: ['decks'] });
            queryClient.invalidateQueries({ queryKey: ['study-queue'] });
            queryClient.invalidateQueries({ queryKey: ['deck-stats'] });
            queryClient.invalidateQueries({ queryKey: ['study-stats'] });
          }
        } catch (error) {
          console.error('Failed to get sync status:', error);
        }
      }, 1000);

      return () => clearInterval(interval);
    }
  }, [status.type, queryClient]);

  const startSync = useMutation({
    mutationFn: async ({ url, watchedDirs }: { url: string; watchedDirs: string[] }) => {
      setBackendUrl(url);
      return tauri.startSync(url, watchedDirs);
    },
    onSuccess: (newStatus) => {
      setStatus(newStatus);
    },
    onError: (error) => {
      setStatus({ type: 'Failed', error: String(error) });
    },
  });

  const confirmOrphans = useMutation({
    mutationFn: (cardIds: number[]) => tauri.confirmOrphanDeletion(cardIds),
    onSuccess: async () => {
      // Continue polling for status
      const newStatus = await tauri.getSyncStatus();
      setStatus(newStatus);
    },
  });

  const skipOrphans = useMutation({
    mutationFn: () => tauri.skipOrphanDeletion(),
    onSuccess: (stats) => {
      setStatus({
        type: 'Completed',
        stats,
        synced_at: new Date().toISOString(),
      });
    },
  });

  const cancelSync = useMutation({
    mutationFn: () => tauri.cancelSync(),
    onSuccess: () => {
      setStatus({ type: 'Idle' });
    },
  });

  const refreshStatus = useCallback(async () => {
    try {
      const newStatus = await tauri.getSyncStatus();
      setStatus(newStatus);
    } catch (error) {
      console.error('Failed to refresh sync status:', error);
    }
  }, []);

  return {
    status,
    startSync: startSync.mutate,
    confirmOrphans: confirmOrphans.mutate,
    skipOrphans: skipOrphans.mutate,
    cancelSync: cancelSync.mutate,
    refreshStatus,
    isSyncing: status.type === 'Syncing',
    isAwaitingConfirmation: status.type === 'AwaitingOrphanConfirmation',
    isPending: startSync.isPending,
    backendUrl,
    setBackendUrl,
  };
}

export function useDeviceRegistration() {
  const registerDevice = useMutation({
    mutationFn: ({ backendUrl, deviceName }: { backendUrl: string; deviceName?: string }) =>
      tauri.registerDevice(backendUrl, deviceName),
  });

  const deviceStatus = useQuery({
    queryKey: ['device-status'],
    queryFn: () => tauri.getDeviceStatus(),
    staleTime: Infinity,
  });

  return {
    register: registerDevice.mutate,
    isRegistering: registerDevice.isPending,
    deviceInfo: deviceStatus.data,
    isLoading: deviceStatus.isLoading,
    refetch: deviceStatus.refetch,
  };
}

export function useConnectivity(backendUrl: string) {
  return useQuery({
    queryKey: ['connectivity', backendUrl],
    queryFn: () => tauri.checkConnectivity(backendUrl),
    enabled: !!backendUrl,
    refetchInterval: 30000, // Check every 30 seconds
    retry: false,
  });
}

export function useLocalSyncState() {
  return useQuery({
    queryKey: ['local-sync-state'],
    queryFn: () => tauri.getLocalSyncState(),
  });
}
