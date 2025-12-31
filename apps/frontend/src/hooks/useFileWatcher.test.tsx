import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useFileWatcher } from './useFileWatcher';

// Mock the tauri module
vi.mock('../lib/tauri', () => ({
  tauri: {
    getWatchedDirectories: vi.fn(),
    startWatching: vi.fn(),
    stopWatching: vi.fn(),
  },
}));

// Mock the event API - simplified to avoid async issues
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
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

describe('useFileWatcher', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch watched directories', async () => {
    vi.mocked(tauri.getWatchedDirectories).mockResolvedValue([
      '/path/to/dir1',
      '/path/to/dir2',
    ]);

    const { result } = renderHook(() => useFileWatcher(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.watchedDirectories).toEqual([
        '/path/to/dir1',
        '/path/to/dir2',
      ]);
    });

    expect(tauri.getWatchedDirectories).toHaveBeenCalled();
  });

  it('should start watching a directory', async () => {
    vi.mocked(tauri.getWatchedDirectories).mockResolvedValue([]);
    vi.mocked(tauri.startWatching).mockResolvedValue(undefined);

    const { result } = renderHook(() => useFileWatcher(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isStartingWatch).toBe(false);
    });

    await act(async () => {
      result.current.startWatching('/new/path');
    });

    await waitFor(() => {
      expect(result.current.isStartingWatch).toBe(false);
    });

    expect(tauri.startWatching).toHaveBeenCalled();
    expect(vi.mocked(tauri.startWatching).mock.calls[0][0]).toBe('/new/path');
  });

  it('should stop watching a directory', async () => {
    vi.mocked(tauri.getWatchedDirectories).mockResolvedValue(['/path/to/dir']);
    vi.mocked(tauri.stopWatching).mockResolvedValue(undefined);

    const { result } = renderHook(() => useFileWatcher(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.watchedDirectories).toHaveLength(1);
    });

    await act(async () => {
      result.current.stopWatching('/path/to/dir');
    });

    await waitFor(() => {
      expect(result.current.isStoppingWatch).toBe(false);
    });

    expect(tauri.stopWatching).toHaveBeenCalled();
    expect(vi.mocked(tauri.stopWatching).mock.calls[0][0]).toBe('/path/to/dir');
  });

  it('should return empty directories initially before fetch', () => {
    vi.mocked(tauri.getWatchedDirectories).mockImplementation(
      () => new Promise(() => {})
    );

    const { result } = renderHook(() => useFileWatcher(), {
      wrapper: createWrapper(),
    });

    expect(result.current.watchedDirectories).toEqual([]);
  });

  it('should return isStartingWatch and isStoppingWatch states', async () => {
    vi.mocked(tauri.getWatchedDirectories).mockResolvedValue([]);
    vi.mocked(tauri.startWatching).mockImplementation(
      () => new Promise((resolve) => setTimeout(resolve, 100))
    );

    const { result } = renderHook(() => useFileWatcher(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.watchedDirectories).toEqual([]);
    });

    expect(result.current.isStartingWatch).toBe(false);
    expect(result.current.isStoppingWatch).toBe(false);
  });

  it('should return dismissToast function', async () => {
    vi.mocked(tauri.getWatchedDirectories).mockResolvedValue([]);

    const { result } = renderHook(() => useFileWatcher(), {
      wrapper: createWrapper(),
    });

    expect(typeof result.current.dismissToast).toBe('function');
  });

  it('should return toasts array', async () => {
    vi.mocked(tauri.getWatchedDirectories).mockResolvedValue([]);

    const { result } = renderHook(() => useFileWatcher(), {
      wrapper: createWrapper(),
    });

    expect(Array.isArray(result.current.toasts)).toBe(true);
  });
});
