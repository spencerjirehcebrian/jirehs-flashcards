import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type {
  DeckSettings,
  EffectiveSettings,
  GlobalSettings,
} from '@jirehs-flashcards/shared-types';
import { tauri } from '../lib/tauri';

/**
 * Hook for managing global settings.
 */
export function useSettings() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['global-settings'],
    queryFn: tauri.getGlobalSettings,
  });

  const mutation = useMutation({
    mutationFn: tauri.saveGlobalSettings,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['global-settings'] });
      queryClient.invalidateQueries({ queryKey: ['effective-settings'] });
    },
  });

  return {
    settings: query.data,
    isLoading: query.isLoading,
    error: query.error,
    save: mutation.mutate,
    saveAsync: mutation.mutateAsync,
    isSaving: mutation.isPending,
    saveError: mutation.error,
  };
}

/**
 * Hook for managing deck-specific settings.
 */
export function useDeckSettings(deckPath: string) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['deck-settings', deckPath],
    queryFn: () => tauri.getDeckSettings(deckPath),
    enabled: !!deckPath,
  });

  const saveMutation = useMutation({
    mutationFn: tauri.saveDeckSettings,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deck-settings', deckPath] });
      queryClient.invalidateQueries({ queryKey: ['effective-settings'] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => tauri.deleteDeckSettings(deckPath),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deck-settings', deckPath] });
      queryClient.invalidateQueries({ queryKey: ['effective-settings'] });
    },
  });

  return {
    settings: query.data,
    isLoading: query.isLoading,
    error: query.error,
    save: saveMutation.mutate,
    saveAsync: saveMutation.mutateAsync,
    isSaving: saveMutation.isPending,
    delete: deleteMutation.mutate,
    deleteAsync: deleteMutation.mutateAsync,
    isDeleting: deleteMutation.isPending,
  };
}

/**
 * Hook for getting effective settings (merged global + deck).
 */
export function useEffectiveSettings(deckPath?: string) {
  return useQuery({
    queryKey: ['effective-settings', deckPath],
    queryFn: () => tauri.getEffectiveSettings(deckPath),
  });
}
