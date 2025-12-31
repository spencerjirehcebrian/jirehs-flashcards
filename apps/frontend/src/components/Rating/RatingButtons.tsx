import type { Rating, RatingScale } from '@jirehs-flashcards/shared-types';
import { TwoPointRatingButtons } from './TwoPointRatingButtons';

interface RatingButtonsProps {
  onRate: (rating: Rating) => void;
  disabled?: boolean;
  ratingScale?: RatingScale;
}

const fourPointRatings: { value: Rating; label: string; color: string }[] = [
  { value: 1, label: 'Again', color: '#ef4444' },
  { value: 2, label: 'Hard', color: '#f97316' },
  { value: 3, label: 'Good', color: '#22c55e' },
  { value: 4, label: 'Easy', color: '#3b82f6' },
];

export function RatingButtons({ onRate, disabled, ratingScale = '4point' }: RatingButtonsProps) {
  if (ratingScale === '2point') {
    return <TwoPointRatingButtons onRate={onRate} disabled={disabled} />;
  }

  return (
    <div className="rating-buttons">
      {fourPointRatings.map(({ value, label, color }) => (
        <button
          key={value}
          className="rating-button"
          style={{ '--rating-color': color } as React.CSSProperties}
          onClick={() => onRate(value)}
          disabled={disabled}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
