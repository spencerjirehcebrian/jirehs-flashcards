import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useStudyStore } from './studyStore';
import { createMockCompareAnswerResponse } from '../test/factories';

describe('useStudyStore', () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useStudyStore.setState({
      currentIndex: 0,
      revealed: false,
      startTime: null,
      answerMode: 'flip',
      typedAnswer: '',
      compareResult: null,
    });
  });

  describe('initial state', () => {
    it('should have correct initial values', () => {
      const state = useStudyStore.getState();

      expect(state.currentIndex).toBe(0);
      expect(state.revealed).toBe(false);
      expect(state.startTime).toBeNull();
      expect(state.answerMode).toBe('flip');
      expect(state.typedAnswer).toBe('');
      expect(state.compareResult).toBeNull();
    });
  });

  describe('setCurrentIndex', () => {
    it('should update the current index', () => {
      useStudyStore.getState().setCurrentIndex(5);

      expect(useStudyStore.getState().currentIndex).toBe(5);
    });

    it('should allow setting to zero', () => {
      useStudyStore.getState().setCurrentIndex(10);
      useStudyStore.getState().setCurrentIndex(0);

      expect(useStudyStore.getState().currentIndex).toBe(0);
    });
  });

  describe('setRevealed', () => {
    it('should set revealed to true', () => {
      useStudyStore.getState().setRevealed(true);

      expect(useStudyStore.getState().revealed).toBe(true);
    });

    it('should set revealed to false', () => {
      useStudyStore.getState().setRevealed(true);
      useStudyStore.getState().setRevealed(false);

      expect(useStudyStore.getState().revealed).toBe(false);
    });
  });

  describe('startTimer', () => {
    it('should set startTime to current timestamp', () => {
      const before = Date.now();
      useStudyStore.getState().startTimer();
      const after = Date.now();

      const startTime = useStudyStore.getState().startTime;
      expect(startTime).not.toBeNull();
      expect(startTime).toBeGreaterThanOrEqual(before);
      expect(startTime).toBeLessThanOrEqual(after);
    });
  });

  describe('getElapsedMs', () => {
    it('should return 0 when timer not started', () => {
      const elapsed = useStudyStore.getState().getElapsedMs();

      expect(elapsed).toBe(0);
    });

    it('should return elapsed time when timer is running', () => {
      vi.useFakeTimers();

      useStudyStore.getState().startTimer();
      vi.advanceTimersByTime(1000);

      const elapsed = useStudyStore.getState().getElapsedMs();
      expect(elapsed).toBeGreaterThanOrEqual(1000);

      vi.useRealTimers();
    });

    it('should return correct elapsed time after multiple advances', () => {
      vi.useFakeTimers();

      useStudyStore.getState().startTimer();
      vi.advanceTimersByTime(500);
      vi.advanceTimersByTime(500);

      const elapsed = useStudyStore.getState().getElapsedMs();
      expect(elapsed).toBeGreaterThanOrEqual(1000);

      vi.useRealTimers();
    });
  });

  describe('setAnswerMode', () => {
    it('should switch to typed mode', () => {
      useStudyStore.getState().setAnswerMode('typed');

      expect(useStudyStore.getState().answerMode).toBe('typed');
    });

    it('should switch back to flip mode', () => {
      useStudyStore.getState().setAnswerMode('typed');
      useStudyStore.getState().setAnswerMode('flip');

      expect(useStudyStore.getState().answerMode).toBe('flip');
    });
  });

  describe('setTypedAnswer', () => {
    it('should update the typed answer', () => {
      useStudyStore.getState().setTypedAnswer('my answer');

      expect(useStudyStore.getState().typedAnswer).toBe('my answer');
    });

    it('should allow empty string', () => {
      useStudyStore.getState().setTypedAnswer('something');
      useStudyStore.getState().setTypedAnswer('');

      expect(useStudyStore.getState().typedAnswer).toBe('');
    });
  });

  describe('setCompareResult', () => {
    it('should store comparison result', () => {
      const result = createMockCompareAnswerResponse({
        is_correct: true,
        similarity: 1.0,
      });

      useStudyStore.getState().setCompareResult(result);

      expect(useStudyStore.getState().compareResult).toEqual(result);
    });

    it('should allow setting to null', () => {
      const result = createMockCompareAnswerResponse();
      useStudyStore.getState().setCompareResult(result);
      useStudyStore.getState().setCompareResult(null);

      expect(useStudyStore.getState().compareResult).toBeNull();
    });
  });

  describe('reset', () => {
    it('should clear all state except answerMode', () => {
      // Set up non-default state
      useStudyStore.setState({
        currentIndex: 5,
        revealed: true,
        startTime: Date.now(),
        answerMode: 'typed',
        typedAnswer: 'some answer',
        compareResult: createMockCompareAnswerResponse(),
      });

      useStudyStore.getState().reset();

      const state = useStudyStore.getState();
      expect(state.currentIndex).toBe(0);
      expect(state.revealed).toBe(false);
      expect(state.startTime).toBeNull();
      expect(state.answerMode).toBe('typed'); // Should preserve answer mode
      expect(state.typedAnswer).toBe('');
      expect(state.compareResult).toBeNull();
    });

    it('should preserve flip mode when resetting', () => {
      useStudyStore.setState({
        currentIndex: 3,
        answerMode: 'flip',
      });

      useStudyStore.getState().reset();

      expect(useStudyStore.getState().answerMode).toBe('flip');
    });
  });

  describe('nextCard', () => {
    it('should increment the current index', () => {
      useStudyStore.getState().nextCard();

      expect(useStudyStore.getState().currentIndex).toBe(1);
    });

    it('should reset card-specific state', () => {
      // Set up state as if we just reviewed a card
      useStudyStore.setState({
        currentIndex: 2,
        revealed: true,
        startTime: Date.now(),
        typedAnswer: 'my answer',
        compareResult: createMockCompareAnswerResponse(),
      });

      useStudyStore.getState().nextCard();

      const state = useStudyStore.getState();
      expect(state.currentIndex).toBe(3);
      expect(state.revealed).toBe(false);
      expect(state.startTime).toBeNull();
      expect(state.typedAnswer).toBe('');
      expect(state.compareResult).toBeNull();
    });

    it('should preserve answer mode', () => {
      useStudyStore.setState({ answerMode: 'typed' });

      useStudyStore.getState().nextCard();

      expect(useStudyStore.getState().answerMode).toBe('typed');
    });

    it('should increment correctly multiple times', () => {
      useStudyStore.getState().nextCard();
      useStudyStore.getState().nextCard();
      useStudyStore.getState().nextCard();

      expect(useStudyStore.getState().currentIndex).toBe(3);
    });
  });
});
