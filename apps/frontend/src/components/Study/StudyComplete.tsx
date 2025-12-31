import { Link } from 'react-router-dom';

interface StudyCompleteProps {
  onRestart?: () => void;
}

export function StudyComplete({ onRestart }: StudyCompleteProps) {
  return (
    <div className="study-complete">
      <h2>Session Complete</h2>
      <p>You've reviewed all cards for this session.</p>
      <div className="study-complete-actions">
        {onRestart && (
          <button className="button" onClick={onRestart}>
            Study Again
          </button>
        )}
        <Link to="/" className="button button-secondary">
          Back to Decks
        </Link>
      </div>
    </div>
  );
}
