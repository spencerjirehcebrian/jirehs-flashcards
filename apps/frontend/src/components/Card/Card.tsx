import type { Card as CardType } from '@jirehs-flashcards/shared-types';

interface CardProps {
  card: CardType;
  revealed: boolean;
  onReveal: () => void;
}

export function Card({ card, revealed, onReveal }: CardProps) {
  return (
    <div className="card">
      <div className="card-content">
        <div className="card-question">
          <div className="card-label">Question</div>
          <div className="card-text">{card.question}</div>
        </div>

        {revealed ? (
          <div className="card-answer">
            <div className="card-label">Answer</div>
            <div className="card-text">{card.answer}</div>
          </div>
        ) : (
          <button className="reveal-button" onClick={onReveal}>
            Show Answer
          </button>
        )}
      </div>
    </div>
  );
}
