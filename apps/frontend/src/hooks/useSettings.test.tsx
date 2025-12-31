import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useSettings, useDeckSettings, useEffectiveSettings } from './useSettings';
import {
  createMockGlobalSettings,
  createMockDeckSettings,
  createMockEffectiveSettings,
} from '../test/factories';

// Mock the tauri module
vi.mock('../lib/tauri', () => ({
  tauri: {
    getGlobalSettings: vi.fn(),
    saveGlobalSettings: vi.fn(),
    getDeckSettings: vi.fn(),
    saveDeckSettings: vi.fn(),
    deleteDeckSettings: vi.fn(),
    getEffectiveSettings: vi.fn(),
  },
}));

import { tauri } from '../lib/tauri';

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  });
}

function createWrapper(queryClient?: QueryClient) {
  const client = queryClient ?? createTestQueryClient();

  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  };
}

describe('useSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch global settings on mount', async () => {
    const mockSettings = createMockGlobalSettings({ new_cards_per_day: 30 });
    vi.mocked(tauri.getGlobalSettings).mockResolvedValue(mockSettings);

    const { result } = renderHook(() => useSettings(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.settings).toEqual(mockSettings);
    expect(tauri.getGlobalSettings).toHaveBeenCalled();
  });

  it('should return loading state while fetching', () => {
    vi.mocked(tauri.getGlobalSettings).mockImplementation(
      () => new Promise(() => {}) // Never resolves
    );

    const { result } = renderHook(() => useSettings(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);
    expect(result.current.settings).toBeUndefined();
  });

  it('should handle save errors', async () => {
    const mockSettings = createMockGlobalSettings();
    vi.mocked(tauri.getGlobalSettings).mockResolvedValue(mockSettings);
    vi.mocked(tauri.saveGlobalSettings).mockRejectedValue(new Error('Save failed'));

    const { result } = renderHook(() => useSettings(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    await act(async () => {
      result.current.save(mockSettings);
    });

    await waitFor(() => {
      expect(result.current.saveError).toBeDefined();
    });

    expect(result.current.saveError?.message).toBe('Save failed');
  });

  it('should return isSaving state', async () => {
    const mockSettings = createMockGlobalSettings();
    vi.mocked(tauri.getGlobalSettings).mockResolvedValue(mockSettings);

    const { result } = renderHook(() => useSettings(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.isSaving).toBe(false);
  });
});

describe('useDeckSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch deck settings when deckPath provided', async () => {
    const mockSettings = createMockDeckSettings({ algorithm: 'fsrs' });
    vi.mocked(tauri.getDeckSettings).mockResolvedValue(mockSettings);

    const { result } = renderHook(() => useDeckSettings('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.settings).toEqual(mockSettings);
    expect(tauri.getDeckSettings).toHaveBeenCalledWith('/decks/test');
  });

  it('should not fetch when deckPath is empty', () => {
    const { result } = renderHook(() => useDeckSettings(''), {
      wrapper: createWrapper(),
    });

    expect(tauri.getDeckSettings).not.toHaveBeenCalled();
    expect(result.current.isLoading).toBe(false);
  });

  it('should delete deck settings', async () => {
    const mockSettings = createMockDeckSettings();
    vi.mocked(tauri.getDeckSettings).mockResolvedValue(mockSettings);
    vi.mocked(tauri.deleteDeckSettings).mockResolvedValue(undefined);

    const { result } = renderHook(() => useDeckSettings('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    await act(async () => {
      result.current.delete();
    });

    await waitFor(() => {
      expect(result.current.isDeleting).toBe(false);
    });

    expect(tauri.deleteDeckSettings).toHaveBeenCalledWith('/decks/test');
  });

  it('should return save and delete functions', async () => {
    vi.mocked(tauri.getDeckSettings).mockResolvedValue(null);

    const { result } = renderHook(() => useDeckSettings('/decks/test'), {
      wrapper: createWrapper(),
    });

    expect(typeof result.current.save).toBe('function');
    expect(typeof result.current.delete).toBe('function');
    expect(typeof result.current.saveAsync).toBe('function');
    expect(typeof result.current.deleteAsync).toBe('function');
  });
});

describe('useEffectiveSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch effective settings without deck path', async () => {
    const mockSettings = createMockEffectiveSettings();
    vi.mocked(tauri.getEffectiveSettings).mockResolvedValue(mockSettings);

    const { result } = renderHook(() => useEffectiveSettings(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual(mockSettings);
    expect(tauri.getEffectiveSettings).toHaveBeenCalledWith(undefined);
  });

  it('should fetch effective settings with deck path', async () => {
    const mockSettings = createMockEffectiveSettings({ algorithm: 'fsrs' });
    vi.mocked(tauri.getEffectiveSettings).mockResolvedValue(mockSettings);

    const { result } = renderHook(() => useEffectiveSettings('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual(mockSettings);
    expect(tauri.getEffectiveSettings).toHaveBeenCalledWith('/decks/test');
  });
});
