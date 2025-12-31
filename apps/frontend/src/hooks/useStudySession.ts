import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { Rating, ReviewRequest, AnswerMode, RatingScale } from '@jirehs-flashcards/shared-types';
import { tauri } from '../lib/tauri';
import { useStudyStore } from '../stores/studyStore';
import { useEffectiveSettings } from './useSettings';

export function useStudySession(deckPath?: string) {
  const queryClient = useQueryClient();
  const {
    currentIndex,
    revealed,
    answerMode,
    typedAnswer,
    compareResult,
    setRevealed,
    startTimer,
    getElapsedMs,
    setTypedAnswer,
    setCompareResult,
    setAnswerMode,
    nextCard,
    reset,
  } = useStudyStore();

  // Get effective settings for rating scale
  const { data: settings } = useEffectiveSettings(deckPath);
  const ratingScale: RatingScale = settings?.rating_scale ?? '4point';

  const queue = useQuery({
    queryKey: ['study-queue', deckPath],
    queryFn: () => tauri.getStudyQueue(deckPath),
  });

  const submitReview = useMutation({
    mutationFn: (request: ReviewRequest) => tauri.submitReview(request),
    onSuccess: () => {
      nextCard();
      queryClient.invalidateQueries({ queryKey: ['study-queue'] });
    },
  });

  const compareAnswer = useMutation({
    mutationFn: ({ typed, correct }: { typed: string; correct: string }) =>
      tauri.compareTypedAnswer(typed, correct, deckPath),
    onSuccess: (result) => {
      setCompareResult(result);
      setRevealed(true);
    },
  });

  const allCards = [...(queue.data?.new_cards ?? []), ...(queue.data?.review_cards ?? [])];
  const currentCard = allCards[currentIndex];
  const isComplete = currentIndex >= allCards.length && queue.isSuccess;
  const total = allCards.length;
  const progress = total > 0 ? currentIndex / total : 0;

  const reveal = () => {
    startTimer();
    setRevealed(true);
  };

  const submitTypedAnswer = () => {
    if (!currentCard || !typedAnswer.trim()) return;
    startTimer();
    compareAnswer.mutate({
      typed: typedAnswer,
      correct: currentCard.answer,
    });
  };

  const rate = (rating: Rating) => {
    if (!currentCard) return;

    submitReview.mutate({
      card_id: currentCard.id,
      rating,
      rating_scale: ratingScale,
      answer_mode: answerMode,
      typed_answer: answerMode === 'typed' ? typedAnswer : undefined,
      time_taken_ms: getElapsedMs(),
    });
  };

  const restart = () => {
    reset();
    queryClient.invalidateQueries({ queryKey: ['study-queue'] });
  };

  const toggleAnswerMode = () => {
    setAnswerMode(answerMode === 'flip' ? 'typed' : 'flip');
  };

  return {
    queue,
    currentCard,
    currentIndex,
    total,
    progress,
    revealed,
    isComplete,
    isLoading: queue.isLoading,
    isSubmitting: submitReview.isPending,
    isComparing: compareAnswer.isPending,
    answerMode,
    typedAnswer,
    compareResult,
    ratingScale,
    reveal,
    rate,
    restart,
    setTypedAnswer,
    submitTypedAnswer,
    toggleAnswerMode,
    setAnswerMode,
  };
}
