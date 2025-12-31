import { useQuery } from '@tanstack/react-query';
import { tauri } from '../lib/tauri';

/**
 * Hook for fetching deck statistics.
 */
export function useDeckStats(deckPath?: string) {
  return useQuery({
    queryKey: ['deck-stats', deckPath],
    queryFn: () => tauri.getDeckStats(deckPath),
  });
}

/**
 * Hook for fetching overall study statistics.
 */
export function useStudyStats() {
  return useQuery({
    queryKey: ['study-stats'],
    queryFn: tauri.getStudyStats,
  });
}

/**
 * Hook for fetching calendar heatmap data.
 */
export function useCalendarData(days = 90) {
  return useQuery({
    queryKey: ['calendar-data', days],
    queryFn: () => tauri.getCalendarData(days),
  });
}
