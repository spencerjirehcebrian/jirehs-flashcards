import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useStudySession } from './useStudySession';
import { useStudyStore } from '../stores/studyStore';
import {
  createMockCard,
  createMockStudyQueue,
  createMockEffectiveSettings,
  createMockCompareAnswerResponse,
} from '../test/factories';

// Mock the tauri module
vi.mock('../lib/tauri', () => ({
  tauri: {
    getStudyQueue: vi.fn(),
    submitReview: vi.fn(),
    compareTypedAnswer: vi.fn(),
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

describe('useStudySession', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Reset Zustand store
    useStudyStore.setState({
      currentIndex: 0,
      revealed: false,
      startTime: null,
      answerMode: 'flip',
      typedAnswer: '',
      compareResult: null,
    });

    // Default mock implementations
    vi.mocked(tauri.getEffectiveSettings).mockResolvedValue(
      createMockEffectiveSettings()
    );
  });

  it('should fetch study queue for deck', async () => {
    const mockQueue = createMockStudyQueue({
      new_cards: [createMockCard()],
      review_cards: [createMockCard()],
    });
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(mockQueue);

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(tauri.getStudyQueue).toHaveBeenCalledWith('/decks/test');
  });

  it('should return loading state initially', () => {
    vi.mocked(tauri.getStudyQueue).mockImplementation(
      () => new Promise(() => {})
    );

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);
    expect(result.current.currentCard).toBeUndefined();
  });

  it('should combine new_cards and review_cards into allCards', async () => {
    const newCard = createMockCard({ id: 1 });
    const reviewCard = createMockCard({ id: 2 });
    const mockQueue = createMockStudyQueue({
      new_cards: [newCard],
      review_cards: [reviewCard],
    });
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(mockQueue);

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.total).toBe(2);
    expect(result.current.currentCard).toEqual(newCard);
  });

  it('should return currentCard at current index', async () => {
    const card1 = createMockCard({ id: 1, question: 'Q1' });
    const card2 = createMockCard({ id: 2, question: 'Q2' });
    const mockQueue = createMockStudyQueue({
      new_cards: [card1, card2],
    });
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(mockQueue);

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.currentCard?.question).toBe('Q1');

    // Advance to next card
    act(() => {
      useStudyStore.getState().setCurrentIndex(1);
    });

    expect(result.current.currentCard?.question).toBe('Q2');
  });

  it('should calculate progress correctly', async () => {
    const mockQueue = createMockStudyQueue({
      new_cards: [createMockCard(), createMockCard(), createMockCard(), createMockCard()],
    });
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(mockQueue);

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.progress).toBe(0);
    expect(result.current.total).toBe(4);

    act(() => {
      useStudyStore.getState().setCurrentIndex(2);
    });

    expect(result.current.progress).toBe(0.5);
  });

  it('should return isComplete true when all cards reviewed', async () => {
    const mockQueue = createMockStudyQueue({
      new_cards: [createMockCard()],
    });
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(mockQueue);

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.isComplete).toBe(false);

    act(() => {
      useStudyStore.getState().setCurrentIndex(1);
    });

    expect(result.current.isComplete).toBe(true);
  });

  it('should toggle answer mode', async () => {
    const mockQueue = createMockStudyQueue({
      new_cards: [createMockCard()],
    });
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(mockQueue);

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.answerMode).toBe('flip');

    act(() => {
      result.current.toggleAnswerMode();
    });

    expect(result.current.answerMode).toBe('typed');

    act(() => {
      result.current.toggleAnswerMode();
    });

    expect(result.current.answerMode).toBe('flip');
  });

  it('should reveal card and set revealed state', async () => {
    const mockQueue = createMockStudyQueue({
      new_cards: [createMockCard()],
    });
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(mockQueue);

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.revealed).toBe(false);

    act(() => {
      result.current.reveal();
    });

    expect(result.current.revealed).toBe(true);
  });

  it('should set typed answer', async () => {
    const mockQueue = createMockStudyQueue({
      new_cards: [createMockCard()],
    });
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(mockQueue);

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.setTypedAnswer('my answer');
    });

    expect(result.current.typedAnswer).toBe('my answer');
  });

  it('should return default rating scale when settings not loaded', async () => {
    vi.mocked(tauri.getEffectiveSettings).mockImplementation(
      () => new Promise(() => {})
    );
    vi.mocked(tauri.getStudyQueue).mockResolvedValue(createMockStudyQueue());

    const { result } = renderHook(() => useStudySession('/decks/test'), {
      wrapper: createWrapper(),
    });

    // Default should be 4point
    expect(result.current.ratingScale).toBe('4point');
  });
});
