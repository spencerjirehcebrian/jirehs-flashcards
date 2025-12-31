import { useStudySession } from '../../hooks/useStudySession';
import { Card } from '../Card/Card';
import { RatingButtons } from '../Rating/RatingButtons';
import { StudyComplete } from './StudyComplete';
import { StudyProgress } from './StudyProgress';
import { TypedAnswerInput } from './TypedAnswerInput';
import { AnswerComparison } from './AnswerComparison';

interface StudySessionProps {
  deckPath?: string;
}

export function StudySession({ deckPath }: StudySessionProps) {
  const {
    currentCard,
    currentIndex,
    total,
    revealed,
    isComplete,
    isLoading,
    isSubmitting,
    isComparing,
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
  } = useStudySession(deckPath);

  if (isLoading) {
    return <div className="loading">Loading cards...</div>;
  }

  if (isComplete) {
    return <StudyComplete onRestart={restart} />;
  }

  if (!currentCard) {
    return (
      <div className="no-cards">
        <h2>No cards to study</h2>
        <p>Import some flashcards to get started.</p>
      </div>
    );
  }

  return (
    <div className="study-session">
      <div className="study-header">
        <StudyProgress current={currentIndex} total={total} />
        <button
          type="button"
          className="button-secondary mode-toggle"
          onClick={toggleAnswerMode}
        >
          {answerMode === 'flip' ? 'Switch to Typed' : 'Switch to Flip'}
        </button>
      </div>

      {answerMode === 'flip' ? (
        <>
          <Card card={currentCard} revealed={revealed} onReveal={reveal} />
          {revealed && (
            <RatingButtons
              onRate={rate}
              disabled={isSubmitting}
              ratingScale={ratingScale}
            />
          )}
        </>
      ) : (
        <>
          {/* Typed mode: show question only */}
          <div className="card">
            <div className="card-content">
              <div className="card-question">
                <div className="card-label">Question</div>
                <div className="card-text">{currentCard.question}</div>
              </div>

              {!revealed ? (
                <TypedAnswerInput
                  value={typedAnswer}
                  onChange={setTypedAnswer}
                  onSubmit={submitTypedAnswer}
                  disabled={isComparing}
                />
              ) : (
                <>
                  {compareResult && (
                    <AnswerComparison
                      result={compareResult}
                      correctAnswer={currentCard.answer}
                    />
                  )}
                </>
              )}
            </div>
          </div>

          {revealed && (
            <RatingButtons
              onRate={rate}
              disabled={isSubmitting}
              ratingScale={ratingScale}
            />
          )}
        </>
      )}
    </div>
  );
}
