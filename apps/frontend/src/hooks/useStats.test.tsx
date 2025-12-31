import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useDeckStats, useStudyStats, useCalendarData } from './useStats';
import {
  createMockDeckStats,
  createMockStudyStats,
  createMockCalendarData,
} from '../test/factories';

// Mock the tauri module
vi.mock('../lib/tauri', () => ({
  tauri: {
    getDeckStats: vi.fn(),
    getStudyStats: vi.fn(),
    getCalendarData: vi.fn(),
  },
}));

import { tauri } from '../lib/tauri';

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });

  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };
}

describe('useDeckStats', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch deck stats for a specific deck', async () => {
    const mockStats = createMockDeckStats({ total_cards: 50 });
    vi.mocked(tauri.getDeckStats).mockResolvedValue(mockStats);

    const { result } = renderHook(() => useDeckStats('/decks/test'), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual(mockStats);
    expect(tauri.getDeckStats).toHaveBeenCalledWith('/decks/test');
  });

  it('should fetch global stats when no deck path provided', async () => {
    const mockStats = createMockDeckStats({ total_cards: 100 });
    vi.mocked(tauri.getDeckStats).mockResolvedValue(mockStats);

    const { result } = renderHook(() => useDeckStats(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual(mockStats);
    expect(tauri.getDeckStats).toHaveBeenCalledWith(undefined);
  });

  it('should handle errors', async () => {
    const error = new Error('Failed to fetch stats');
    vi.mocked(tauri.getDeckStats).mockRejectedValue(error);

    const { result } = renderHook(() => useDeckStats('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toEqual(error);
  });
});

describe('useStudyStats', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch study stats', async () => {
    const mockStats = createMockStudyStats({ streak_days: 10 });
    vi.mocked(tauri.getStudyStats).mockResolvedValue(mockStats);

    const { result } = renderHook(() => useStudyStats(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual(mockStats);
    expect(tauri.getStudyStats).toHaveBeenCalled();
  });

  it('should handle errors', async () => {
    const error = new Error('Failed to fetch study stats');
    vi.mocked(tauri.getStudyStats).mockRejectedValue(error);

    const { result } = renderHook(() => useStudyStats(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toEqual(error);
  });
});

describe('useCalendarData', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch calendar data with default 90 days', async () => {
    const mockData = createMockCalendarData(90);
    vi.mocked(tauri.getCalendarData).mockResolvedValue(mockData);

    const { result } = renderHook(() => useCalendarData(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual(mockData);
    expect(tauri.getCalendarData).toHaveBeenCalledWith(90);
  });

  it('should fetch calendar data with custom days', async () => {
    const mockData = createMockCalendarData(30);
    vi.mocked(tauri.getCalendarData).mockResolvedValue(mockData);

    const { result } = renderHook(() => useCalendarData(30), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(tauri.getCalendarData).toHaveBeenCalledWith(30);
  });

  it('should handle errors', async () => {
    const error = new Error('Failed to fetch calendar data');
    vi.mocked(tauri.getCalendarData).mockRejectedValue(error);

    const { result } = renderHook(() => useCalendarData(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toEqual(error);
  });
});
