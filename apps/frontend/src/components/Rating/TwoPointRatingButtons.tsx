import type { Rating } from '@jirehs-flashcards/shared-types';

interface TwoPointRatingButtonsProps {
  onRate: (rating: Rating) => void;
  disabled?: boolean;
}

export function TwoPointRatingButtons({ onRate, disabled = false }: TwoPointRatingButtonsProps) {
  // 2-point scale: Wrong (1) -> Again, Correct (2) -> Good (3)
  return (
    <div className="rating-buttons two-point">
      <button
        type="button"
        className="rating-button rating-wrong"
        style={{ '--rating-color': 'var(--danger)' } as React.CSSProperties}
        onClick={() => onRate(1)}
        disabled={disabled}
      >
        Wrong
      </button>
      <button
        type="button"
        className="rating-button rating-correct"
        style={{ '--rating-color': 'var(--success)' } as React.CSSProperties}
        onClick={() => onRate(3)}
        disabled={disabled}
      >
        Correct
      </button>
    </div>
  );
}
